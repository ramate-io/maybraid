//! World-space Hanabi streak that crosses in front of the listener.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, EffectMaterial, ExprWriter, ImageSampleMapping, LinearDragModifier, OrientMode,
	OrientModifier, ParticleEffect, ParticleTextureModifier, SetAttributeModifier,
	SetPositionSphereModifier, SetVelocitySphereModifier, ShapeDimension, SimulationSpace,
	SizeOverLifetimeModifier, SpawnerSettings,
};
use bevy_hanabi::Gradient;

use crate::wind::WeatherKind;

const PUFF_MASK_SIZE: u32 = 96;
/// Metres in front of the listener so the ribbon sits on the character, not the lens.
const CROSS_DEPTH: f32 = 6.0;
/// Half-width: starts just off-frustum and exits the other side.
const CROSS_HALF: f32 = 8.0;
const CROSS_LIFT: f32 = 0.45;

/// Shared swirl assets. Missing this resource is a silent no-op.
#[derive(Resource, Clone)]
pub struct WindSwirlEffects {
	breeze: Handle<EffectAsset>,
	gust: Handle<EffectAsset>,
	puff: Handle<Image>,
}

/// Emitter that rides a world line baked from the listener pose at spawn.
#[derive(Component, Clone, Copy, Debug)]
pub struct WindSwirl {
	pub age: f32,
	pub life: f32,
	pub origin: Vec3,
	pub forward: Vec3,
	pub right: Vec3,
	pub up: Vec3,
	pub sign: f32,
}

impl WindSwirl {
	pub fn at_listener(listener: &GlobalTransform, life: f32, sign: f32) -> Self {
		Self {
			age: 0.0,
			life: life.max(0.1),
			origin: listener.translation(),
			forward: listener.forward().as_vec3(),
			right: listener.right().as_vec3(),
			up: listener.up().as_vec3(),
			sign: if sign < 0.0 { -1.0 } else { 1.0 },
		}
	}

	pub fn progress(self) -> f32 {
		(self.age / self.life).clamp(0.0, 1.0)
	}

	pub fn pose(self) -> Transform {
		crossing_pose(self.origin, self.forward, self.right, self.up, self.sign, self.progress())
	}
}

/// `t = 0` is off one side of the view; `t = 1` is off the other.
pub fn crossing_pose(
	origin: Vec3,
	forward: Vec3,
	right: Vec3,
	up: Vec3,
	sign: f32,
	t: f32,
) -> Transform {
	let t = t.clamp(0.0, 1.0);
	let ease = t * t * (3.0 - 2.0 * t);
	let lateral = sign * CROSS_HALF;
	let start = origin + forward * CROSS_DEPTH - right * lateral + up * CROSS_LIFT;
	let end = origin + forward * CROSS_DEPTH + right * lateral + up * (CROSS_LIFT - 0.2);
	let travel = end - start;
	let pos = start + travel * ease;
	let dir = travel.normalize_or(right * sign);
	Transform::from_translation(pos).looking_to(dir, up)
}

pub(crate) fn setup_wind_swirls(
	mut commands: Commands,
	mut effects: ResMut<Assets<EffectAsset>>,
	mut images: ResMut<Assets<Image>>,
) {
	let puff = images.add(swirl_mask());
	commands.insert_resource(WindSwirlEffects {
		breeze: effects.add(swirl_effect(WeatherKind::Breeze)),
		gust: effects.add(swirl_effect(WeatherKind::Gust)),
		puff,
	});
}

pub(crate) fn spawn_wind_swirl(
	commands: &mut Commands,
	effects: &WindSwirlEffects,
	listener: &GlobalTransform,
	parent: Entity,
	kind: WeatherKind,
	life: f32,
	sign: f32,
) {
	let swirl = WindSwirl::at_listener(listener, life, sign);
	let asset = match kind {
		WeatherKind::Breeze => effects.breeze.clone(),
		WeatherKind::Gust => effects.gust.clone(),
	};
	commands.spawn((
		Name::new(match kind {
			WeatherKind::Breeze => "weather-breeze-swirl",
			WeatherKind::Gust => "weather-gust-swirl",
		}),
		ChildOf(parent),
		swirl.pose(),
		Visibility::Visible,
		NotShadowCaster,
		ParticleEffect::new(asset),
		EffectMaterial { images: vec![effects.puff.clone()] },
		swirl,
	));
}

pub(crate) fn tick_wind_swirls(
	time: Res<Time>,
	mut swirls: Query<(&mut WindSwirl, &mut Transform)>,
) {
	let dt = time.delta_secs();
	for (mut swirl, mut transform) in &mut swirls {
		swirl.age = (swirl.age + dt).min(swirl.life);
		*transform = swirl.pose();
	}
}

fn swirl_effect(kind: WeatherKind) -> EffectAsset {
	// Size is (length along velocity, thickness). A handful of long slashes
	// reads more like a graphic than a dust cloud.
	let (name, capacity, rate, speed, life, size, drag) = match kind {
		WeatherKind::Breeze => (
			"wind-breeze",
			32_u32,
			4.0,
			(2.8, 4.6),
			(1.2, 2.2),
			(Vec3::new(3.4, 0.22, 1.0), Vec3::new(6.2, 0.38, 1.0), Vec3::new(7.4, 0.1, 1.0)),
			0.35,
		),
		WeatherKind::Gust => (
			"wind-gust",
			48,
			9.0,
			(6.5, 11.0),
			(0.55, 1.1),
			(Vec3::new(4.8, 0.28, 1.0), Vec3::new(8.5, 0.48, 1.0), Vec3::new(10.0, 0.12, 1.0)),
			0.2,
		),
	};
	let writer = ExprWriter::new();
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: writer.lit(1.6).expr(),
		dimension: ShapeDimension::Volume,
	};
	// `looking_to` aims local −Z along travel; a center in +Z sprays that way.
	let speed = writer.lit(speed.0).uniform(writer.lit(speed.1));
	let init_vel = SetVelocitySphereModifier {
		center: writer.lit(Vec3::new(0.0, 0.0, 1.0)).expr(),
		speed: speed.expr(),
	};
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = SetAttributeModifier::new(
		Attribute::LIFETIME,
		writer.lit(life.0).uniform(writer.lit(life.1)).expr(),
	);
	// World-space lift only; directional spray is baked at spawn from the emitter pose.
	let update_accel = AccelModifier::new(writer.lit(Vec3::new(0.0, 0.55, 0.0)).expr());
	let update_drag = LinearDragModifier::new(writer.lit(drag).expr());
	let texture_slot = writer.lit(0u32).expr();

	let mut color = Gradient::new();
	color.add_key(0.0, Vec4::new(0.98, 0.99, 1.0, 0.42));
	color.add_key(0.35, Vec4::new(0.72, 0.8, 0.9, 0.16));
	color.add_key(1.0, Vec4::new(0.45, 0.52, 0.62, 0.0));
	let mut size_gradient = Gradient::new();
	size_gradient.add_key(0.0, size.0);
	size_gradient.add_key(0.4, size.1);
	size_gradient.add_key(1.0, size.2);

	let mut module = writer.finish();
	module.add_texture_slot("puff");

	EffectAsset::new(capacity, SpawnerSettings::rate(rate.into()), module)
		.with_name(name)
		.with_simulation_space(SimulationSpace::Global)
		.with_alpha_mode(bevy_hanabi::AlphaMode::Blend)
		.init(init_pos)
		.init(init_vel)
		.init(init_age)
		.init(init_lifetime)
		.update(update_drag)
		.update(update_accel)
		.render(ParticleTextureModifier {
			texture_slot,
			sample_mapping: ImageSampleMapping::ModulateOpacityFromR,
		})
		.render(ColorOverLifetimeModifier {
			gradient: color,
			blend: ColorBlendMode::Overwrite,
			mask: ColorBlendMask::RGBA,
		})
		.render(SizeOverLifetimeModifier { gradient: size_gradient, screen_space_size: false })
		.render(OrientModifier::new(OrientMode::AlongVelocity))
}

fn swirl_mask() -> Image {
	let n = PUFF_MASK_SIZE;
	let mut data = vec![0u8; (n * n) as usize];
	let c = (n as f32 - 1.0) * 0.5;
	for y in 0..n {
		for x in 0..n {
			let nx = (x as f32 - c) / c;
			// Slight cubic bend so the stroke reads as a slash, not a capsule.
			let bend = nx * nx * nx * 0.22;
			let dx = nx / 0.98;
			let dy = ((y as f32 - c) / c - bend) / 0.11;
			let r = (dx * dx + dy * dy).sqrt();
			let t = (1.0 - r).max(0.0);
			let t = t * t * (3.0 - 2.0 * t);
			data[(y * n + x) as usize] = (t * 235.0) as u8;
		}
	}
	let mut image = Image::new(
		Extent3d { width: n, height: n, depth_or_array_layers: 1 },
		TextureDimension::D2,
		data,
		TextureFormat::R8Unorm,
		RenderAssetUsages::RENDER_WORLD,
	);
	image.sampler = ImageSampler::linear();
	image
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn crossing_starts_off_one_side_and_ends_off_the_other() {
		let origin = Vec3::ZERO;
		let start = crossing_pose(origin, Vec3::NEG_Z, Vec3::X, Vec3::Y, 1.0, 0.0);
		let mid = crossing_pose(origin, Vec3::NEG_Z, Vec3::X, Vec3::Y, 1.0, 0.5);
		let end = crossing_pose(origin, Vec3::NEG_Z, Vec3::X, Vec3::Y, 1.0, 1.0);
		assert!(start.translation.x < -CROSS_HALF + 0.1);
		assert!(end.translation.x > CROSS_HALF - 0.1);
		assert!(start.translation.x < mid.translation.x);
		assert!(mid.translation.x < end.translation.x);
		assert!((start.translation.z + CROSS_DEPTH).abs() < 1e-4);
	}

	#[test]
	fn opposite_sign_flips_the_entry_side() {
		let left = crossing_pose(Vec3::ZERO, Vec3::NEG_Z, Vec3::X, Vec3::Y, -1.0, 0.0);
		let right = crossing_pose(Vec3::ZERO, Vec3::NEG_Z, Vec3::X, Vec3::Y, 1.0, 0.0);
		assert!(left.translation.x > 0.0);
		assert!(right.translation.x < 0.0);
	}

	#[test]
	fn swirl_mask_is_a_long_slash() {
		let image = swirl_mask();
		let Some(data) = image.data.as_ref() else {
			panic!("swirl mask pixels");
		};
		let n = PUFF_MASK_SIZE;
		let at = |x: u32, y: u32| data[(y * n + x) as usize];
		assert!(at(n / 2, n / 2) > 160, "center {}", at(n / 2, n / 2));
		assert_eq!(at(0, 0), 0);
		assert!(
			at(n * 3 / 4, n / 2) > at(n / 2, n * 3 / 4),
			"mask should be longer than it is tall"
		);
	}
}
