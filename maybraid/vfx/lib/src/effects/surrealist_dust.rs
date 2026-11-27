//! Seed13 non-contact Combined Wave, baked offline; no gameplay simulation.
mod grains;
use crate::composition::{ParticlePart, ParticleShade};
use crate::parts::particle::add_instance_properties;
use crate::{EffectLayer, EffectLayerGroup, EffectPart, EffectRecipe};
use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, EffectAsset, ExprWriter, OrientMode, OrientModifier,
	SetAttributeModifier, SimulationSpace, SpawnerSettings,
};
/// Shared assets/configuration only; clocks and ownership belong to the VFX host.
#[derive(Component, Clone, Debug, EffectRecipe)]
#[effect(name="surrealist-dust",layers=Self::layers)]
pub struct SurrealistDust {
	grains: Vec<EffectLayer>,
}
impl SurrealistDust {
	pub const DENSITY: &'static str = "vfx/surrealist_dust/surrealist.volume_flipbook.json";
	/// Compile a bounded reusable grain kit once, then clone this recipe for instances.
	/// Grain birth/initial states come from the reference; subsequent ballistic motion
	/// approximates its short flow-guided trajectories. Density itself is GPU readback.
	pub fn compile(world: &mut World) -> Self {
		let mut effects = world.resource_mut::<Assets<EffectAsset>>();
		let mut layers = Vec::new();
		for g in grains::TRACKS {
			let life = g.life.min(4.7 - g.birth).max(0.01);
			let writer = ExprWriter::new();
			let props = add_instance_properties(&writer);
			let position =
				SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());
			let velocity = SetAttributeModifier::new(
				Attribute::VELOCITY,
				(writer.lit(Vec3::from_array(g.velocity)) * props.playback.clone()).expr(),
			);
			let age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
			let lifetime = SetAttributeModifier::new(
				Attribute::LIFETIME,
				(writer.lit(life) / props.playback.clone()).expr(),
			);
			let color = SetAttributeModifier::new(
				Attribute::HDR_COLOR,
				(writer.lit(Vec3::from_array(g.color).extend(1.)) * props.tint.clone()).expr(),
			);
			let size = SetAttributeModifier::new(
				Attribute::SIZE3,
				(writer.lit(Vec3::splat(g.size)) * props.scale.clone()).expr(),
			);
			let update_size = crate::parts::particle::update_scaled_size3(
				&writer,
				&props.scale,
				Vec3::splat(g.size),
				Vec3::ZERO,
			);
			let accel = AccelModifier::new(
				(writer.lit(Vec3::new(0., -0.5, 0.))
					* props.scale * props.playback.clone()
					* props.playback)
					.expr(),
			);
			let effect = effects.add(
				EffectAsset::new(1, SpawnerSettings::once(1.0.into()), writer.finish())
					.with_name("surrealist reference grain")
					.with_simulation_space(SimulationSpace::Global)
					.with_alpha_mode(bevy_hanabi::AlphaMode::Blend)
					.init(position)
					.init(velocity)
					.init(age)
					.init(lifetime)
					.init(color)
					.init(size)
					.update(accel)
					.update(update_size)
					.render(OrientModifier::new(OrientMode::FaceCameraPosition)),
			);
			layers.push(
				EffectLayer::particle(ParticlePart {
					name: "surrealist grain".into(),
					effect,
					images: vec![],
					count: 1.,
					capacity: 1,
					max_lifetime: life,
					shade: ParticleShade::Smoke,
				})
				.with_translation(Vec3::from_array(g.position))
				.with_delay(g.birth),
			);
		}
		Self { grains: layers }
	}
	fn layers(&self) -> EffectLayerGroup {
		let mut body = EffectLayer::volume_flipbook_path(Self::DENSITY)
			.with_name("rolling body and fine dust");
		if let EffectPart::VolumeFlipbook(part) = &mut body.part {
			part.tint = Color::linear_rgb(0.65, 0.44, 0.25);
			part.illumination = Vec4::new(-3.8, 5.5, 3., 0.9);
		}
		let mut layers = vec![body];
		layers.extend(self.grains.clone());
		EffectLayerGroup::new(layers)
	}
}

#[cfg(test)]
mod tests {
	use crate::effects::surrealist_dust::SurrealistDust;
	use crate::{EffectPart, EffectRecipe};
	use bevy::prelude::*;
	#[test]
	fn recipe_uses_one_path_clip_and_shared_bounded_grains() -> Result<(), String> {
		let mut world = World::new();
		world.insert_resource(Assets::<bevy_hanabi::EffectAsset>::default());
		let recipe = SurrealistDust::compile(&mut world);
		let a = recipe.effect_definition();
		let b = recipe.clone().effect_definition();
		assert_eq!(a.name, "surrealist-dust");
		assert_eq!(a.layers.len(), crate::effects::surrealist_dust::grains::TRACKS.len() + 1);
		assert!((2..=33).contains(&a.layers.len()));
		let EffectPart::VolumeFlipbook(v) = &a.layers[0].part else {
			return Err("volume missing".into());
		};
		assert_eq!(v.flipbook.path, SurrealistDust::DENSITY);
		assert!(v.illumination.w > 0.);
		for (a, b) in a.layers[1..].iter().zip(&b.layers[1..]) {
			let (EffectPart::Particle(a), EffectPart::Particle(b)) = (&a.part, &b.part) else {
				return Err("grains missing".into());
			};
			assert_eq!(a.effect, b.effect);
			assert_eq!(a.capacity, 1);
		}
		assert!(a.layers[1..].iter().all(|l| l.duration() <= 4.7001));
		Ok(())
	}
}

#[cfg(test)]
mod asset_tests {
	#[test]
	fn packaged_contract_and_terminal_samples() -> Result<(), Box<dyn std::error::Error>> {
		use crate::parts::packaged::VolumeFlipbookManifest;
		let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("../../assets/vfx/surrealist_dust");
		let m = VolumeFlipbookManifest::from_json(&std::fs::read(
			root.join("surrealist.volume_flipbook.json"),
		)?)?;
		assert_eq!(m.frames.len(), 48);
		assert_eq!(m.dimensions, [128, 64, 128]);
		assert_eq!(m.byte_len(), Some(100663296));
		assert!((m.duration_seconds() - 4.7).abs() < 1e-5);
		for index in [0, 47] {
			let b = std::fs::read(root.join(&m.frames[index]))?;
			assert_eq!(b.len(), 152 + 128 * 64 * 128 * 2);
			assert!(b[152..].iter().all(|v| *v == 0));
		}
		Ok(())
	}
}
