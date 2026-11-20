//! Small VFX-facing analytic snapshots. No keyboard, gameplay controller or physics engine.
use bevy::{
	math::{DQuat, DVec3},
	prelude::*,
};
use std::collections::VecDeque;

pub const DT: f64 = 1. / 120.;
pub const MAX_COLLIDERS: usize = 20;
pub const MAX_WAKES: usize = 24;
pub const MAX_INPUTS: usize = 256;
pub const MAX_SPEED: f64 = 8.;
#[derive(Clone, Copy, Debug)]
pub enum Shape {
	Capsule { radius: f64, half_segment: f64 },
	Box { half: DVec3 },
}
#[derive(Clone, Copy, Debug)]
pub struct Interactor {
	pub id: u32,
	pub shape: Shape,
	pub position: DVec3,
	pub rotation: DQuat,
	pub linear: DVec3,
	pub angular: DVec3,
	pub influence: f64,
	pub strength: f64,
	pub entrainment: f64,
	pub slide: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Contact {
	pub distance: f64,
	pub normal: DVec3,
	pub surface: DVec3,
	pub velocity: DVec3,
}
impl Interactor {
	pub fn query(self, p: DVec3) -> Contact {
		let local = self.rotation.conjugate() * (p - self.position);
		let (distance, normal) = match self.shape {
			Shape::Capsule { radius, half_segment } => {
				let q = local - DVec3::new(0., local.y.clamp(-half_segment, half_segment), 0.);
				(
					q.length() - radius,
					if q.length_squared() > 1e-16 { q.normalize() } else { DVec3::X },
				)
			}
			Shape::Box { half } => {
				let q = local.abs() - half;
				let outside = q.max(DVec3::ZERO);
				let d = outside.length() + q.max_element().min(0.);
				let n = if outside.length_squared() > 1e-16 {
					outside.normalize() * local.signum()
				} else {
					let axis = if q.x >= q.y && q.x >= q.z {
						0
					} else if q.y >= q.z {
						1
					} else {
						2
					};
					let mut n = DVec3::ZERO;
					n[axis] = if local[axis] < 0. { -1. } else { 1. };
					n
				};
				(d, n)
			}
		};
		let normal = self.rotation * normal;
		let surface = p - distance * normal;
		Contact {
			distance,
			normal,
			surface,
			velocity: self.linear + self.angular.cross(surface - self.position),
		}
	}
	pub fn remove_inward(self, p: DVec3, velocity: DVec3) -> DVec3 {
		let q = self.query(p);
		velocity - q.normal * (velocity - q.velocity).dot(q.normal).min(0.)
	}
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scenario {
	Grazing,
	Manual,
	Static,
	Stop,
	Fast,
	None,
	#[default]
	Cross,
}
#[derive(Clone, Copy, Debug)]
pub struct InputKey {
	pub time: f64,
	pub position: DVec3,
	pub discontinuous: bool,
}
#[derive(Resource, Clone)]
pub struct InteractionScene {
	/// Optional bounded external snapshot. None retains the approved scripted scene.
	pub external: Option<Vec<Interactor>>,
	pub terrain: Option<std::sync::Arc<crate::parts::herd::terrain::Heightfield>>,
	pub ground_floor: Option<f64>,
	pub compact_landing: bool,
	pub cartoon_wave_timing: bool,
	pub clump_carry: f64,
	pub contact_response: f64,
	pub flow_limit: f32,
	pub flow_response: f32,
	pub version: u64,
	pub enabled: bool,
	pub wake: bool,
	pub scenario: Scenario,
	pub wind: DVec3,
	pub strength: f64,
	pub manual: VecDeque<InputKey>,
}
impl Default for InteractionScene {
	fn default() -> Self {
		Self {
			external: None,
			terrain: None,
			ground_floor: Some(0.),
			compact_landing: false,
			cartoon_wave_timing: false,
			clump_carry: 0.15,
			contact_response: 1.,
			flow_limit: 0.3,
			flow_response: 0.4,
			version: 0,
			enabled: true,
			wake: true,
			scenario: Scenario::Cross,
			wind: DVec3::new(0.42, 0., 0.),
			strength: 1.,
			manual: VecDeque::from([InputKey {
				time: 0.,
				position: DVec3::new(0.25, 0.7, -2.),
				discontinuous: true,
			}]),
		}
	}
}
impl InteractionScene {
	pub fn changed(&mut self) {
		self.version = self.version.wrapping_add(1);
	}
	pub fn place(&mut self, time: f64, position: DVec3, discontinuous: bool) {
		// Equal-time placements replace the latest snapshot, including while paused.
		while self.manual.back().is_some_and(|k| k.time >= time) {
			self.manual.pop_back();
		}
		self.manual.push_back(InputKey { time, position, discontinuous });
		while self.manual.len() > MAX_INPUTS {
			self.manual.pop_front();
		}
		self.changed();
	}
	pub fn rebase_manual(&mut self) {
		if let Some(p) = self.manual.back().map(|k| k.position) {
			self.manual.clear();
			self.place(0., p, true);
		}
	}
	pub fn reset_proxy(&mut self, time: f64) {
		self.place(time, DVec3::new(0.25, 0.7, -2.), true);
	}
	pub fn capsule(&self, t: f64) -> Interactor {
		if let Some(c) = self.external.as_ref().and_then(|v| v.first()) {
			return *c;
		}
		let x = if self.scenario == Scenario::Grazing { 1.05 } else { 0.25 };
		// Timestamped piecewise-linear inputs evaluated at simulation ticks, not render frames.
		let (position, linear) = if self.scenario == Scenario::Manual {
			let mut sample = self.manual.front().copied().unwrap_or(InputKey {
				time: t,
				position: DVec3::ZERO,
				discontinuous: true,
			});
			let mut result = (sample.position, DVec3::ZERO);
			for &next in self.manual.iter().skip(1) {
				if t <= sample.time {
					break;
				}
				if t < next.time {
					if !next.discontinuous && next.time > sample.time {
						let v = (next.position - sample.position) / (next.time - sample.time);
						result = (
							sample.position
								+ (next.position - sample.position)
									* ((t - sample.time) / (next.time - sample.time)).clamp(0., 1.),
							v.clamp_length_max(MAX_SPEED),
						);
					}
					break;
				}
				sample = next;
				result = (sample.position, DVec3::ZERO);
			}
			result
		} else {
			let (begin, end, z_end) = match self.scenario {
				Scenario::Static | Scenario::None => (0., 0., 0.),
				Scenario::Stop => (2.8, 3.8, 0.),
				Scenario::Fast => (3., 3.5, 2.),
				_ => (2.8, 4.8, 2.),
			};
			let (begin, end, z_end) = if self.cartoon_wave_timing {
				match self.scenario {
					Scenario::Static | Scenario::None => (0., 0., 0.),
					Scenario::Stop => (1.3, 2.3, 0.),
					Scenario::Fast => (1.5, 2., 2.),
					_ => (1.3, 3.3, 2.),
				}
			} else {
				(begin, end, z_end)
			};
			let z_start = if self.compact_landing { -0.65 } else { -2. };
			let (begin, end, z_end) = if self.compact_landing {
				match self.scenario {
					Scenario::Static | Scenario::None => (0., 0., 0.),
					Scenario::Stop => (0.05, 0.375, 0.),
					Scenario::Fast => (0.1, 0.2625, 0.65),
					_ => (0.05, 0.7, 0.65),
				}
			} else {
				(begin, end, z_end)
			};
			if end == begin {
				(DVec3::new(x, 0.7, 0.), DVec3::ZERO)
			} else {
				let u = ((t - begin) / (end - begin)).clamp(0., 1.);
				(
					DVec3::new(x, 0.7, z_start + (z_end - z_start) * u),
					if t >= begin && t < end {
						DVec3::Z * ((z_end - z_start) / (end - begin))
					} else {
						DVec3::ZERO
					},
				)
			}
		};
		Interactor {
			id: 1,
			shape: Shape::Capsule { radius: 0.28, half_segment: 0.40 },
			position,
			rotation: DQuat::IDENTITY,
			linear,
			angular: DVec3::ZERO,
			influence: 0.42,
			strength: self.strength,
			entrainment: 0.65,
			slide: 0.12,
		}
	}
	pub fn collider_count(&self) -> usize {
		if let Some(v) = &self.external {
			return if self.enabled { v.len().min(MAX_COLLIDERS) } else { 0 };
		}
		if !self.enabled || self.scenario == Scenario::None {
			0
		} else {
			3
		}
	}
	pub fn colliders(&self, t: f64) -> [Interactor; MAX_COLLIDERS] {
		let mut result = [self.capsule(t); MAX_COLLIDERS];
		if let Some(v) = &self.external {
			for (to, from) in result.iter_mut().zip(v) {
				*to = *from;
			}
		} else {
			result[1] = self.boxes()[0];
			result[2] = self.boxes()[1];
		}
		result
	}
	pub fn boxes(&self) -> [Interactor; 2] {
		[
			Interactor {
				id: 2,
				shape: Shape::Box { half: DVec3::new(0.32, 0.4, 0.4) },
				position: DVec3::new(1.25, 0.4, 0.65),
				rotation: DQuat::IDENTITY,
				linear: DVec3::ZERO,
				angular: DVec3::ZERO,
				influence: 0.24,
				strength: self.strength,
				entrainment: 0.,
				slide: 0.16,
			},
			Interactor {
				id: 3,
				shape: Shape::Box { half: DVec3::new(0.04, 0.65, 0.75) },
				position: DVec3::new(1.8, 0.65, -0.75),
				rotation: DQuat::IDENTITY,
				linear: DVec3::ZERO,
				angular: DVec3::ZERO,
				influence: 0.24,
				strength: self.strength,
				entrainment: 0.,
				slide: 0.16,
			},
		]
	}
	pub fn flow(&self, p: DVec3, t: f64, wakes: &VecDeque<Wake>, small: bool) -> DVec3 {
		let mut u = self.wind;
		// Low-amplitude coherent rolling, not a solved incompressible field.
		let d = p - DVec3::new(0.4, 0.6, 0.);
		u += DVec3::new(-d.y, d.x, 0.) * 0.16 * (-d.length_squared() / 4.).exp();
		for c in self.colliders(t).into_iter().take(self.collider_count()) {
			let q = c.query(p);
			let w = ((1. - q.distance.max(0.) / c.influence).clamp(0., 1.).powi(2) * c.strength)
				.min(1.);
			let relative = u - q.velocity;
			u -= q.normal * relative.dot(q.normal).min(0.) * w;
			u += q.velocity * (c.entrainment * w);
			if relative.dot(q.normal) < 0. {
				u += DVec3::Z * (if p.z < c.position.z { -c.slide } else { c.slide }) * w;
			}
		}
		if self.enabled && self.wake {
			for w in wakes {
				let d = p - w.position;
				let weight = (1. - d.length_squared() / 0.64).max(0.).powi(2)
					* (1. - (t - w.birth) / 1.2).clamp(0., 1.);
				u += (w.velocity * 0.18 + DVec3::Y * 0.12) * weight * if small { 1. } else { 0.25 };
			}
		}
		u.clamp_length_max(MAX_SPEED)
	}
	/// Eight justified microsteps: relative speed <=16 m/s means <=16.7 mm
	/// per sample, below the test wall's80 mm thickness. No final-point-only test.
	pub fn integrate(
		&self,
		mut p: DVec3,
		mut v: DVec3,
		target: DVec3,
		drag: f64,
		gravity: f64,
		extent: f64,
		t: f64,
	) -> (DVec3, DVec3) {
		v = v.clamp_length_max(MAX_SPEED);
		let h = DT / 8.;
		for step in 0..8 {
			let next = target + (v - target) * (-drag * h).exp() - DVec3::Y * gravity * h;
			p += (v + next) * (0.5 * h);
			v = next.clamp_length_max(MAX_SPEED);
			for _ in 0..3 {
				let mut penetrating = false;
				for c in self
					.colliders(t + (step + 1) as f64 * h)
					.into_iter()
					.take(self.collider_count().min(MAX_COLLIDERS))
				{
					let q = c.query(p);
					if q.distance < extent {
						p += q.normal * (extent - q.distance + 1e-5);
						v = c.remove_inward(p, v);
						penetrating = true;
					}
				}
				if !penetrating {
					break;
				}
			}
			v = v.clamp_length_max(MAX_SPEED);
			if self.ground_floor.is_some_and(|floor| p.y < floor + extent) {
				p.y = self.ground_floor.unwrap_or(0.) + extent;
				v.y = v.y.max(0.);
			}
		}
		(p, v)
	}
}
#[derive(Clone, Copy, Debug)]
pub struct Wake {
	pub birth: f64,
	pub position: DVec3,
	pub velocity: DVec3,
}
#[cfg(test)]
mod tests;
