//! Incremental 120 Hz authored local-coordinate integration; backwards seeks replay.
use crate::parts::smoke_explosion::density::{smooth, GrainBirth};
use bevy::prelude::*;
#[derive(Clone, Debug)]
pub struct Grain {
	pub position: Vec3,
	pub velocity: Vec3,
	pub contact: Option<f32>,
	pub radius: f32,
}
#[derive(Default)]
pub struct GrainSimulation {
	pub time: f32,
	pub states: Vec<Grain>,
	pub steps: u64,
}
impl GrainSimulation {
	pub fn advance(&mut self, t: f32, births: &[GrainBirth]) {
		if self.states.len() != births.len() || t + 1e-6 < self.time {
			self.time = 0.;
			self.steps = 0;
			self.states = births
				.iter()
				.map(|g| Grain {
					position: Vec3::from_array(g.world_origin),
					velocity: Vec3::from_array(g.initial_velocity),
					contact: None,
					radius: 0.,
				})
				.collect();
		}
		let target = (t * 120. + 1e-5).floor() as u64;
		while self.steps < target {
			let end = (self.steps + 1) as f32 / 120.;
			let begin = self.steps as f32 / 120.;
			for (b, s) in births.iter().zip(&mut self.states) {
				if end < b.birth || begin >= 3.95 {
					continue;
				}
				let dt = end - begin.max(b.birth);
				if dt <= 0. {
					continue;
				}
				let v0 = s.velocity;
				let terminal = Vec3::new(0., -9.81 / b.drag, 0.);
				s.velocity = terminal + (v0 - terminal) * (-b.drag * dt).exp();
				s.position += (v0 + s.velocity) * (0.5 * dt);
				if s.position.y < b.radius {
					s.position.y = b.radius;
					s.velocity.y = (-s.velocity.y * 0.15).max(0.);
					s.velocity.x *= 0.6;
					s.velocity.z *= 0.6;
					s.contact.get_or_insert(end);
				}
				let start = s.contact.unwrap_or(2.).min(2.);
				let duration = b.shrink_t.min(3.95 - start);
				s.radius = b.radius * (1. - smooth((end - start) / duration));
			}
			self.steps += 1;
		}
		self.time = t;
	}
}
