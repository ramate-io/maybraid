//! Reusable drifting smoke flipbook emitter.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, FlipbookModifier, ImageSampleMapping, LinearDragModifier, OrientMode,
	OrientModifier, ParticleTextureModifier, SetAttributeModifier, SetPositionSphereModifier,
	SetVelocitySphereModifier, ShapeDimension, SimulationSpace, SizeOverLifetimeModifier,
	SpawnerSettings,
};

use crate::assets::FlipbookAsset;
use crate::composition::{EffectDefinition, EffectLayer, ParticlePart, SMOKE};
use crate::flipbook::update_sprite_index;

pub const SMOKE_COUNT: f32 = 10.0;
pub const SMOKE_CAPACITY: u32 = 32;
pub const SMOKE_LIFE_MIN: f32 = 1.2;
pub const SMOKE_LIFE_MAX: f32 = 2.4;
pub const SMOKE_DELAY: f32 = 0.05;

pub fn compile(effects: &mut Assets<EffectAsset>, smoke: &FlipbookAsset) -> ParticlePart {
	let writer = ExprWriter::new();
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: writer.lit(0.14).expr(),
		dimension: ShapeDimension::Volume,
	};
	let speed = writer.lit(0.35).uniform(writer.lit(0.9));
	let init_vel = SetVelocitySphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.35, 0.0)).expr(),
		speed: speed.expr(),
	};
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = SetAttributeModifier::new(
		Attribute::LIFETIME,
		writer.lit(SMOKE_LIFE_MIN).uniform(writer.lit(SMOKE_LIFE_MAX)).expr(),
	);
	let init_sprite = SetAttributeModifier::new(Attribute::SPRITE_INDEX, writer.lit(0i32).expr());
	let update_drag = LinearDragModifier::new(writer.lit(1.1).expr());
	let update_accel = AccelModifier::new(writer.lit(Vec3::new(0.0, 0.85, 0.0)).expr());
	let update_sprite = update_sprite_index(&writer, smoke);
	let texture_slot = writer.lit(0u32).expr();

	let mut color = bevy_hanabi::Gradient::new();
	color.add_key(0.0, Vec4::new(0.55, 0.54, 0.52, 0.55));
	color.add_key(0.4, Vec4::new(0.32, 0.32, 0.33, 0.28));
	color.add_key(1.0, Vec4::new(0.16, 0.16, 0.17, 0.0));
	let mut size = bevy_hanabi::Gradient::new();
	size.add_key(0.0, Vec3::splat(0.55));
	size.add_key(0.5, Vec3::splat(0.95));
	size.add_key(1.0, Vec3::splat(1.35));

	let mut module = writer.finish();
	module.add_texture_slot("smoke");

	let effect = effects.add(
		EffectAsset::new(SMOKE_CAPACITY, SpawnerSettings::once(SMOKE_COUNT.into()), module)
			.with_name("vfx-smoke")
			.with_simulation_space(SimulationSpace::Local)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Blend)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_sprite)
			.update(update_drag)
			.update(update_accel)
			.update(update_sprite)
			.render(ParticleTextureModifier {
				texture_slot,
				sample_mapping: ImageSampleMapping::Modulate,
			})
			.render(FlipbookModifier { sprite_grid_size: smoke.grid })
			.render(ColorOverLifetimeModifier {
				gradient: color,
				blend: ColorBlendMode::Overwrite,
				mask: ColorBlendMask::RGBA,
			})
			.render(SizeOverLifetimeModifier { gradient: size, screen_space_size: false })
			.render(OrientModifier::new(OrientMode::FaceCameraPosition)),
	);

	ParticlePart {
		name: SMOKE.into(),
		effect,
		images: vec![smoke.image.clone()],
		count: SMOKE_COUNT,
		capacity: SMOKE_CAPACITY,
		max_lifetime: SMOKE_LIFE_MAX,
	}
}

pub fn definition(part: ParticlePart) -> EffectDefinition {
	EffectDefinition::new(SMOKE, [EffectLayer::particle(part).with_delay(SMOKE_DELAY)])
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::assets::smoke_flipbook;

	#[test]
	fn standalone_smoke_keeps_its_delay() {
		let mut images = Assets::<Image>::default();
		let mut effects = Assets::<EffectAsset>::default();
		let part = compile(&mut effects, &smoke_flipbook(&mut images));
		let def = definition(part);
		assert_eq!(def.layers.len(), 1);
		assert!((def.layers[0].delay - SMOKE_DELAY).abs() < 1e-4);
		assert!((def.duration() - (SMOKE_DELAY + SMOKE_LIFE_MAX)).abs() < 1e-4);
	}
}
