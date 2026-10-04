//! Reusable outward spark burst.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, LinearDragModifier, OrientMode, OrientModifier, SetAttributeModifier,
	SetPositionSphereModifier, SetVelocitySphereModifier, ShapeDimension, SimulationSpace,
	SizeOverLifetimeModifier, SpawnerSettings,
};

use crate::composition::{EffectDefinition, EffectLayer, ParticlePart, SPARKS};

pub const SPARKS_COUNT: f32 = 28.0;
pub const SPARKS_CAPACITY: u32 = 48;
pub const SPARKS_LIFE_MIN: f32 = 0.18;
pub const SPARKS_LIFE_MAX: f32 = 0.4;

pub fn compile(effects: &mut Assets<EffectAsset>) -> ParticlePart {
	let writer = ExprWriter::new();
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: writer.lit(0.06).expr(),
		dimension: ShapeDimension::Volume,
	};
	let speed = writer.lit(5.0).uniform(writer.lit(11.0));
	let init_vel =
		SetVelocitySphereModifier { center: writer.lit(Vec3::ZERO).expr(), speed: speed.expr() };
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = SetAttributeModifier::new(
		Attribute::LIFETIME,
		writer.lit(SPARKS_LIFE_MIN).uniform(writer.lit(SPARKS_LIFE_MAX)).expr(),
	);
	let update_drag = LinearDragModifier::new(writer.lit(1.8).expr());
	let update_accel = AccelModifier::new(writer.lit(Vec3::new(0.0, -14.0, 0.0)).expr());

	let mut color = bevy_hanabi::Gradient::new();
	color.add_key(0.0, Vec4::new(4.0, 3.1, 1.2, 1.0));
	color.add_key(0.4, Vec4::new(2.2, 0.7, 0.1, 1.0));
	color.add_key(1.0, Vec4::new(0.4, 0.05, 0.01, 0.0));
	let mut size = bevy_hanabi::Gradient::new();
	size.add_key(0.0, Vec3::splat(0.07));
	size.add_key(1.0, Vec3::splat(0.012));

	let effect = effects.add(
		EffectAsset::new(
			SPARKS_CAPACITY,
			SpawnerSettings::once(SPARKS_COUNT.into()),
			writer.finish(),
		)
		.with_name("vfx-sparks")
		.with_simulation_space(SimulationSpace::Local)
		.with_alpha_mode(bevy_hanabi::AlphaMode::Add)
		.init(init_pos)
		.init(init_vel)
		.init(init_age)
		.init(init_lifetime)
		.update(update_drag)
		.update(update_accel)
		.render(ColorOverLifetimeModifier {
			gradient: color,
			blend: ColorBlendMode::Overwrite,
			mask: ColorBlendMask::RGBA,
		})
		.render(SizeOverLifetimeModifier { gradient: size, screen_space_size: false })
		.render(OrientModifier::new(OrientMode::AlongVelocity)),
	);

	ParticlePart {
		name: SPARKS.into(),
		effect,
		images: Vec::new(),
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
	fn compile_is_an_immediate_burst() {
		let mut effects = Assets::<EffectAsset>::default();
		let part = compile(&mut effects);
		assert_eq!(part.name, SPARKS);
		assert!(part.images.is_empty());
		assert!((definition(part).duration() - SPARKS_LIFE_MAX).abs() < 1e-4);
	}
}
