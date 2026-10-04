//! World-space spark streaks with an elongated tapered mask.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, ImageSampleMapping, LinearDragModifier, OrientMode, OrientModifier,
	ParticleTextureModifier, SetAttributeModifier, SetPositionSphereModifier, ShapeDimension,
	SimulationSpace, SpawnerSettings,
};

use crate::composition::{EffectDefinition, EffectLayer, ParticlePart, SPARKS};
use crate::particles::{add_instance_properties, init_radial_velocity, update_scaled_size3};

pub const SPARKS_COUNT: f32 = 28.0;
pub const SPARKS_CAPACITY: u32 = 48;
pub const SPARKS_LIFE_MIN: f32 = 0.18;
pub const SPARKS_LIFE_MAX: f32 = 0.4;

pub fn compile(effects: &mut Assets<EffectAsset>, taper: Handle<Image>) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: (writer.lit(0.06) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel = init_radial_velocity(&writer, &props.scale, 3.5, 7.0);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = SetAttributeModifier::new(
		Attribute::LIFETIME,
		writer.lit(SPARKS_LIFE_MIN).uniform(writer.lit(SPARKS_LIFE_MAX)).expr(),
	);
	let init_color = SetAttributeModifier::new(Attribute::HDR_COLOR, props.tint.expr());
	let update_drag = LinearDragModifier::new(writer.lit(1.2).expr());
	let update_accel =
		AccelModifier::new((writer.lit(Vec3::new(0.0, -10.0, 0.0)) * props.scale.clone()).expr());
	let update_size = update_scaled_size3(
		&writer,
		&props.scale,
		Vec3::new(0.24, 0.036, 1.0),
		Vec3::new(0.10, 0.014, 1.0),
	);
	let texture_slot = writer.lit(0u32).expr();

	let mut color = bevy_hanabi::Gradient::new();
	color.add_key(0.0, Vec4::new(1.6, 1.05, 0.32, 1.0));
	color.add_key(0.4, Vec4::new(1.2, 0.42, 0.08, 1.0));
	color.add_key(1.0, Vec4::new(0.2, 0.04, 0.01, 0.0));

	let mut module = writer.finish();
	module.add_texture_slot("spark");

	let effect = effects.add(
		EffectAsset::new(SPARKS_CAPACITY, SpawnerSettings::once(SPARKS_COUNT.into()), module)
			.with_name("vfx-sparks")
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Add)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_color)
			.update(update_drag)
			.update(update_accel)
			.update(update_size)
			.render(ParticleTextureModifier {
				texture_slot,
				sample_mapping: ImageSampleMapping::Modulate,
			})
			.render(ColorOverLifetimeModifier {
				gradient: color,
				blend: ColorBlendMode::Modulate,
				mask: ColorBlendMask::RGBA,
			})
			.render(OrientModifier::new(OrientMode::AlongVelocity)),
	);

	ParticlePart {
		name: SPARKS.into(),
		effect,
		images: vec![taper],
		count: SPARKS_COUNT,
		capacity: SPARKS_CAPACITY,
		max_lifetime: SPARKS_LIFE_MAX,
	}
}

pub fn definition(part: ParticlePart) -> EffectDefinition {
	EffectDefinition::new(SPARKS, [EffectLayer::particle(part)])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn compile_is_an_immediate_streak_burst() {
		let mut effects = Assets::<EffectAsset>::default();
		let part = compile(&mut effects, Handle::default());
		assert_eq!(part.name, SPARKS);
		assert_eq!(part.images.len(), 1);
		assert!((definition(part).duration() - SPARKS_LIFE_MAX).abs() < 1e-4);
	}
}
