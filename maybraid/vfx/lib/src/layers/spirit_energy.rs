//! Shared additive energy layers for spiritual spawn / despawn.
//!
//! These are particle-only. A Flash mesh lobe plus HDR point lights was reading
//! as a solid white ball on a discrete GPU; keep per-particle alpha low, size
//! falling to zero, and light peaks in the hundreds of lumens.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, ImageSampleMapping, LinearDragModifier, OrientMode, OrientModifier,
	ParticleTextureModifier, SetAttributeModifier, SetPositionSphereModifier, ShapeDimension,
	SimulationSpace, SpawnerSettings,
};

use crate::composition::{LightPulse, ParticlePart, ParticleShade};
use crate::palette::ExplosionPalette;
use crate::parts::particle::{
	add_instance_properties, init_lifetime, init_palette_color, init_scaled_size3, seeded_range,
	update_palette_color, update_scaled_size3, value_lifetime_gradient,
};

/// Total spawn envelope: gather, flash, reveal, then staggered peel.
pub const SPIRIT_SPAWN_SECS: f32 = 4.0;
/// Character mesh becomes visible after the core flash.
pub const SPIRIT_SPAWN_REVEAL: f32 = 1.4;
/// Despawn envelope: flash then layered dispersal.
pub const SPIRIT_DISSOLVE_SECS: f32 = 3.2;

pub const STREAK_COUNT: f32 = 28.0;
pub const STREAK_CAPACITY: u32 = 40;
pub const STREAK_LIFE_MIN: f32 = 0.38;
pub const STREAK_LIFE_MAX: f32 = 0.88;
/// AlongVelocity uses SIZE.x as length, SIZE.y as width.
pub const STREAK_SIZE_START: Vec3 = Vec3::new(1.85, 0.034, 1.0);
pub const STREAK_SIZE_END: Vec3 = Vec3::new(0.55, 0.010, 1.0);

pub const WISP_COUNT: f32 = 52.0;
pub const WISP_CAPACITY: u32 = 72;
pub const WISP_LIFE_MIN: f32 = 1.05;
pub const WISP_LIFE_MAX: f32 = 1.65;
pub const WISP_SIZE_START: Vec3 = Vec3::splat(0.48);
pub const WISP_SIZE_END: Vec3 = Vec3::splat(0.06);

pub const CRACKLE_COUNT: f32 = 36.0;
pub const CRACKLE_CAPACITY: u32 = 48;
pub const CRACKLE_LIFE_MIN: f32 = 0.55;
pub const CRACKLE_LIFE_MAX: f32 = 1.15;
pub const CRACKLE_SIZE_START: Vec3 = Vec3::new(0.42, 0.022, 1.0);
pub const CRACKLE_SIZE_END: Vec3 = Vec3::new(0.08, 0.006, 1.0);

pub const EMBER_COUNT: f32 = 28.0;
pub const EMBER_CAPACITY: u32 = 40;
pub const EMBER_LIFE_MIN: f32 = 0.22;
pub const EMBER_LIFE_MAX: f32 = 0.58;
pub const EMBER_SIZE_START: Vec3 = Vec3::splat(0.055);
pub const EMBER_SIZE_END: Vec3 = Vec3::splat(0.008);

/// Soft additive envelope; overlapping wisps must not stack to opaque white.
pub const WISP_ALPHA: [(f32, f32); 4] = [(0.0, 0.16), (0.18, 0.26), (0.62, 0.10), (1.0, 0.0)];
pub const CRACKLE_ALPHA: [(f32, f32); 4] = [(0.0, 0.32), (0.22, 0.40), (0.7, 0.12), (1.0, 0.0)];
pub const STREAK_ALPHA: [(f32, f32); 4] = [(0.0, 0.62), (0.18, 0.78), (0.55, 0.28), (1.0, 0.0)];
pub const EMBER_ALPHA: [(f32, f32); 3] = [(0.0, 0.80), (0.4, 0.45), (1.0, 0.0)];

/// Bevy `PointLight` lumens. Previous peaks (180k / 95k) blew the shot to white.
pub const CORE_FLASH_PEAK: f32 = 1_400.0;
pub const CORE_FLASH_RANGE: f32 = 2.6;
pub const CORE_FLASH_FADE: f32 = 0.18;
pub const AURA_PEAK: f32 = 320.0;
pub const AURA_RANGE: f32 = 3.8;
pub const AURA_FADE: f32 = 2.4;

pub fn core_flash() -> LightPulse {
	LightPulse {
		color: ExplosionPalette::color(LinearRgba::rgb(0.96, 0.98, 1.05)),
		peak_intensity: CORE_FLASH_PEAK,
		range: CORE_FLASH_RANGE,
		fade: CORE_FLASH_FADE,
	}
}

pub fn aura_light() -> LightPulse {
	LightPulse {
		color: ExplosionPalette::color(LinearRgba::rgb(0.45, 0.78, 0.95)),
		peak_intensity: AURA_PEAK,
		range: AURA_RANGE,
		fade: AURA_FADE,
	}
}

fn init_signed_radial_velocity(
	writer: &ExprWriter,
	scale: &bevy_hanabi::prelude::WriterExpr,
	playback: &bevy_hanabi::prelude::WriterExpr,
	seed: &bevy_hanabi::prelude::WriterExpr,
	speed_min: f32,
	speed_max: f32,
	inward: bool,
) -> SetAttributeModifier {
	let pos = writer.attr(Attribute::POSITION);
	let dir = pos.clone().div(pos.length().max(writer.lit(1e-3)));
	let speed = seeded_range(writer, seed, 2.53, speed_min, speed_max);
	let sign = if inward { -1.0 } else { 1.0 };
	let vel = dir * speed * writer.lit(sign) * scale.clone() * playback.clone();
	SetAttributeModifier::new(Attribute::VELOCITY, vel.expr())
}

pub fn compile_wisps(
	effects: &mut Assets<EffectAsset>,
	wisp: Handle<Image>,
	part_name: &str,
	inward: bool,
) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let radius = if inward { 0.62 } else { 0.48 };
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.18, 0.0)).expr(),
		radius: (writer.lit(radius) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Surface,
	};
	let (speed_min, speed_max) = if inward { (0.55, 1.15) } else { (0.22, 0.62) };
	let init_vel = init_signed_radial_velocity(
		&writer,
		&props.scale,
		&props.playback,
		&props.seed,
		speed_min,
		speed_max,
		inward,
	);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = init_lifetime(&writer, &props.playback, &props.seed, WISP_LIFE_MIN, WISP_LIFE_MAX);
	let init_color = init_palette_color(&props);
	let init_size = init_scaled_size3(&writer, &props.scale, WISP_SIZE_START);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(0.35) * props.playback.clone()).expr());
	let lift = if inward { -0.12 } else { 0.28 };
	let update_accel = AccelModifier::new(
		(writer.lit(Vec3::new(0.0, lift, 0.0)) * props.scale.clone() * props.playback).expr(),
	);
	let update_size = update_scaled_size3(&writer, &props.scale, WISP_SIZE_START, WISP_SIZE_END);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&WISP_ALPHA);

	let mut module = writer.finish();
	module.add_texture_slot("energy", bevy_hanabi::SlotDimension::D2);

	let effect = effects.add(
		EffectAsset::new(WISP_CAPACITY, SpawnerSettings::once(WISP_COUNT.into()), module)
			.with_name(if inward {
				"vfx-spirit-wisps-in"
			} else {
				"vfx-spirit-wisps-out"
			})
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Add)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_color)
			.init(init_size)
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
			.render(OrientModifier::new(OrientMode::FaceCameraPosition)),
	);

	ParticlePart {
		name: part_name.into(),
		effect,
		images: vec![wisp],
		count: WISP_COUNT,
		capacity: WISP_CAPACITY,
		max_lifetime: WISP_LIFE_MAX,
		shade: ParticleShade::Spirit,
	}
}

pub fn compile_crackle(
	effects: &mut Assets<EffectAsset>,
	spark: Handle<Image>,
	part_name: &str,
	inward: bool,
) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let radius = if inward { 1.05 } else { 0.58 };
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.16, 0.0)).expr(),
		radius: (writer.lit(radius) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Surface,
	};
	let (speed_min, speed_max) = if inward { (0.7, 1.4) } else { (0.35, 0.95) };
	let init_vel = init_signed_radial_velocity(
		&writer,
		&props.scale,
		&props.playback,
		&props.seed,
		speed_min,
		speed_max,
		inward,
	);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, CRACKLE_LIFE_MIN, CRACKLE_LIFE_MAX);
	let init_color = init_palette_color(&props);
	let init_size = init_scaled_size3(&writer, &props.scale, CRACKLE_SIZE_START);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(0.55) * props.playback.clone()).expr());
	let update_size = update_scaled_size3(&writer, &props.scale, CRACKLE_SIZE_START, CRACKLE_SIZE_END);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&CRACKLE_ALPHA);

	let mut module = writer.finish();
	module.add_texture_slot("spark", bevy_hanabi::SlotDimension::D2);

	let effect = effects.add(
		EffectAsset::new(CRACKLE_CAPACITY, SpawnerSettings::once(CRACKLE_COUNT.into()), module)
			.with_name(if inward {
				"vfx-spirit-crackle-in"
			} else {
				"vfx-spirit-crackle-out"
			})
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Add)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_color)
			.init(init_size)
			.update(update_drag)
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
		name: part_name.into(),
		effect,
		images: vec![spark],
		count: CRACKLE_COUNT,
		capacity: CRACKLE_CAPACITY,
		max_lifetime: CRACKLE_LIFE_MAX,
		shade: ParticleShade::Spark,
	}
}

pub fn compile_streaks(
	effects: &mut Assets<EffectAsset>,
	spark: Handle<Image>,
	part_name: &str,
) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.08, 0.0)).expr(),
		radius: (writer.lit(0.05) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel = init_signed_radial_velocity(
		&writer,
		&props.scale,
		&props.playback,
		&props.seed,
		4.2,
		8.5,
		false,
	);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, STREAK_LIFE_MIN, STREAK_LIFE_MAX);
	let init_color = init_palette_color(&props);
	let init_size = init_scaled_size3(&writer, &props.scale, STREAK_SIZE_START);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(0.85) * props.playback.clone()).expr());
	let update_size = update_scaled_size3(&writer, &props.scale, STREAK_SIZE_START, STREAK_SIZE_END);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&STREAK_ALPHA);

	let mut module = writer.finish();
	module.add_texture_slot("spark", bevy_hanabi::SlotDimension::D2);

	let effect = effects.add(
		EffectAsset::new(STREAK_CAPACITY, SpawnerSettings::once(STREAK_COUNT.into()), module)
			.with_name("vfx-spirit-streaks-out")
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Add)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_color)
			.init(init_size)
			.update(update_drag)
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
		name: part_name.into(),
		effect,
		images: vec![spark],
		count: STREAK_COUNT,
		capacity: STREAK_CAPACITY,
		max_lifetime: STREAK_LIFE_MAX,
		shade: ParticleShade::Spark,
	}
}

pub fn compile_embers(
	effects: &mut Assets<EffectAsset>,
	spark: Handle<Image>,
	part_name: &str,
) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.06, 0.0)).expr(),
		radius: (writer.lit(0.10) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel = init_signed_radial_velocity(
		&writer,
		&props.scale,
		&props.playback,
		&props.seed,
		1.1,
		2.8,
		false,
	);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, EMBER_LIFE_MIN, EMBER_LIFE_MAX);
	let init_color = init_palette_color(&props);
	let init_size = init_scaled_size3(&writer, &props.scale, EMBER_SIZE_START);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(1.6) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(Vec3::new(0.0, 1.4, 0.0)) * props.scale.clone() * props.playback).expr(),
	);
	let update_size = update_scaled_size3(&writer, &props.scale, EMBER_SIZE_START, EMBER_SIZE_END);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&EMBER_ALPHA);

	let mut module = writer.finish();
	module.add_texture_slot("spark", bevy_hanabi::SlotDimension::D2);

	let effect = effects.add(
		EffectAsset::new(EMBER_CAPACITY, SpawnerSettings::once(EMBER_COUNT.into()), module)
			.with_name("vfx-spirit-embers")
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Add)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_color)
			.init(init_size)
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
		name: part_name.into(),
		effect,
		images: vec![spark],
		count: EMBER_COUNT,
		capacity: EMBER_CAPACITY,
		max_lifetime: EMBER_LIFE_MAX,
		shade: ParticleShade::Spark,
	}
}
