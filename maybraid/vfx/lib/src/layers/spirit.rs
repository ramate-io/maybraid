//! Ethereal rising motes and a soft point glow for defeated characters.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, LinearDragModifier, SetAttributeModifier, SetPositionSphereModifier,
	ShapeDimension, SimulationSpace, SpawnerSettings,
};

use crate::composition::{EffectDefinition, EffectLayer, LightPulse, ParticlePart, ParticleShade};
use crate::names::SPIRIT_DISSOLVE;
use crate::palette::ExplosionPalette;
use crate::parts::particle::assets::FlipbookAsset;
use crate::parts::particle::flipbook::update_sprite_index;
use crate::parts::particle::{
	add_instance_properties, init_lifetime, init_palette_color, init_smoke_velocity,
	update_palette_color, update_scaled_size3, value_lifetime_gradient,
};

pub const SPIRIT_COUNT: f32 = 18.0;
pub const SPIRIT_CAPACITY: u32 = 32;
pub const SPIRIT_LIFE_MIN: f32 = 1.4;
pub const SPIRIT_LIFE_MAX: f32 = 2.6;
pub const SPIRIT_LIGHT_FADE: f32 = 2.2;
pub const SPIRIT_LIGHT_PEAK: f32 = 28_000.0;
pub const SPIRIT_LIGHT_RANGE: f32 = 5.5;

pub fn light_pulse() -> LightPulse {
	LightPulse {
		color: ExplosionPalette::color(ExplosionPalette::celestial().flash),
		peak_intensity: SPIRIT_LIGHT_PEAK,
		range: SPIRIT_LIGHT_RANGE,
		fade: SPIRIT_LIGHT_FADE,
	}
}

pub fn compile_wisps(
	effects: &mut Assets<EffectAsset>,
	smoke: &FlipbookAsset,
) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.15, 0.0)).expr(),
		radius: (writer.lit(0.35) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel = init_smoke_velocity(
		&writer,
		&props.scale,
		&props.playback,
		&props.seed,
		0.25,
		0.65,
		2.4,
		false,
	);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, SPIRIT_LIFE_MIN, SPIRIT_LIFE_MAX);
	let init_color = init_palette_color(&props);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(0.55) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(Vec3::new(0.0, 1.8, 0.0)) * props.scale.clone() * props.playback).expr(),
	);
	let update_size = update_scaled_size3(
		&writer,
		&props.scale,
		Vec3::new(0.20, 0.20, 0.20),
		Vec3::new(0.38, 0.38, 0.38),
	);
	let sprite = update_sprite_index(&writer, smoke);
	let color = value_lifetime_gradient(&[(0.0, 0.0), (0.12, 0.85), (0.55, 0.7), (1.0, 0.0)]);

	let effect = effects.add(
		EffectAsset::new(SPIRIT_CAPACITY, SpawnerSettings::once(SPIRIT_COUNT.into()), writer.finish())
			.with_name("vfx-spirit-wisps")
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
			.update(update_color)
			.update(sprite)
			.render(ColorOverLifetimeModifier {
				gradient: color,
				blend: ColorBlendMode::Modulate,
				mask: ColorBlendMask::RGBA,
			}),
	);

	ParticlePart {
		name: SPIRIT_DISSOLVE.into(),
		effect,
		images: vec![smoke.image.clone()],
		count: SPIRIT_COUNT,
		capacity: SPIRIT_CAPACITY,
		max_lifetime: SPIRIT_LIFE_MAX,
		shade: ParticleShade::Spirit,
	}
}

pub fn definition(wisps: ParticlePart) -> EffectDefinition {
	EffectDefinition::new(
		SPIRIT_DISSOLVE,
		[EffectLayer::light(light_pulse()), EffectLayer::particle(wisps)],
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn spirit_dissolve_is_a_slow_rise() {
		let mut effects = Assets::<EffectAsset>::default();
		let smoke = FlipbookAsset {
			image: Handle::default(),
			grid: UVec2::ONE,
			frame_count: 1,
			playback: 1.0,
		};
		let part = compile_wisps(&mut effects, &smoke);
		assert_eq!(definition(part).name, SPIRIT_DISSOLVE);
	}
}
