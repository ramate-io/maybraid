//! Local closed-lobe response. No positional collision projection or authored tether.
use crate::parts::{
	body_flow::BodyMotion,
	combined::interaction::{InteractionScene, DT},
};
use bevy::{math::DVec3, prelude::*};
pub(crate) fn respond(
	body: &mut BodyMotion,
	side_memory: &mut f64,
	scene: &InteractionScene,
	t: f64,
) {
	if scene.collider_count() == 0 {
		return;
	}
	let capsule = scene.capsule(t);
	let q = capsule.query(body.position);
	let mut predicted = capsule;
	predicted.position += capsule.linear.clamp_length_max(3.) * 0.15;
	let proximity = q.distance.min(predicted.query(body.position).distance);
	let radius = body.scale.max_element() * 0.62;
	let influence =
		(1. - ((proximity - radius) / 0.30).clamp(0., 1.)).powi(2) * scene.contact_response;
	if influence == 0. {
		return;
	}
	// Persistent spatial side choice; adjacent released lobes use the same flow,
	// while those on opposite sides can roll independently. No random kicks.
	let axis = if scene.external.is_some() {
		let v = capsule.linear.normalize_or(DVec3::Z);
		DVec3::new(v.z, 0., -v.x).normalize_or(DVec3::X)
	} else {
		DVec3::X
	};
	if *side_memory == 0. {
		*side_memory = if (body.position - capsule.position).dot(axis) < 0. { -1. } else { 1. };
	}
	let side = *side_memory;

	let speed = capsule.linear.length().min(3.);
	let blend = 1. - (-5. * influence * DT).exp();
	let mobility = if body.id % 3 == 0 { 0.15 } else { 1. };
	if scene.external.is_some() {
		let along = body.velocity.dot(axis);
		body.velocity += axis * (side * (0.25 + speed * 0.275) - along) * blend * mobility;
	} else {
		body.velocity.x += (side * (0.25 + speed * 0.275) - body.velocity.x) * blend * mobility;
	}
	body.velocity.y += (0.08 - body.velocity.y) * blend * 0.25;
	let normal = q.normal.as_vec3();
	let compression = Mat3::IDENTITY
		- Mat3::from_cols(normal * normal.x, normal * normal.y, normal * normal.z) * 0.22;
	body.affine += (compression - body.affine) * ((influence * DT * 4.) as f32);
	let d = body.affine - Mat3::IDENTITY;
	let norm = d.to_cols_array().iter().map(|v| v * v).sum::<f32>().sqrt();
	body.affine = Mat3::IDENTITY + d * (scene.flow_limit / norm.max(scene.flow_limit).max(1e-8));
	let yaw = bevy::math::DQuat::from_rotation_z(-side * influence * 0.3 * DT);
	body.rotation = (yaw * body.rotation).normalize();
}
