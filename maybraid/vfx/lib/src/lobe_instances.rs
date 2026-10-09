//! Packed per-lobe GPU records indexed through [`MeshTag`].

use std::sync::atomic::{AtomicU32, Ordering};

use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use bevy::render::storage::ShaderBuffer;

use crate::composition::LobeKind;
use crate::lobe_material::LobeMaterial;
use crate::lobes::VfxLobe;
use crate::palette::ExplosionPalette;

static NEXT_LOBE_SLOT: AtomicU32 = AtomicU32::new(0);

/// Stable packed-buffer index for one lobe. Assigned once at spawn; never remapped.
pub fn next_lobe_slot() -> u32 {
	NEXT_LOBE_SLOT.fetch_add(1, Ordering::Relaxed)
}

/// One lobe's shader uniforms. Age lives in `params.x` and is updated each frame.
#[derive(Component, Clone, Copy, Debug, Default, ShaderType)]
pub struct LobeInstanceGpu {
	/// `x` age, `y` duration, `z` seed, `w` [`LobeKind`] as f32.
	pub params: Vec4,
	pub tint: Vec4,
	/// `x` emission/gain, `y` deform roll rate, `z` displace, `w` value-band count.
	pub extras: Vec4,
	pub color_hot: Vec4,
	pub color_mid: Vec4,
	pub color_cool: Vec4,
}

impl LobeInstanceGpu {
	pub fn new(
		kind: LobeKind,
		duration: f32,
		seed: f32,
		palette: &ExplosionPalette,
		tint: Color,
		intensity: f32,
	) -> Self {
		let tint = LinearRgba::from(tint);
		let (gain, roll, displace, bands) = match kind {
			LobeKind::Fire => (1.15 * intensity, 0.45, 0.24, 4.0),
			LobeKind::Smoke => (0.95 * intensity, 1.85, 0.30, 3.0),
			LobeKind::Flash => (2.4 * intensity, 0.0, 0.10, 2.0),
		};
		let (hot, mid, cool) = match kind {
			LobeKind::Fire => (palette.fire_hot, palette.fire_mid, palette.fire_cool),
			LobeKind::Smoke => (palette.smoke_lit, palette.smoke_lit, palette.smoke_shadow),
			LobeKind::Flash => (palette.flash, palette.fire_hot, palette.fire_mid),
		};
		Self {
			params: Vec4::new(0.0, duration.max(1e-3), seed, kind.as_f32()),
			tint: Vec4::new(tint.red, tint.green, tint.blue, 1.0),
			extras: Vec4::new(gain, roll, displace, bands),
			color_hot: ExplosionPalette::vec4(hot),
			color_mid: ExplosionPalette::vec4(mid),
			color_cool: ExplosionPalette::vec4(cool),
		}
	}

	pub fn set_age(&mut self, age: f32) {
		self.params.x = age.max(0.0);
	}

	pub fn alpha_mode_for_kind(&self) -> AlphaMode {
		if self.params.w > 1.5 {
			AlphaMode::Add
		} else {
			AlphaMode::Blend
		}
	}
}

/// Shared blend/add materials plus the instance buffer every lobe indexes.
#[derive(Resource, Clone)]
pub struct LobeInstancePack {
	pub buffer: Handle<ShaderBuffer>,
	pub blend: Handle<LobeMaterial>,
	pub add: Handle<LobeMaterial>,
}

impl LobeInstancePack {
	pub fn material_for(&self, kind: LobeKind) -> Handle<LobeMaterial> {
		match kind {
			LobeKind::Flash => self.add.clone(),
			LobeKind::Fire | LobeKind::Smoke => self.blend.clone(),
		}
	}
}

pub fn setup_lobe_instance_pack(
	mut commands: Commands,
	mut buffers: ResMut<Assets<ShaderBuffer>>,
	mut materials: ResMut<Assets<LobeMaterial>>,
) {
	let buffer = buffers.add(ShaderBuffer::from(vec![LobeInstanceGpu::default()]));
	let blend = materials.add(LobeMaterial::blend(buffer.clone()));
	let add = materials.add(LobeMaterial::additive(buffer.clone()));
	commands.insert_resource(LobeInstancePack { buffer, blend, add });
}

/// Write each lobe into its spawned [`MeshTag`] slot. Tags stay stable so the
/// render-world copy cannot point at a neighbor after query order changes.
pub fn sync_lobe_instance_buffer(
	pack: Option<Res<LobeInstancePack>>,
	mut buffers: ResMut<Assets<ShaderBuffer>>,
	mut lobes: Query<(&VfxLobe, &mut LobeInstanceGpu, &MeshTag)>,
) {
	let Some(pack) = pack else {
		return;
	};
	let Some(mut buffer) = buffers.get_mut(&pack.buffer) else {
		return;
	};
	let mut records = Vec::new();
	let mut max_idx = 0u32;
	for (lobe, mut gpu, tag) in &mut lobes {
		gpu.set_age(lobe.age);
		max_idx = max_idx.max(tag.0);
		records.push((tag.0, *gpu));
	}
	let mut instances = vec![LobeInstanceGpu::default(); max_idx as usize + 1];
	for (idx, gpu) in records {
		instances[idx as usize] = gpu;
	}
	buffer.set_data(instances);
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::lobes::{lobe_transform, vary_lobe};
	use crate::seed;

	#[test]
	fn flash_is_additive_fire_and_smoke_blend() {
		let palette = ExplosionPalette::maybraid();
		let fire = LobeInstanceGpu::new(LobeKind::Fire, 0.5, 0.0, &palette, Color::WHITE, 1.0);
		let smoke = LobeInstanceGpu::new(LobeKind::Smoke, 2.0, 1.0, &palette, Color::WHITE, 1.0);
		let flash = LobeInstanceGpu::new(LobeKind::Flash, 0.1, 2.0, &palette, Color::WHITE, 1.0);
		assert_eq!(fire.alpha_mode_for_kind(), AlphaMode::Blend);
		assert_eq!(smoke.alpha_mode_for_kind(), AlphaMode::Blend);
		assert_eq!(flash.alpha_mode_for_kind(), AlphaMode::Add);
		assert!(smoke.extras.z > fire.extras.z);
		assert!(smoke.extras.y > fire.extras.y);
	}

	#[test]
	fn packed_records_keep_sampled_variation() {
		let authored = crate::composition::LobeSpec::new(Vec3::ZERO, Vec3::new(0.7, 0.4, 0.5))
			.with_euler(Vec3::new(0.2, 0.4, -0.1))
			.with_expand(0.8)
			.with_rise(0.2)
			.with_roll(1.4)
			.with_duration(1.0);
		let spec = vary_lobe(authored, 11, seed::LAYER_FIRE, 0);
		let palette = ExplosionPalette::maybraid();
		let gpu = LobeInstanceGpu::new(
			LobeKind::Fire,
			spec.duration,
			seed::unit(seed::stream(11, seed::LAYER_FIRE, 0)),
			&palette,
			Color::WHITE,
			1.0,
		);
		let early = lobe_transform(&spec, 0.05);
		let late = lobe_transform(&spec, 0.8);
		assert!(early.rotation.angle_between(late.rotation) > 0.2);
		assert!((gpu.params.y - spec.duration).abs() < 1e-4);
	}

	#[test]
	fn sync_keeps_spawned_mesh_tags() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_plugins(AssetPlugin::default())
			.add_plugins(bevy::render::storage::StoragePlugin)
			.init_asset::<LobeMaterial>()
			.init_asset::<ShaderBuffer>()
			.add_systems(Startup, setup_lobe_instance_pack)
			.add_systems(Update, sync_lobe_instance_buffer);
		app.update();

		let palette = ExplosionPalette::maybraid();
		let first = MeshTag(next_lobe_slot());
		let second = MeshTag(next_lobe_slot());
		let first_idx = first.0;
		let second_idx = second.0;
		let spec = crate::composition::LobeSpec::new(Vec3::ZERO, Vec3::splat(0.5));
		app.world_mut().spawn((
			VfxLobe { age: 0.1, spec, playback: 1.0 },
			LobeInstanceGpu::new(LobeKind::Fire, 0.5, 0.0, &palette, Color::WHITE, 1.0),
			first,
		));
		app.world_mut().spawn((
			VfxLobe { age: 0.2, spec, playback: 1.0 },
			LobeInstanceGpu::new(LobeKind::Smoke, 2.0, 1.0, &palette, Color::WHITE, 1.0),
			second,
		));
		app.update();

		let tags: Vec<u32> =
			app.world_mut().query::<&MeshTag>().iter(app.world()).map(|tag| tag.0).collect();
		assert!(tags.contains(&first_idx), "{tags:?}");
		assert!(tags.contains(&second_idx), "{tags:?}");

		app.world_mut().spawn((
			VfxLobe { age: 0.0, spec, playback: 1.0 },
			LobeInstanceGpu::new(LobeKind::Flash, 0.1, 2.0, &palette, Color::WHITE, 1.0),
			MeshTag(next_lobe_slot()),
		));
		app.update();

		let tags: Vec<u32> =
			app.world_mut().query::<&MeshTag>().iter(app.world()).map(|tag| tag.0).collect();
		assert!(tags.contains(&first_idx), "first slot remapped: {tags:?}");
		assert!(tags.contains(&second_idx), "second slot remapped: {tags:?}");
	}

	#[test]
	fn one_explosion_uses_two_shared_materials() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_plugins(AssetPlugin::default())
			.add_plugins(bevy::render::storage::StoragePlugin)
			.init_asset::<LobeMaterial>()
			.init_asset::<ShaderBuffer>()
			.add_systems(Startup, setup_lobe_instance_pack);
		app.update();
		let pack = app.world().resource::<LobeInstancePack>();
		let materials = app.world().resource::<Assets<LobeMaterial>>();
		assert_eq!(materials.len(), 2);
		assert_ne!(pack.blend, pack.add);
		assert_eq!(
			materials.get(&pack.blend).map(|m| m.instances.id()),
			materials.get(&pack.add).map(|m| m.instances.id())
		);
	}
}
