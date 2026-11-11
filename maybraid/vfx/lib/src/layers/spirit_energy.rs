//! Shared energy shell, streaks, and embers for spiritual spawn / despawn bursts.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, FlipbookModifier, ImageSampleMapping, LinearDragModifier, OrientMode,
	OrientModifier, ParticleTextureModifier, SetAttributeModifier, SetPositionSphereModifier,
	ShapeDimension, SimulationSpace, SpawnerSettings,
};

use crate::composition::{LobeKind, LobeSpec, MeshPart, ParticlePart, ParticleShade};
use crate::parts::particle::assets::FlipbookAsset;
use crate::parts::particle::flipbook::update_sprite_index;
use crate::parts::particle::{
	add_instance_properties, init_lifetime, init_palette_color, init_radial_velocity,
	seeded_range, update_palette_color, update_scaled_size3, value_lifetime_gradient,
};

pub const STREAK_COUNT: f32 = 56.0;
pub const STREAK_CAPACITY: u32 = 72;
pub const STREAK_LIFE_MIN: f32 = 0.22;
pub const STREAK_LIFE_MAX: f32 = 0.62;

pub const MOTE_COUNT: f32 = 36.0;
pub const MOTE_CAPACITY: u32 = 52;
pub const MOTE_LIFE_MIN: f32 = 0.55;
pub const MOTE_LIFE_MAX: f32 = 1.35;

pub const EMBER_COUNT: f32 = 22.0;
pub const EMBER_CAPACITY: u32 = 32;
pub const EMBER_LIFE_MIN: f32 = 0.35;
pub const EMBER_LIFE_MAX: f32 = 0.95;

pub const SHELL_FADE: f32 = 1.05;
pub const CORE_FLASH_FADE: f32 = 0.14;
pub const CORE_FLASH_PEAK: f32 = 180_000.0;
pub const CORE_FLASH_RANGE: f32 = 9.5;
pub const BURST_LIGHT_PEAK: f32 = 95_000.0;
pub const BURST_LIGHT_RANGE: f32 = 8.0;
pub const BURST_LIGHT_FADE: f32 = 1.35;

pub fn compile_shell(mesh: Handle<Mesh>, name: &str, duration: f32) -> MeshPart {
	MeshPart::duration_from_lobes(
		name,
		mesh,
		LobeKind::Flash,
		vec![
			LobeSpec::new(Vec3::new(0.0, 0.22, 0.0), Vec3::new(1.18, 1.05, 1.18))
				.with_euler(Vec3::new(0.08, 0.35, 0.1))
				.with_expand(1.42)
				.with_roll(0.15)
				.with_duration(duration),
			LobeSpec::new(Vec3::new(0.12, 0.32, 0.06), Vec3::new(0.82, 0.95, 0.82))
				.with_euler(Vec3::new(-0.25, 0.85, 0.22))
				.with_expand(1.28)
				.with_roll(0.35)
				.with_duration(duration * 0.96),
			LobeSpec::new(Vec3::new(-0.1, 0.28, -0.05), Vec3::new(0.88, 0.88, 0.88))
				.with_euler(Vec3::new(0.45, -0.55, -0.18))
				.with_expand(1.22)
				.with_roll(0.28)
				.with_duration(duration * 0.92),
			LobeSpec::new(Vec3::new(0.0, 0.08, 0.0), Vec3::new(1.35, 0.42, 1.35))
				.with_euler(Vec3::new(0.02, 0.12, 0.0))
				.with_expand(1.55)
				.with_duration(duration * 0.88),
		],
	)
}

fn init_converge_streak_velocity(
	writer: &ExprWriter,
	scale: &bevy_hanabi::prelude::WriterExpr,
	playback: &bevy_hanabi::prelude::WriterExpr,
	seed: &bevy_hanabi::prelude::WriterExpr,
) -> SetAttributeModifier {
	let pos = writer.attr(Attribute::POSITION);
	let dir = pos.clone().div(pos.length().max(writer.lit(1e-3)));
	let speed = seeded_range(writer, seed, 3.7, 2.4, 6.2);
	let vel = dir * speed * writer.lit(-1.0) * scale.clone() * playback.clone();
	SetAttributeModifier::new(Attribute::VELOCITY, vel.expr())
}

pub fn compile_outward_streaks(
	effects: &mut Assets<EffectAsset>,
	spark: Handle<Image>,
	part_name: &str,
) -> ParticlePart {
	compile_streaks(effects, spark, part_name, false)
}

pub fn compile_inward_streaks(
	effects: &mut Assets<EffectAsset>,
	spark: Handle<Image>,
	part_name: &str,
) -> ParticlePart {
	compile_streaks(effects, spark, part_name, true)
}

fn compile_streaks(
	effects: &mut Assets<EffectAsset>,
	spark: Handle<Image>,
	part_name: &str,
	inward: bool,
) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.12, 0.0)).expr(),
		radius: (writer.lit(0.08) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel = if inward {
		init_converge_streak_velocity(&writer, &props.scale, &props.playback, &props.seed)
	} else {
		init_radial_velocity(&writer, &props.scale, &props.playback, &props.seed, 5.5, 11.0)
	};
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, STREAK_LIFE_MIN, STREAK_LIFE_MAX);
	let init_color = init_palette_color(&props);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(0.65) * props.playback.clone()).expr());
	let update_size = update_scaled_size3(
		&writer,
		&props.scale,
		Vec3::new(0.22, 0.028, 1.45),
		Vec3::new(0.08, 0.012, 0.55),
	);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&[(0.0, 1.0), (0.25, 1.0), (0.7, 0.65), (1.0, 0.0)]);

	let mut module = writer.finish();
	module.add_texture_slot("spark", bevy_hanabi::SlotDimension::D2);

	let effect = effects.add(
		EffectAsset::new(STREAK_CAPACITY, SpawnerSettings::once(STREAK_COUNT.into()), module)
			.with_name(if inward {
				"vfx-spirit-streaks-in"
			} else {
				"vfx-spirit-streaks-out"
			})
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Add)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_color)
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

pub fn compile_shell_motes(
	effects: &mut Assets<EffectAsset>,
	smoke: &FlipbookAsset,
	part_name: &str,
	outward: bool,
) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.28, 0.0)).expr(),
		radius: (writer.lit(0.62) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Surface,
	};
	let init_vel = if outward {
		init_radial_velocity(&writer, &props.scale, &props.playback, &props.seed, 0.35, 1.1)
	} else {
		init_converge_streak_velocity(&writer, &props.scale, &props.playback, &props.seed)
	};
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = init_lifetime(&writer, &props.playback, &props.seed, MOTE_LIFE_MIN, MOTE_LIFE_MAX);
	let init_sprite = SetAttributeModifier::new(Attribute::SPRITE_INDEX, writer.lit(0i32).expr());
	let init_color = init_palette_color(&props);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(0.45) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(Vec3::new(0.0, if outward { 0.55 } else { -0.35 }, 0.0))
			* props.scale.clone()
			* props.playback)
		.expr(),
	);
	let update_size = update_scaled_size3(
		&writer,
		&props.scale,
		Vec3::splat(if outward { 0.72 } else { 0.95 }),
		Vec3::splat(if outward { 0.95 } else { 0.35 }),
	);
	let sprite = update_sprite_index(&writer, smoke);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&[(0.0, 0.85), (0.35, 0.92), (0.75, 0.55), (1.0, 0.0)]);

	let mut module = writer.finish();
	module.add_texture_slot("smoke", bevy_hanabi::SlotDimension::D2);

	let effect = effects.add(
		EffectAsset::new(MOTE_CAPACITY, SpawnerSettings::once(MOTE_COUNT.into()), module)
			.with_name("vfx-spirit-shell-motes")
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Add)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_sprite)
			.init(init_color)
			.update(update_drag)
			.update(update_accel)
			.update(update_size)
			.update(sprite)
			.update(update_color)
			.render(ParticleTextureModifier {
				texture_slot,
				sample_mapping: ImageSampleMapping::Modulate,
			})
			.render(FlipbookModifier { sprite_grid_size: smoke.grid })
			.render(ColorOverLifetimeModifier {
				gradient: color,
				blend: ColorBlendMode::Modulate,
				mask: ColorBlendMask::RGBA,
			}),
	);

	ParticlePart {
		name: part_name.into(),
		effect,
		images: vec![smoke.image.clone()],
		count: MOTE_COUNT,
		capacity: MOTE_CAPACITY,
		max_lifetime: MOTE_LIFE_MAX,
		shade: ParticleShade::Spirit,
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
		center: writer.lit(Vec3::new(0.0, 0.05, 0.0)).expr(),
		radius: (writer.lit(0.22) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel = init_radial_velocity(&writer, &props.scale, &props.playback, &props.seed, 1.2, 3.8);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, EMBER_LIFE_MIN, EMBER_LIFE_MAX);
	let init_color = init_palette_color(&props);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(1.4) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(Vec3::new(0.0, 2.4, 0.0)) * props.scale.clone() * props.playback).expr(),
	);
	let update_size = update_scaled_size3(
		&writer,
		&props.scale,
		Vec3::new(0.18, 0.18, 0.18),
		Vec3::new(0.08, 0.08, 0.08),
	);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&[(0.0, 1.0), (0.5, 0.85), (1.0, 0.0)]);

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
