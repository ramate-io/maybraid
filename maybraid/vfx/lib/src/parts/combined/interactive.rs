//! Stateful density carriers. Analytic wave trajectories are never evaluated here.
pub use crate::parts::body_flow::BodyMotion as Puff;
use crate::parts::{
	cartoon_wave::motion::{Deformation, Pose},
	combined::{
		interaction::{InteractionScene, Wake, DT, MAX_WAKES},
		simulation::{random, smooth, Particle, Simulation},
	},
};
use bevy::{
	math::{DQuat, DVec3},
	prelude::*,
};
use std::collections::VecDeque;
pub struct Interactive {
	pub puffs: [Option<Puff>; 15],
	pub wakes: VecDeque<Wake>,
	pub carry_motion: VecDeque<Wake>,
	pub flow: Option<crate::parts::combined::flow::Flow>,
	pub continuous: bool,
	pub rejected_bodies: u32,
	pub stop_at: Option<f64>,
	pub serial: u32,
	pub next_birth: f64,
	pub last_wake: f64,
}
impl Interactive {
	pub fn new(continuous: bool) -> Self {
		Self {
			puffs: std::array::from_fn(|_| None),
			wakes: VecDeque::with_capacity(MAX_WAKES),
			carry_motion: VecDeque::with_capacity(MAX_WAKES),
			flow: None,
			continuous,
			rejected_bodies: 0,
			stop_at: None,
			serial: 0,
			next_birth: 0.,
			last_wake: -1.,
		}
	}

	pub fn duration(&self) -> f64 {
		if self.continuous {
			56.
		} else {
			14.
		}
	}
	pub fn pose(&self, id: usize) -> Option<Pose> {
		self.puffs.get(id)?.as_ref().map(|p| p.pose)
	}
	fn emit_body(&mut self, sim: &Simulation, t: f64) {
		let id = self.serial;
		self.serial += 1;
		let Some(slot) = self.puffs.iter_mut().find(|p| p.is_none()) else {
			self.rejected_bodies += 1;
			return;
		};
		let lane = (id % 3) as f64 - 1.;
		let rand = |c| random(sim.config.seed, id, 0, c);
		let position = sim.placement.transform_point3(DVec3::new(-1.25, 0.48, lane * 0.42));
		let scale_factor = sim.placement.x_axis.truncate().length();
		let scale = DVec3::new(0.60, 0.52, 0.58) * scale_factor * 0.5;
		let rotation = sim.placement.to_scale_rotation_translation().1
			* DQuat::from_rotation_y((rand(1) - 0.5) * 0.6);
		let pose = Pose {
			position: Vec3::ZERO,
			scale: scale.as_vec3(),
			rotation: rotation.as_quat(),
			pivot: Vec3::ZERO,
			coefficients: Deformation(Vec4::ZERO),
			variant: id as usize % 3,
			phase: 0.,
			clip: id as usize % 3,
			birth: t as f32,
			yaw: 0.,
			support: 0.,
			tilt: 0.,
		};
		let mut p = Puff {
			flow_guided: self.flow.is_some(),
			affine: Mat3::IDENTITY,
			previous_model: Mat4::IDENTITY,
			id,
			position,
			velocity: sim.placement.transform_vector3(DVec3::new(
				0.8 + rand(2) * 0.2,
				0.10,
				lane * 0.18,
			)),
			rotation,
			scale,
			age: 0.,
			phase: 0.,
			birth: t,
			pose,
			previous: pose,
			concentration: 1.,
		};
		p.update_pose();
		p.previous = p.pose;
		p.previous_model = p.model();
		*slot = Some(p);
	}
	pub fn step(&mut self, sim: &mut Simulation, scene: &InteractionScene) {
		let t = sim.time();
		if let Some(f) = &mut self.flow {
			if sim.tick % 2 == 0 {
				f.step(scene, t);
			}
		}
		for p in &mut self.puffs {
			if p.as_ref().is_some_and(|p| t - p.birth >= 8.) {
				*p = None;
			}
		}
		let end = (if self.continuous { 45.0f64 } else { 1.4 })
			.min(self.stop_at.unwrap_or(f64::INFINITY));
		if t < end && t + 1e-8 >= self.next_birth {
			self.emit_body(sim, t);
			self.next_birth += if self.continuous { 0.55 } else { 0.12 };
		}
		self.wakes.retain(|w| t - w.birth < 1.2);
		if !scene.wake || !scene.enabled {
			self.wakes.clear();
		}
		let capsule = scene.capsule(t);
		if scene.enabled
			&& scene.wake
			&& capsule.linear.length_squared() > 0.01
			&& t - self.last_wake >= 0.08
		{
			if self.wakes.len() < MAX_WAKES {
				self.wakes.push_back(Wake {
					birth: t,
					position: capsule.position,
					velocity: capsule.linear,
				});
			}

			self.last_wake = t;
		}
		if self.flow.is_some() {
			Puff::remember(&mut self.carry_motion, scene, t);
		}

		let factor = sim.placement.x_axis.truncate().length();
		for p in self.puffs.iter_mut().flatten() {
			p.previous = p.pose;
			p.previous_model = p.model();
			p.age = t + DT - p.birth;
			p.phase = (p.phase + DT * 0.52).min(3.);
			if let Some(f) = &self.flow {
				p.advect(f, scene, &self.carry_motion, t);
			} else {
				let flow = scene.flow(p.position, t, &self.wakes, false);
				(p.position, p.velocity) =
					scene.integrate(p.position, p.velocity, flow, 1., 0., 0.22 * factor, t);
			}
			// Bounded yaw response, not rigid spinning to impersonate billowing.
			p.turn();
			p.scale = DVec3::new(0.60, 0.52, 0.58) * factor * (0.5 + 0.5 * smooth(p.age / 0.8));
			p.concentration = (1. - smooth((p.age - 6.5) / 1.5)) as f32;
			p.update_pose();
			// Flat-ground support uses the same conservative bounds as density rendering.
			let b = sim.body.rest[p.pose.variant]
				.iter()
				.map(|r| p.model().transform_point3(p.pose.coefficients.map(*r)).y)
				.fold(f32::INFINITY, f32::min);
			if b < 0.015 {
				p.position.y += (0.015 - b) as f64;
				p.velocity.y = p.velocity.y.max(0.);
				p.update_pose();
			}
		}
		sim.particles.retain(|p| t < p.birth + p.life);
		let mut counts = [
			sim.particles.iter().filter(|p| p.kind == 0).count(),
			sim.particles.iter().filter(|p| p.kind == 1).count(),
		];
		for puff in self.puffs.iter().flatten() {
			if puff.age < 0.15 || puff.age > 5. || self.stop_at.is_some_and(|end| t >= end) {
				continue;
			}
			for kind in 0..2 {
				let rand = |ch| random(sim.config.seed, puff.id, sim.tick, ch + kind * 40);
				if rand(0) > [3.0, 1.5][kind as usize] * DT {
					continue;
				}
				let capacity =
					if kind == 0 { sim.config.max_clouds } else { sim.config.max_grains };
				if counts[kind as usize] >= capacity {
					sim.dropped[kind as usize] += 1;
					continue;
				}
				let a = sim.anchors[puff.pose.variant][(puff.id as usize + sim.tick as usize) % 3];
				let rest = Vec3::from_array(a.point) + Vec3::from_array(a.normal) * 0.12;
				let (position, inherited) = puff.material_point(rest);
				let radius = factor
					* if kind == 0 { 0.10 + 0.09 * rand(5) } else { 0.0021 + 0.0023 * rand(5) };
				sim.particles.push(Particle {
					id: kind * (1 << 30) + puff.id * 100000 + sim.tick,
					parent: puff.id as usize,
					region: puff.id as usize,
					kind,
					variant: (rand(9) * 4.) as usize,
					birth: t + DT,
					life: if kind == 0 { 1. + 0.8 * rand(6) } else { 0.65 + 0.8 * rand(6) },
					position,
					velocity: (inherited
						+ (puff.affine.inverse().transpose()
							* puff.pose.normal(rest, Vec3::from_array(a.normal)))
						.normalize()
						.as_dvec3() * 0.16)
						.clamp_length_max(8.),
					radius,
					rotation: puff.rotation * DQuat::from_rotation_y(rand(7) * 6.283185307179586),
					drag: if kind == 0 { 2.5 } else { 4. },
					gravity: if kind == 0 { 0. } else { 0.25 },
					base_concentration: sim.config.concentration,
					spin: 0.25,
					scale: if kind == 0 {
						DVec3::splat(radius * 0.3)
					} else {
						radius * DVec3::new(1., 0.66, 1.23)
					},
					concentration: if kind == 0 { 0. } else { 1. },
					age: 0.,
					color: DVec3::new(0.65, 0.44, 0.25) * if kind == 0 { 1. } else { 0.5 },
				});
				counts[kind as usize] += 1;
			}
		}
		for p in &mut sim.particles {
			if p.birth > t {
				continue;
			}
			let flow = self.flow.as_ref().map_or_else(
				|| scene.flow(p.position, t, &self.wakes, true),
				|f| f.sample(p.position),
			);
			let extent = if p.kind == 0 { p.scale.max_element() * 0.8 } else { p.radius * 1.23 };
			(p.position, p.velocity) =
				scene.integrate(p.position, p.velocity, flow, p.drag, p.gravity, extent, t);
			p.age = t + DT - p.birth;
			let a = p.age / p.life;
			if p.kind == 0 {
				p.scale = p.radius
					* (0.3 + 0.7 * smooth(p.age / 0.18))
					* (1. + 0.4 * a)
					* DVec3::new(1.12, 0.8, 1.);
				p.concentration = p.base_concentration
					* smooth(p.age / 0.12)
					* (-0.55 * p.age).exp()
					* (1. - smooth((a - 0.65) / 0.35));
				p.rotation = (DQuat::from_rotation_z(-p.spin * DT) * p.rotation).normalize();
			} else {
				p.scale = p.radius * DVec3::new(1., 0.66, 1.23);
				p.concentration = 1.;
			}
		}
		sim.tick += 1;
	}
}

#[cfg(test)]
mod carry_tests {
	use super::*;
	#[test]
	fn axial_carry_preserves_side_flow_wind_and_baseline() -> Result<(), String> {
		let mut scene = InteractionScene { clump_carry: 0.15, ..default() };
		scene.wind = DVec3::new(0.42, 0., 1.);
		let history = VecDeque::from([Wake {
			birth: 0.,
			position: DVec3::new(0.25, 0.7, 0.),
			velocity: DVec3::Z * 2.,
		}]);
		let target = DVec3::new(0.8, 0.3, 2.);
		let velocity = DVec3::new(0.6, 0.5, 3.);
		let (a, b) =
			Puff::body_transport(&scene, &history, 0., DVec3::new(0.25, 0.7, 0.), target, velocity);
		assert_eq!(a.x, target.x);
		assert_eq!(a.y, target.y);
		assert!((a.z - 1.15).abs() < 1e-12);
		assert_eq!(b.x, velocity.x);
		assert_eq!(b.y, velocity.y);
		assert!(b.z < 3. && b.z > 2.8);
		scene.clump_carry = 1.;
		assert_eq!(
			Puff::body_transport(&scene, &history, 0., DVec3::ZERO, target, velocity),
			(target, velocity)
		);
		scene.clump_carry = 0.15;
		assert_eq!(
			Puff::body_transport(&scene, &VecDeque::new(), 0., DVec3::ZERO, target, velocity),
			(target, velocity)
		);
		Ok(())
	}
}
