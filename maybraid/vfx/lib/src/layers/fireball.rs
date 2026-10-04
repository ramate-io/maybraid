//! Reusable expanding fire flipbook emitter.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, FlipbookModifier, ImageSampleMapping, LinearDragModifier, OrientMode,
	OrientModifier, ParticleTextureModifier, SetAttributeModifier, SetPositionSphereModifier,
	SetVelocitySphereModifier, ShapeDimension, SimulationSpace, SizeOverLifetimeModifier,
	SpawnerSettings,
};

use crate::assets::FlipbookAsset;
use crate::composition::{EffectDefinition, EffectLayer, ParticlePart, FIREBALL};
use crate::flipbook::update_sprite_index;

pub const FIREBALL_COUNT: f32 = 6.0;
pub const FIREBALL_CAPACITY: u32 = 16;
pub const FIREBALL_LIFE_MIN: f32 = 0.35;
pub const FIREBALL_LIFE_MAX: f32 = 0.55;

pub fn compile(effects: &mut Assets<EffectAsset>, fire: &FlipbookAsset) -> ParticlePart {
	let writer = ExprWriter::new();
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: writer.lit(0.08).expr(),
		dimension: ShapeDimension::Volume,
	};
	let speed = writer.lit(1.4).uniform(writer.lit(2.8));
	let init_vel =
		SetVelocitySphereModifier { center: writer.lit(Vec3::ZERO).expr(), speed: speed.expr() };
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = SetAttributeModifier::new(
		Attribute::LIFETIME,
		writer.lit(FIREBALL_LIFE_MIN).uniform(writer.lit(FIREBALL_LIFE_MAX)).expr(),
	);
	let init_sprite = SetAttributeModifier::new(Attribute::SPRITE_INDEX, writer.lit(0i32).expr());
	let update_drag = LinearDragModifier::new(writer.lit(1.4).expr());
	let update_accel = AccelModifier::new(writer.lit(Vec3::new(0.0, 0.6, 0.0)).expr());
	let update_sprite = update_sprite_index(&writer, fire);
	let texture_slot = writer.lit(0u32).expr();

	let mut color = bevy_hanabi::Gradient::new();
	color.add_key(0.0, Vec4::new(1.6, 1.2, 0.55, 1.0));
	color.add_key(0.4, Vec4::new(1.2, 0.45, 0.08, 0.9));
	color.add_key(1.0, Vec4::new(0.25, 0.04, 0.01, 0.0));
	let mut size = bevy_hanabi::Gradient::new();
	size.add_key(0.0, Vec3::splat(0.55));
	size.add_key(0.35, Vec3::splat(0.95));
	size.add_key(1.0, Vec3::splat(1.25));

	let mut module = writer.finish();
	module.add_texture_slot("fire");

	let effect = effects.add(
		EffectAsset::new(FIREBALL_CAPACITY, SpawnerSettings::once(FIREBALL_COUNT.into()), module)
			.with_name("vfx-fireball")
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Add)
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
			.render(FlipbookModifier { sprite_grid_size: fire.grid })
			.render(ColorOverLifetimeModifier {
				gradient: color,
				blend: ColorBlendMode::Overwrite,
				mask: ColorBlendMask::RGBA,
			})
			.render(SizeOverLifetimeModifier { gradient: size, screen_space_size: false })
			.render(OrientModifier::new(OrientMode::FaceCameraPosition)),
	);

	ParticlePart {
		name: FIREBALL.into(),
		effect,
		images: vec![fire.image.clone()],
		count: FIREBALL_COUNT,
		capacity: FIREBALL_CAPACITY,
		max_lifetime: FIREBALL_LIFE_MAX,
	}
}

pub fn definition(part: ParticlePart) -> EffectDefinition {
	EffectDefinition::new(FIREBALL, [EffectLayer::particle(part)])
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::assets::fire_flipbook;

	#[test]
	fn compile_preserves_burst_defaults() {
		let mut images = Assets::<Image>::default();
		let mut effects = Assets::<EffectAsset>::default();
		let part = compile(&mut effects, &fire_flipbook(&mut images));
		assert_eq!(part.name, FIREBALL);
		assert_eq!(part.count, FIREBALL_COUNT);
		assert_eq!(part.capacity, FIREBALL_CAPACITY);
		assert!((part.max_lifetime - FIREBALL_LIFE_MAX).abs() < 1e-4);
		assert_eq!(part.images.len(), 1);
	}
}
