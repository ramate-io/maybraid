//! Standalone directional soundwave layer.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, EffectMaterial, EffectProperties, ExprWriter, ImageSampleMapping,
	LinearDragModifier, OrientMode, OrientModifier, ParticleEffect, ParticleTextureModifier,
	SetAttributeModifier, SetPositionCircleModifier, ShapeDimension, SimulationSpace,
	SpawnerSettings,
};

use crate::composition::{
	EffectDefinition, EffectLayer, ParticlePart, ParticleShade, SoundwavePart,
};
use crate::names::SOUNDWAVE;
use crate::parts::particle::{
	add_instance_properties, init_lifetime, init_palette_color, seeded_range, update_palette_color,
	update_scaled_size3, value_lifetime_gradient, PROP_COLOR0, PROP_COLOR1, PROP_COLOR2,
	PROP_PLAYBACK, PROP_SCALE, PROP_SEED, PROP_TINT,
};
use crate::parts::soundwave::soundwave_torus_mesh;

pub const SOUNDWAVE_DURATION: f32 = 1.6;

pub const SOUNDWAVE_WISP_NAME: &str = "vfx-soundwave-bray-wisps";
pub const SOUNDWAVE_STREAK_NAME: &str = "vfx-soundwave-bray-streaks";

pub const WISP_COUNT: f32 = 36.0;
pub const WISP_CAPACITY: u32 = 96;
pub const WISP_RATE: f32 = 64.0;
pub const WISP_LIFE_MIN: f32 = 0.55;
pub const WISP_LIFE_MAX: f32 = 1.15;
pub const WISP_SIZE_START: Vec3 = Vec3::splat(0.85);
pub const WISP_SIZE_END: Vec3 = Vec3::splat(0.12);
pub const WISP_ALPHA: [(f32, f32); 4] = [(0.0, 0.16), (0.18, 0.26), (0.62, 0.10), (1.0, 0.0)];

pub const STREAK_COUNT: f32 = 22.0;
pub const STREAK_CAPACITY: u32 = 64;
pub const STREAK_RATE: f32 = 40.0;
pub const STREAK_LIFE_MIN: f32 = 0.38;
pub const STREAK_LIFE_MAX: f32 = 0.88;
pub const STREAK_SIZE_START: Vec3 = Vec3::new(2.6, 0.055, 1.0);
pub const STREAK_SIZE_END: Vec3 = Vec3::new(0.85, 0.016, 1.0);
pub const STREAK_ALPHA: [(f32, f32); 4] = [(0.0, 0.62), (0.18, 0.78), (0.55, 0.28), (1.0, 0.0)];

pub fn compile_mesh(mesh: Handle<Mesh>) -> SoundwavePart {
	SoundwavePart { name: SOUNDWAVE.into(), mesh, duration: SOUNDWAVE_DURATION }
}

/// Pose the Y-axle torus so the disc faces a horizontal Bray heading (−Z).
pub fn facing_player_transform() -> Transform {
	Transform {
		translation: Vec3::ZERO,
		rotation: Quat::from_rotation_arc(Vec3::Y, Vec3::NEG_Z),
		scale: Vec3::ONE,
	}
}

pub fn definition(part: SoundwavePart) -> EffectDefinition {
	EffectDefinition::new(
		SOUNDWAVE,
		[EffectLayer::soundwave(part).with_transform(facing_player_transform())],
	)
}

pub fn definition_with_trail(
	part: SoundwavePart,
	wisps: ParticlePart,
	streaks: ParticlePart,
) -> EffectDefinition {
	EffectDefinition::new(
		SOUNDWAVE,
		[
			EffectLayer::soundwave(part).with_transform(facing_player_transform()),
			EffectLayer::particle(wisps),
			EffectLayer::particle(streaks),
		],
	)
}

pub fn torus_mesh() -> Mesh {
	soundwave_torus_mesh()
}

fn init_along_negative_y(
	writer: &ExprWriter,
	scale: &bevy_hanabi::prelude::WriterExpr,
	playback: &bevy_hanabi::prelude::WriterExpr,
	seed: &bevy_hanabi::prelude::WriterExpr,
	speed_min: f32,
	speed_max: f32,
) -> SetAttributeModifier {
	let speed = seeded_range(writer, seed, 2.53, speed_min, speed_max);
	let vel = writer.lit(Vec3::new(0.0, -1.0, 0.0)) * speed * scale.clone() * playback.clone();
	SetAttributeModifier::new(Attribute::VELOCITY, vel.expr())
}

fn init_size3(
	writer: &ExprWriter,
	scale: &bevy_hanabi::prelude::WriterExpr,
	size: Vec3,
) -> SetAttributeModifier {
	SetAttributeModifier::new(Attribute::SIZE3, (writer.lit(size) * scale.clone()).expr())
}

/// Tinted additive cards on the travelling ring, drifting behind the front.
pub fn compile_wisps(effects: &mut Assets<EffectAsset>, spark: Handle<Image>) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionCircleModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		axis: writer.lit(Vec3::Y).expr(),
		radius: writer.lit(1.0).expr(),
		dimension: ShapeDimension::Surface,
	};
	let init_vel =
		init_along_negative_y(&writer, &writer.lit(1.0), &props.playback, &props.seed, 0.35, 0.95);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, WISP_LIFE_MIN, WISP_LIFE_MAX);
	let init_color = init_palette_color(&props);
	let init_size = init_size3(&writer, &props.scale, WISP_SIZE_START);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(0.45) * props.playback.clone()).expr());
	let update_accel = AccelModifier::new(
		(writer.lit(Vec3::new(0.0, -0.35, 0.0)) * props.scale.clone() * props.playback.clone())
			.expr(),
	);
	let update_size = update_scaled_size3(&writer, &props.scale, WISP_SIZE_START, WISP_SIZE_END);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&WISP_ALPHA);

	let mut module = writer.finish();
	module.add_texture_slot("energy", bevy_hanabi::SlotDimension::D2);

	let effect = effects.add(
		EffectAsset::new(WISP_CAPACITY, SpawnerSettings::rate(WISP_RATE.into()), module)
			.with_name("vfx-soundwave-wisps")
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
		name: SOUNDWAVE_WISP_NAME.into(),
		effect,
		images: vec![spark],
		count: WISP_COUNT,
		capacity: WISP_CAPACITY,
		max_lifetime: WISP_LIFE_MAX,
		shade: ParticleShade::Spark,
	}
}

/// Along-velocity spirit streaks trailing the live front.
pub fn compile_streaks(effects: &mut Assets<EffectAsset>, spark: Handle<Image>) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionCircleModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		axis: writer.lit(Vec3::Y).expr(),
		radius: writer.lit(1.0).expr(),
		dimension: ShapeDimension::Surface,
	};
	let init_vel =
		init_along_negative_y(&writer, &writer.lit(1.0), &props.playback, &props.seed, 1.8, 4.6);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime =
		init_lifetime(&writer, &props.playback, &props.seed, STREAK_LIFE_MIN, STREAK_LIFE_MAX);
	let init_color = init_palette_color(&props);
	let init_size = init_size3(&writer, &props.scale, STREAK_SIZE_START);
	let update_color = update_palette_color(&writer, &props);
	let update_drag = LinearDragModifier::new((writer.lit(0.85) * props.playback.clone()).expr());
	let update_size =
		update_scaled_size3(&writer, &props.scale, STREAK_SIZE_START, STREAK_SIZE_END);
	let texture_slot = 0;
	let color = value_lifetime_gradient(&STREAK_ALPHA);

	let mut module = writer.finish();
	module.add_texture_slot("spark", bevy_hanabi::SlotDimension::D2);

	let effect = effects.add(
		EffectAsset::new(STREAK_CAPACITY, SpawnerSettings::rate(STREAK_RATE.into()), module)
			.with_name("vfx-soundwave-streaks")
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
		name: SOUNDWAVE_STREAK_NAME.into(),
		effect,
		images: vec![spark],
		count: STREAK_COUNT,
		capacity: STREAK_CAPACITY,
		max_lifetime: STREAK_LIFE_MAX,
		shade: ParticleShade::Spark,
	}
}

/// Pale cyan / gold like the Bray mesh tint, not the fire spark palette.
pub fn trail_properties(fade: f32) -> EffectProperties {
	let mut properties = EffectProperties::default();
	properties.set(PROP_SCALE, 1.0.into());
	properties.set(PROP_TINT, Vec4::new(0.72, 0.88, 1.0, fade.clamp(0.0, 1.0)).into());
	properties.set(PROP_SEED, 0.17.into());
	properties.set(PROP_PLAYBACK, 1.0.into());
	properties.set(PROP_COLOR0, Vec4::new(0.96, 0.98, 1.05, 1.0).into());
	properties.set(PROP_COLOR1, Vec4::new(0.45, 0.78, 0.95, 1.0).into());
	properties.set(PROP_COLOR2, Vec4::new(0.95, 0.92, 0.75, 1.0).into());
	properties
}

pub fn fade_trail(properties: &mut EffectProperties, fade: f32) {
	properties.set(PROP_TINT, Vec4::new(0.72, 0.88, 1.0, fade.clamp(0.0, 1.0)).into());
}

fn spawn_trail_layer(
	commands: &mut Commands,
	parent: Entity,
	name: &'static str,
	part: Option<&ParticlePart>,
) -> Entity {
	let mut entity = commands.spawn((
		Name::new(name),
		ChildOf(parent),
		Transform::IDENTITY,
		Visibility::Inherited,
	));
	if let Some(part) = part {
		entity.insert((ParticleEffect::new(part.effect.clone()), trail_properties(1.0)));
		if !part.images.is_empty() {
			entity.insert(EffectMaterial { images: part.images.clone() });
		}
	}
	entity.id()
}

/// Named trailing wisp and streak emitters on the wake. Particle handles are
/// optional so headless consumer tests still pin emission presence.
pub fn attach_soundwave_trail(
	commands: &mut Commands,
	parent: Entity,
	wisps: Option<&ParticlePart>,
	streaks: Option<&ParticlePart>,
) -> (Entity, Entity) {
	let wisps = spawn_trail_layer(commands, parent, SOUNDWAVE_WISP_NAME, wisps);
	let streaks = spawn_trail_layer(commands, parent, SOUNDWAVE_STREAK_NAME, streaks);
	(wisps, streaks)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::composition::EffectPart;

	#[test]
	fn layer_owns_the_soundwave_part() {
		let part = compile_mesh(Handle::default());
		let def = definition(part);
		assert_eq!(def.name, SOUNDWAVE);
		assert!(matches!(def.layers[0].part, EffectPart::Soundwave(_)));
	}

	#[test]
	fn recipe_faces_a_horizontal_heading() {
		let axle = facing_player_transform().rotation * Vec3::Y;
		assert!(
			(axle - Vec3::NEG_Z).length() < 1e-4,
			"playground recipe must face the camera like Bray, got {axle}"
		);
	}

	#[test]
	fn wisps_emit_on_the_ring_and_trail_behind_the_front() {
		let mut effects = Assets::<EffectAsset>::default();
		let part = compile_wisps(&mut effects, Handle::default());
		assert_eq!(part.name, SOUNDWAVE_WISP_NAME);
		assert_eq!(part.images.len(), 1);
		assert_eq!(part.capacity, WISP_CAPACITY);
		let asset = effects.get(&part.effect).expect("compiled wisp asset");
		assert!(
			asset.spawner.is_forever(),
			"wisps must keep emitting while the wave lives, not a one-shot burst"
		);
		assert!(!asset.spawner.is_once());
	}

	#[test]
	fn streaks_are_along_velocity_and_keep_emitting() {
		let mut effects = Assets::<EffectAsset>::default();
		let part = compile_streaks(&mut effects, Handle::default());
		assert_eq!(part.name, SOUNDWAVE_STREAK_NAME);
		let asset = effects.get(&part.effect).expect("compiled streak asset");
		assert!(asset.spawner.is_forever());
		assert!(!asset.spawner.is_once());
	}

	#[test]
	fn trail_recipe_keeps_the_mesh_and_particle_layers() {
		let mut effects = Assets::<EffectAsset>::default();
		let wisps = compile_wisps(&mut effects, Handle::default());
		let streaks = compile_streaks(&mut effects, Handle::default());
		let def = definition_with_trail(compile_mesh(Handle::default()), wisps, streaks);
		assert_eq!(def.layers.len(), 3);
		assert!(matches!(def.layers[0].part, EffectPart::Soundwave(_)));
		assert!(matches!(def.layers[1].part, EffectPart::Particle(_)));
		assert!(matches!(def.layers[2].part, EffectPart::Particle(_)));
	}
}
