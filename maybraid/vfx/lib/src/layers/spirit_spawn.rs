//! Converging celestial motes and a soft glow for character materialization.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, FlipbookModifier, ImageSampleMapping, LinearDragModifier,
	ParticleTextureModifier, SetAttributeModifier, SetPositionSphereModifier, ShapeDimension,
	SimulationSpace, SpawnerSettings,
};

use crate::composition::{
	EffectDefinition, EffectLayer, LightPulse, LobeKind, LobeSpec, MeshPart, ParticlePart,
	ParticleShade,
};
use crate::layers::spirit::{self, SPIRIT_CAPACITY, SPIRIT_COUNT, light_pulse};
use crate::names::SPIRIT_SPAWN;
use crate::parts::particle::assets::FlipbookAsset;
use crate::parts::particle::flipbook::update_sprite_index;
use crate::parts::particle::{
	add_instance_properties, init_lifetime, init_palette_color, seeded_range, update_palette_color,
	update_scaled_size3, value_lifetime_gradient,
};

pub const SPIRIT_SPAWN_LIFE_MIN: f32 = 0.95;
pub const SPIRIT_SPAWN_LIFE_MAX: f32 = 1.75;
pub const SPIRIT_SPAWN_MESH_FADE: f32 = 1.15;

pub fn spawn_light_pulse() -> LightPulse {
	let base = light_pulse();
	LightPulse {
		peak_intensity: base.peak_intensity * 0.85,
		fade: base.fade * 0.75,
		..base
	}
}

pub fn compile_core(mesh: Handle<Mesh>) -> MeshPart {
	let mut core = spirit::compile_core(mesh);
	core.name = SPIRIT_SPAWN.into();
	core.duration = SPIRIT_SPAWN_MESH_FADE;
	core
}

fn init_converge_velocity(
	writer: &ExprWriter,
	scale: &bevy_hanabi::prelude::WriterExpr,
	playback: &bevy_hanabi::prelude::WriterExpr,
	seed: &bevy_hanabi::prelude::WriterExpr,
) -> SetAttributeModifier {
	let pos = writer.attr(Attribute::POSITION);
	let dir = pos.clone().div(pos.length().max(writer.lit(1e-3)));
	let inward = seeded_range(writer, seed, 2.2, 1.1, 2.6);
	let fall = seeded_range(writer, seed, 4.7, 0.55, 1.2);
	let vel = (dir * inward * writer.lit(-1.0) + writer.lit(Vec3::new(0.0, -1.0, 0.0)) * fall)
		* scale.clone()
		* playback.clone();
	SetAttributeModifier::new(Attribute::VELOCITY, vel.expr())
}

pub fn compile_wisps(
	effects: &mut Assets<EffectAsset>,
	smoke: &FlipbookAsset,
) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.85, 0.0)).expr(),
		radius: (writer.lit(0.95) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel = init_converge_velocity(&writer, &props.scale, &props.playback, &props.seed);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = init_lifetime(
		&writer,
		&props.playback,
		&props.seed,
		SPIRIT_SPAWN_LIFE_MIN,
		SPIRIT_SPAWN_LIFE_MAX,
	);
	let init_sprite = SetAttributeModifier::new(Attribute::SPRITE_INDEX, writer.lit(0i32).expr());
	let init_color = init_palette_color(&props);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(0.85) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(Vec3::new(0.0, -2.4, 0.0)) * props.scale.clone() * props.playback).expr(),
	);
	let update_size = update_scaled_size3(
		&writer,
		&props.scale,
		Vec3::new(0.62, 0.62, 0.62),
		Vec3::new(0.18, 0.18, 0.18),
	);
	let sprite = update_sprite_index(&writer, smoke);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&[(0.0, 0.9), (0.42, 0.8), (0.78, 0.35), (1.0, 0.0)]);

	let mut module = writer.finish();
	module.add_texture_slot("smoke", bevy_hanabi::SlotDimension::D2);

	let effect = effects.add(
		EffectAsset::new(SPIRIT_CAPACITY, SpawnerSettings::once(SPIRIT_COUNT.into()), module)
			.with_name("vfx-spirit-spawn-wisps")
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
		name: SPIRIT_SPAWN.into(),
		effect,
		images: vec![smoke.image.clone()],
		count: SPIRIT_COUNT,
		capacity: SPIRIT_CAPACITY,
		max_lifetime: SPIRIT_SPAWN_LIFE_MAX,
		shade: ParticleShade::Spirit,
	}
}

pub fn definition(core: MeshPart, wisps: ParticlePart) -> EffectDefinition {
	EffectDefinition::new(
		SPIRIT_SPAWN,
		[
			EffectLayer::mesh(core),
			EffectLayer::light(spawn_light_pulse()),
			EffectLayer::particle(wisps),
		],
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn spirit_spawn_converges_faster_than_dissolve() {
		let mut effects = Assets::<EffectAsset>::default();
		let smoke = FlipbookAsset {
			image: Handle::default(),
			grid: UVec2::ONE,
			frame_count: 1,
			playback: 1.0,
		};
		let part = compile_wisps(&mut effects, &smoke);
		let def = definition(compile_core(Handle::default()), part);
		assert_eq!(def.name, SPIRIT_SPAWN);
		assert!(
			def.duration() <= crate::layers::spirit::SPIRIT_LIFE_MAX + 0.5,
			"spawn {}",
			def.duration()
		);
	}
}
