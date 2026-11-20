//! Shared large-body interaction; no emission, secondary particles or optical resources.
use crate::parts::{
	cartoon_wave::motion::{Deformation, Pose},
	combined::{
		flow::Flow,
		interaction::{InteractionScene, Wake, DT, MAX_WAKES},
	},
};
use bevy::{
	math::{DQuat, DVec3},
	prelude::*,
};
use std::collections::VecDeque;
#[derive(Clone, Debug)]
pub struct BodyMotion {
	pub flow_guided: bool,
	pub affine: Mat3,
	pub previous_model: Mat4,
	pub id: u32,
	pub position: DVec3,
	pub velocity: DVec3,
	pub rotation: DQuat,
	pub scale: DVec3,
	pub age: f64,
	pub phase: f64,
	pub birth: f64,
	pub pose: Pose,
	pub previous: Pose,
	pub concentration: f32,
}
impl BodyMotion {
	/// Artistic axial transport rejection near recent capsule motion. The wind-relative
	/// component is a heuristic, not a decomposition of pressure/obstacle contributions.
	/// The original sampled gradient and the shared field remain untouched.
	pub(crate) fn body_transport(
		scene: &InteractionScene,
		history: &VecDeque<Wake>,
		t: f64,
		p: DVec3,
		mut target: DVec3,
		mut velocity: DVec3,
	) -> (DVec3, DVec3) {
		if scene.clump_carry >= 1. {
			return (target, velocity);
		}
		let mut direction = DVec3::ZERO;
		let mut weight = 0.0f64;
		for w in history {
			let mut c = scene.capsule(t);
			c.position = w.position;
			let spatial = (1. - c.query(p).distance.max(0.) / 0.8).clamp(0., 1.);
			let fade = (1. - (t - w.birth) / 1.6).clamp(0., 1.);
			let influence = spatial * spatial * (3. - 2. * spatial) * fade * fade;
			direction += w.velocity.normalize_or_zero() * influence;
			weight = weight.max(influence);
		}
		let n = direction.normalize_or_zero();
		let reject = (1. - scene.clump_carry) * weight;
		target -= n * ((target - scene.wind).dot(n).max(0.) * reject);
		// Locally shorten the old one-second axial coasting tail without hard braking.
		velocity -= n * ((velocity - scene.wind).dot(n).max(0.) * (1. - (-6. * reject * DT).exp()));
		(target, velocity)
	}
	pub(crate) fn advect(
		&mut self,
		f: &Flow,
		scene: &InteractionScene,
		history: &VecDeque<Wake>,
		t: f64,
	) {
		let centre = Vec3::new(0.05, 0.66, 0.);
		let offsets = [Vec3::X, Vec3::Y, Vec3::Z];
		let mut average = DVec3::ZERO;
		let mut directions = [Vec3::ZERO; 3];
		let mut responses = [Vec3::ZERO; 3];
		for a in 0..3 {
			let lo = self
				.model()
				.transform_point3(self.pose.coefficients.map(centre - offsets[a] * 0.45));
			let hi = self
				.model()
				.transform_point3(self.pose.coefficients.map(centre + offsets[a] * 0.45));
			let u = f.sample(lo.as_dvec3());
			let v = f.sample(hi.as_dvec3());
			average += u + v;
			directions[a] = hi - lo;
			responses[a] = (v - u).as_vec3();
		}
		let basis = Mat3::from_cols(directions[0], directions[1], directions[2]);
		if basis.determinant().abs() > 1e-6 {
			let gradient =
				Mat3::from_cols(responses[0], responses[1], responses[2]) * basis.inverse();
			let next = self.affine
				+ (gradient * self.affine - (self.affine - Mat3::IDENTITY) * 0.3)
					* (DT as f32 * scene.flow_response);
			let delta = next - Mat3::IDENTITY;
			let norm = delta.to_cols_array().iter().map(|x| x * x).sum::<f32>().sqrt();
			self.affine =
				Mat3::IDENTITY + delta * (scene.flow_limit / norm.max(scene.flow_limit).max(1e-8));
		}
		let target = average / 6.;
		let (target, velocity) =
			BodyMotion::body_transport(scene, history, t, self.position, target, self.velocity);
		self.velocity = target + (velocity - target) * (-DT).exp();
		self.position += self.velocity * DT;
	}
	/// Restrained yaw from the approved carrier controller; affine rolling stays separate.
	pub(crate) fn turn(&mut self) {
		let desired = DQuat::from_rotation_y(-self.velocity.z.atan2(self.velocity.x) * 0.25);
		self.rotation = self.rotation.slerp(desired, 1. - (-0.35 * DT).exp()).normalize();
	}

	pub(crate) fn remember(history: &mut VecDeque<Wake>, scene: &InteractionScene, t: f64) {
		history.retain(|w| t - w.birth < 1.6);
		if scene.collider_count() == 0 {
			history.clear();
			return;
		}
		let c = scene.capsule(t);
		if c.linear.length_squared() > 0.01
			&& history.back().is_none_or(|w| t - w.birth >= 0.08)
			&& history.len() < MAX_WAKES
		{
			history.push_back(Wake { position: c.position, velocity: c.linear, birth: t });
		}
	}

	pub(crate) fn update_pose(&mut self) {
		self.pose.phase = self.phase as f32;
		self.pose.coefficients = Deformation::at(self.phase as f32, self.pose.clip);
		self.pose.rotation = self.rotation.as_quat();
		self.pose.scale = self.scale.as_vec3();
		self.pose.position = self.position.as_vec3()
			- self.pose.rotation
				* (self.pose.coefficients.map(Vec3::new(0.05, 0.66, 0.)) * self.pose.scale);
	}
	pub fn model(&self) -> Mat4 {
		let c = self.position.as_vec3();
		Mat4::from_translation(c)
			* Mat4::from_mat3(self.affine)
			* Mat4::from_translation(-c)
			* self.pose.transform().to_matrix()
	}
	pub fn material_point(&self, rest: Vec3) -> (DVec3, DVec3) {
		if !self.flow_guided {
			let p = self.pose.world(rest).as_dvec3();
			return (p, (p - self.previous.world(rest).as_dvec3()) / DT);
		}
		let p = self.model().transform_point3(self.pose.coefficients.map(rest)).as_dvec3();
		let old = self
			.previous_model
			.transform_point3(self.previous.coefficients.map(rest))
			.as_dvec3();
		(p, (p - old) / DT)
	}
}
