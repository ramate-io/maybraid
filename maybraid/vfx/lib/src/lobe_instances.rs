//! Packed per-lobe GPU records indexed through [`MeshTag`].

use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use bevy::render::storage::ShaderBuffer;

use crate::composition::LobeKind;
use crate::lobe_material::LobeMaterial;
use crate::lobes::VfxLobe;
use crate::palette::ExplosionPalette;

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

/// Rebuild the packed instance buffer and stamp [`MeshTag`] indices for the shader.
pub fn sync_lobe_instance_buffer(
	pack: Option<Res<LobeInstancePack>>,
	mut buffers: ResMut<Assets<ShaderBuffer>>,
	mut lobes: Query<(&VfxLobe, &mut LobeInstanceGpu, &mut MeshTag)>,
) {
	let Some(pack) = pack else {
		return;
	};
	let Some(mut buffer) = buffers.get_mut(&pack.buffer) else {
		return;
	};
	let mut instances = Vec::new();
	for (lobe, mut gpu, mut tag) in &mut lobes {
		gpu.set_age(lobe.age);
		*tag = MeshTag(instances.len() as u32);
		instances.push(*gpu);
	}
	if instances.is_empty() {
		instances.push(LobeInstanceGpu::default());
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
