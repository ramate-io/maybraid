//! Crimson droplet spray: small blobs thrown along the hit, falling and fading.
//!
//! No mesh flash. A Smoke/Flash lobe plus `ColorBlendMode::Overwrite` on a
//! multiplied-dark palette was reading as a solid black hit on GPU.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, FlipbookModifier, ImageSampleMapping, LinearDragModifier, OrientMode,
	OrientModifier, ParticleTextureModifier, SetAttributeModifier, SetPositionSphereModifier,
	ShapeDimension, SimulationSpace, SpawnerSettings,
};

use crate::composition::{
	EffectDefinition, EffectLayer, EffectPart, ParticlePart, ParticleShade,
};
use crate::names::{OOZE_BURST, SMOKE};
use crate::parts::particle::assets::FlipbookAsset;
use crate::parts::particle::flipbook::update_sprite_index;
use crate::parts::particle::{
	add_instance_properties, init_lifetime, init_palette_color, init_scaled_size3, seeded_range,
	update_palette_color, update_scaled_size3, value_lifetime_gradient,
};

pub const OOZE_DROPLETS_COUNT: f32 = 96.0;
pub const OOZE_DROPLETS_CAPACITY: u32 = 128;
pub const OOZE_DROPLET_LIFE_MIN: f32 = 0.7;
pub const OOZE_DROPLET_LIFE_MAX: f32 = 1.35;
pub const OOZE_DROPLET_SIZE_START: Vec3 = Vec3::splat(0.16);
pub const OOZE_DROPLET_SIZE_END: Vec3 = Vec3::splat(0.05);

pub const OOZE_DRIP_COUNT: f32 = 22.0;
pub const OOZE_DRIP_CAPACITY: u32 = 32;
pub const OOZE_DRIP_LIFE_MIN: f32 = 0.55;
pub const OOZE_DRIP_LIFE_MAX: f32 = 1.15;
pub const OOZE_DRIP_DELAY: f32 = 0.05;
pub const OOZE_DRIP_SIZE_START: Vec3 = Vec3::splat(0.09);
pub const OOZE_DRIP_SIZE_END: Vec3 = Vec3::splat(0.03);

pub const OOZE_MIST_COUNT: f32 = 22.0;
pub const OOZE_MIST_CAPACITY: u32 = 32;
pub const OOZE_MIST_LIFE_MIN: f32 = 0.4;
pub const OOZE_MIST_LIFE_MAX: f32 = 0.9;
pub const OOZE_MIST_SIZE_START: Vec3 = Vec3::splat(0.10);
pub const OOZE_MIST_SIZE_END: Vec3 = Vec3::splat(0.03);

pub const OOZE_GRAVITY: Vec3 = Vec3::new(0.0, -7.8, 0.0);

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
	let lateral = pos;
	let spread = seeded_range(writer, seed, 4.2, 8.0, 16.0);
	let speed = seeded_range(writer, seed, 5.1, speed_min, speed_max);
	let up = writer.lit(Vec3::Y) * seeded_range(writer, seed, 6.4, 0.8, 2.4);
	let vel = (forward * speed + lateral * spread + up) * scale.clone() * playback.clone();
	SetAttributeModifier::new(Attribute::VELOCITY, vel.expr())
}

pub fn compile_droplets(effects: &mut Assets<EffectAsset>, blob: Handle<Image>) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: (writer.lit(0.07) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel = init_cone_velocity(&writer, &props.scale, &props.playback, &props.seed, 3.4, 6.2);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = init_lifetime(
		&writer,
		&props.playback,
		&props.seed,
		OOZE_DROPLET_LIFE_MIN,
		OOZE_DROPLET_LIFE_MAX,
	);
	let init_color = init_palette_color(&props);
	let init_size = init_scaled_size3(&writer, &props.scale, OOZE_DROPLET_SIZE_START);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(0.35) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(OOZE_GRAVITY) * props.scale.clone() * props.playback).expr(),
	);
	let update_size =
		update_scaled_size3(&writer, &props.scale, OOZE_DROPLET_SIZE_START, OOZE_DROPLET_SIZE_END);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&[(0.0, 0.92), (0.12, 1.0), (0.58, 0.72), (1.0, 0.0)]);

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
		center: writer.lit(Vec3::new(0.0, 0.02, 0.06)).expr(),
		radius: (writer.lit(0.12) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Surface,
	};
	let speed = seeded_range(&writer, &props.seed, 7.3, 0.2, 0.7);
	let vel = (writer.lit(Vec3::NEG_Y) * speed + writer.lit(Vec3::Z) * writer.lit(0.12))
		* props.scale.clone()
		* props.playback.clone();
	let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, vel.expr());
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, OOZE_DRIP_LIFE_MIN, OOZE_DRIP_LIFE_MAX);
	let init_color = init_palette_color(&props);
	let init_size = init_scaled_size3(&writer, &props.scale, OOZE_DRIP_SIZE_START);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(2.4) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(OOZE_GRAVITY * 0.7) * props.scale.clone() * props.playback).expr(),
	);
	let update_size =
		update_scaled_size3(&writer, &props.scale, OOZE_DRIP_SIZE_START, OOZE_DRIP_SIZE_END);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&[(0.0, 0.88), (0.35, 0.7), (1.0, 0.0)]);

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
		name: SMOKE.into(),
		effect,
		images: vec![blob],
		count: OOZE_DRIP_COUNT,
		capacity: OOZE_DRIP_CAPACITY,
		max_lifetime: OOZE_DRIP_LIFE_MAX,
		shade: ParticleShade::Ooze,
	}
}

pub fn compile_mist(effects: &mut Assets<EffectAsset>, smoke: &FlipbookAsset) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.04, 0.04)).expr(),
		radius: (writer.lit(0.08) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel = init_cone_velocity(&writer, &props.scale, &props.playback, &props.seed, 0.4, 1.1);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, OOZE_MIST_LIFE_MIN, OOZE_MIST_LIFE_MAX);
	let init_sprite = SetAttributeModifier::new(Attribute::SPRITE_INDEX, writer.lit(0i32).expr());
	let init_color = init_palette_color(&props);
	let init_size = init_scaled_size3(&writer, &props.scale, OOZE_MIST_SIZE_START);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(1.8) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(Vec3::new(0.0, -3.5, 0.0)) * props.scale.clone() * props.playback).expr(),
	);
	let update_size =
		update_scaled_size3(&writer, &props.scale, OOZE_MIST_SIZE_START, OOZE_MIST_SIZE_END);
	let sprite = update_sprite_index(&writer, smoke);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&[(0.0, 0.18), (0.3, 0.14), (1.0, 0.0)]);

	let mut module = writer.finish();
	module.add_texture_slot("smoke", bevy_hanabi::SlotDimension::D2);

	let effect = effects.add(
		EffectAsset::new(OOZE_MIST_CAPACITY, SpawnerSettings::once(OOZE_MIST_COUNT.into()), module)
			.with_name("vfx-ooze-mist")
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Blend)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_sprite)
			.init(init_color)
			.init(init_size)
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
			})
			.render(OrientModifier::new(OrientMode::FaceCameraPosition)),
	);

	ParticlePart {
		name: format!("{OOZE_BURST}-mist"),
		effect,
		images: vec![smoke.image.clone()],
		count: OOZE_MIST_COUNT,
		capacity: OOZE_MIST_CAPACITY,
		max_lifetime: OOZE_MIST_LIFE_MAX,
		shade: ParticleShade::Ooze,
	}
}

pub fn definition(droplets: ParticlePart, drips: ParticlePart, mist: ParticlePart) -> EffectDefinition {
	EffectDefinition::new(
		OOZE_BURST,
		[
			EffectLayer::particle(droplets),
			EffectLayer::particle(mist),
			EffectLayer::particle(drips).with_delay(OOZE_DRIP_DELAY),
		],
	)
}

pub fn has_mesh_layer(definition: &EffectDefinition) -> bool {
	definition
		.layers
		.iter()
		.any(|layer| matches!(layer.part, EffectPart::Mesh(_)))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ooze_burst_is_droplets_mist_and_delayed_drips() {
		let mut effects = Assets::<EffectAsset>::default();
		let smoke = FlipbookAsset {
			image: Handle::default(),
			grid: UVec2::ONE,
			frame_count: 1,
			playback: 1.0,
		};
		let def = definition(
			compile_droplets(&mut effects, Handle::default()),
			compile_drips(&mut effects, Handle::default()),
			compile_mist(&mut effects, &smoke),
		);
		assert_eq!(def.name, OOZE_BURST);
		assert!(!has_mesh_layer(&def));
		assert_eq!(def.layers.len(), 3);
		let longest = OOZE_DROPLET_LIFE_MAX
			.max(OOZE_MIST_LIFE_MAX)
			.max(OOZE_DRIP_LIFE_MAX + OOZE_DRIP_DELAY);
		assert!(
			(def.duration() - longest).abs() < 1e-4,
			"duration {}",
			def.duration()
		);
	}
}
