//! Shared Hanabi instance properties and smoke launch math.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	Attribute, EffectProperties, ExprWriter, SetAttributeModifier, WriterExpr,
};

use crate::spawn::VfxSpawn;

pub const PROP_SCALE: &str = "vfx_scale";
pub const PROP_TINT: &str = "vfx_tint";
pub const PROP_SEED: &str = "vfx_seed";
pub const PROP_PLAYBACK: &str = "vfx_playback";

pub struct InstanceExprs {
	pub scale: WriterExpr,
	pub tint: WriterExpr,
	pub seed: WriterExpr,
	pub playback: WriterExpr,
}

pub fn add_instance_properties(writer: &ExprWriter) -> InstanceExprs {
	let scale = writer.add_property(PROP_SCALE, 1.0.into());
	let tint = writer.add_property(PROP_TINT, Vec4::ONE.into());
	let seed = writer.add_property(PROP_SEED, 0.0.into());
	let playback = writer.add_property(PROP_PLAYBACK, 1.0.into());
	InstanceExprs {
		scale: writer.prop(scale),
		tint: writer.prop(tint),
		seed: writer.prop(seed),
		playback: writer.prop(playback),
	}
}

pub fn effect_properties(spawn: &VfxSpawn, layer_scale: f32) -> EffectProperties {
	let mut properties = EffectProperties::default();
	properties.set(PROP_SCALE, (spawn.clamped_scale() * layer_scale).into());
	let tint = spawn.tint.map(LinearRgba::from).unwrap_or(LinearRgba::WHITE);
	properties.set(PROP_TINT, Vec4::new(tint.red, tint.green, tint.blue, 1.0).into());
	properties.set(PROP_SEED, seed_as_f32(spawn.seed).into());
	properties.set(PROP_PLAYBACK, spawn.clamped_playback().into());
	properties
}

pub fn seed_as_f32(seed: u64) -> f32 {
	(seed as f32) * 0.017 + 0.13
}

/// `radial_direction * outward_speed + up * rise_speed`, optional ground clamp.
pub fn smoke_launch_velocity(
	position: Vec3,
	outward_speed: f32,
	rise_speed: f32,
	ground: bool,
) -> Vec3 {
	let dir = if position.length_squared() > 1e-8 { position.normalize() } else { Vec3::Y };
	let mut velocity = dir * outward_speed + Vec3::Y * rise_speed;
	if ground {
		velocity.y = velocity.y.max(0.2);
	}
	velocity
}

/// GPU init: radial from the current (local) position plus an upward bias.
///
/// Hanabi prints swizzles without wrapping the product, so `(vel * scale).y`
/// becomes `vel * scale.y` and fails because `vfx_scale` is a float. Keep
/// ground lift in the rise term instead of clamping a swizzled Y.
pub fn init_smoke_velocity(
	writer: &ExprWriter,
	scale: &WriterExpr,
	playback: &WriterExpr,
	outward_min: f32,
	outward_max: f32,
	rise: f32,
	ground: bool,
) -> SetAttributeModifier {
	let pos = writer.attr(Attribute::POSITION);
	let dir = pos.clone().div(pos.length().max(writer.lit(1e-3)));
	let outward = writer.lit(outward_min).uniform(writer.lit(outward_max));
	let lift = if ground { rise.max(outward_max + 0.2) } else { rise };
	let vel =
		(dir * outward + writer.lit(Vec3::Y) * writer.lit(lift)) * scale.clone() * playback.clone();
	SetAttributeModifier::new(Attribute::VELOCITY, vel.expr())
}

pub fn init_radial_velocity(
	writer: &ExprWriter,
	scale: &WriterExpr,
	playback: &WriterExpr,
	speed_min: f32,
	speed_max: f32,
) -> SetAttributeModifier {
	let pos = writer.attr(Attribute::POSITION);
	let dir = pos.clone().div(pos.length().max(writer.lit(1e-3)));
	let speed = writer.lit(speed_min).uniform(writer.lit(speed_max));
	let vel = dir * speed * scale.clone() * playback.clone();
	SetAttributeModifier::new(Attribute::VELOCITY, vel.expr())
}

pub fn init_lifetime(
	writer: &ExprWriter,
	playback: &WriterExpr,
	min: f32,
	max: f32,
) -> SetAttributeModifier {
	let life = writer.lit(min).uniform(writer.lit(max)) / playback.clone();
	SetAttributeModifier::new(Attribute::LIFETIME, life.expr())
}

pub fn update_scaled_size3(
	writer: &ExprWriter,
	scale: &WriterExpr,
	start: Vec3,
	end: Vec3,
) -> SetAttributeModifier {
	let t = writer
		.attr(Attribute::AGE)
		.div(writer.attr(Attribute::LIFETIME).max(writer.lit(1e-3)))
		.saturate();
	let size = writer.lit(start).mix(writer.lit(end), t) * scale.clone();
	SetAttributeModifier::new(Attribute::SIZE3, size.expr())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn smoke_launch_is_outward_and_up() {
		for position in [
			Vec3::new(0.1, 0.05, 0.0),
			Vec3::new(-0.08, -0.12, 0.06),
			Vec3::new(0.0, -0.14, 0.0),
			Vec3::ZERO,
		] {
			let velocity = smoke_launch_velocity(position, 0.6, 1.1, true);
			assert!(velocity.y > 0.0, "downward launch {velocity:?} from {position:?}");
		}
	}

	#[test]
	fn air_blast_keeps_downward_component() {
		let velocity = smoke_launch_velocity(Vec3::new(0.0, -0.14, 0.0), 0.8, 0.1, false);
		assert!(velocity.y < 0.0);
	}
}
