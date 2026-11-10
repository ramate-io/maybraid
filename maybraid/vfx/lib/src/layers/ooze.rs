//! Stylized viscous ooze: forward-biased droplets, trailing drips, and ground splats.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, ImageSampleMapping, LinearDragModifier, OrientMode, OrientModifier,
	ParticleTextureModifier, SetAttributeModifier, SetPositionSphereModifier, ShapeDimension,
	SimulationSpace, SpawnerSettings,
};

use crate::composition::{EffectDefinition, EffectLayer, ParticlePart, ParticleShade};
use crate::names::{OOZE_BURST, SMOKE};
use crate::parts::particle::{
	add_instance_properties, init_lifetime, init_palette_color, seeded_range, update_palette_color,
	update_scaled_size3, value_lifetime_gradient,
};

pub const OOZE_DROPLETS_COUNT: f32 = 22.0;
pub const OOZE_DROPLETS_CAPACITY: u32 = 40;
pub const OOZE_DROPLET_LIFE_MIN: f32 = 0.22;
pub const OOZE_DROPLET_LIFE_MAX: f32 = 0.55;

pub const OOZE_DRIP_COUNT: f32 = 6.0;
pub const OOZE_DRIP_CAPACITY: u32 = 12;
pub const OOZE_DRIP_LIFE_MIN: f32 = 0.45;
pub const OOZE_DRIP_LIFE_MAX: f32 = 1.1;
pub const OOZE_DRIP_DELAY: f32 = 0.06;

pub const OOZE_GRAVITY: Vec3 = Vec3::new(0.0, -16.5, 0.0);

fn init_cone_velocity(
	writer: &ExprWriter,
	scale: &bevy_hanabi::prelude::WriterExpr,
	playback: &bevy_hanabi::prelude::WriterExpr,
	seed: &bevy_hanabi::prelude::WriterExpr,
	speed_min: f32,
	speed_max: f32,
) -> SetAttributeModifier {
	let pos = writer.attr(Attribute::POSITION);
	let forward = writer.lit(Vec3::Z);
	let lateral = pos.clone().sub(writer.lit(Vec3::new(0.0, 0.0, 0.0)));
	let spread = seeded_range(writer, seed, 4.2, 0.12, 0.55);
	let speed = seeded_range(writer, seed, 5.1, speed_min, speed_max);
	let vel = (forward * speed + lateral * spread) * scale.clone() * playback.clone();
	SetAttributeModifier::new(Attribute::VELOCITY, vel.expr())
}

pub fn compile_droplets(effects: &mut Assets<EffectAsset>, blob: Handle<Image>) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: (writer.lit(0.05) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel = init_cone_velocity(&writer, &props.scale, &props.playback, &props.seed, 2.8, 6.8);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = init_lifetime(
		&writer,
		&props.playback,
		&props.seed,
		OOZE_DROPLET_LIFE_MIN,
		OOZE_DROPLET_LIFE_MAX,
	);
	let init_color = init_palette_color(&props);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(2.4) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(OOZE_GRAVITY) * props.scale.clone() * props.playback).expr(),
	);
	let update_size = update_scaled_size3(
		&writer,
		&props.scale,
		Vec3::new(0.18, 0.18, 0.55),
		Vec3::new(0.10, 0.10, 0.22),
	);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&[(0.0, 0.95), (0.35, 0.85), (0.75, 0.4), (1.0, 0.0)]);

	let mut module = writer.finish();
	module.add_texture_slot("ooze", bevy_hanabi::SlotDimension::D2);

	let effect = effects.add(
		EffectAsset::new(
			OOZE_DROPLETS_CAPACITY,
			SpawnerSettings::once(OOZE_DROPLETS_COUNT.into()),
			module,
		)
		.with_name("vfx-ooze-droplets")
		.with_simulation_space(SimulationSpace::Global)
		.with_alpha_mode(bevy_hanabi::AlphaMode::Blend)
		.init(init_pos)
		.init(init_vel)
		.init(init_age)
		.init(init_lifetime)
		.init(init_color)
		.update(update_drag)
		.update(update_accel)
		.update(update_size)
		.update(update_color)
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
		name: OOZE_BURST.into(),
		effect,
		images: vec![blob],
		count: OOZE_DROPLETS_COUNT,
		capacity: OOZE_DROPLETS_CAPACITY,
		max_lifetime: OOZE_DROPLET_LIFE_MAX,
		shade: ParticleShade::Ooze,
	}
}

pub fn compile_drips(effects: &mut Assets<EffectAsset>, blob: Handle<Image>) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.02, 0.08)).expr(),
		radius: (writer.lit(0.18) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Surface,
	};
	let speed = seeded_range(&writer, &props.seed, 7.3, 0.15, 0.55);
	let vel = (writer.lit(Vec3::NEG_Y) * speed + writer.lit(Vec3::Z) * writer.lit(0.08))
		* props.scale.clone()
		* props.playback.clone();
	let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, vel.expr());
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, OOZE_DRIP_LIFE_MIN, OOZE_DRIP_LIFE_MAX);
	let init_color = init_palette_color(&props);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(3.5) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(OOZE_GRAVITY * 0.65) * props.scale.clone() * props.playback).expr(),
	);
	let update_size = update_scaled_size3(
		&writer,
		&props.scale,
		Vec3::new(0.22, 0.22, 0.08),
		Vec3::new(0.34, 0.34, 0.04),
	);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&[(0.0, 0.7), (0.5, 0.55), (1.0, 0.0)]);

	let mut module = writer.finish();
	module.add_texture_slot("ooze", bevy_hanabi::SlotDimension::D2);

	let effect = effects.add(
		EffectAsset::new(OOZE_DRIP_CAPACITY, SpawnerSettings::once(OOZE_DRIP_COUNT.into()), module)
			.with_name("vfx-ooze-drips")
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Blend)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_color)
			.update(update_drag)
			.update(update_accel)
			.update(update_size)
			.update(update_color)
			.render(ParticleTextureModifier {
				texture_slot,
				sample_mapping: ImageSampleMapping::Modulate,
			})
			.render(ColorOverLifetimeModifier {
				gradient: color,
				blend: ColorBlendMode::Modulate,
				mask: ColorBlendMask::RGBA,
			}),
	);

	ParticlePart {
		name: SMOKE.into(),
		effect,
		images: vec![blob],
		count: OOZE_DRIP_COUNT,
		capacity: OOZE_DRIP_CAPACITY,
		max_lifetime: OOZE_DRIP_LIFE_MAX,
		shade: ParticleShade::Ooze,
	}
}

pub fn definition(droplets: ParticlePart, drips: ParticlePart) -> EffectDefinition {
	EffectDefinition::new(
		OOZE_BURST,
		[
			EffectLayer::particle(droplets),
			EffectLayer::particle(drips).with_delay(OOZE_DRIP_DELAY),
		],
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ooze_burst_covers_droplets_and_delayed_drips() {
		let mut effects = Assets::<EffectAsset>::default();
		let part = compile_droplets(&mut effects, Handle::default());
		let drips = compile_drips(&mut effects, Handle::default());
		let def = definition(part, drips);
		assert_eq!(def.name, OOZE_BURST);
		assert!(
			(def.duration() - OOZE_DRIP_LIFE_MAX - OOZE_DRIP_DELAY).abs() < 1e-4,
			"duration {}",
			def.duration()
		);
	}
}
