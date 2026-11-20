//! Distance approximation, deliberately independent of foot animation or actor paths.
use crate::parts::combined::simulation::mix32;
use bevy::math::DVec3;
#[derive(Clone, Copy, Debug)]
pub struct Ground {
	pub normal: DVec3,
	pub grounded: bool,
}
impl Default for Ground {
	fn default() -> Self {
		Self { normal: DVec3::Y, grounded: true }
	}
}
#[derive(Clone, Copy, Debug)]
pub struct EmitterInput {
	pub id: u32,
	pub position: DVec3,
	pub velocity: DVec3,
	pub ground: Ground,
	pub discontinuous: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Birth {
	pub time: f64,
	pub position: DVec3,
	pub normal: DVec3,
	pub velocity: DVec3,
	pub seed: u32,
}
#[derive(Clone, Debug)]
pub struct DistanceEmitter {
	previous: Option<(f64, EmitterInput)>,
	remaining: f64,
	moving: bool,
	sequence: u32,
	seed: u32,
}
impl DistanceEmitter {
	pub fn new(seed: u32, id: u32) -> Self {
		let seed = mix32(seed ^ id.wrapping_mul(0x9e3779b9));
		Self { previous: None, remaining: 0., moving: false, sequence: 0, seed }
	}
	fn random(&self, salt: u32) -> f64 {
		(mix32(self.seed ^ self.sequence.wrapping_mul(0x27d4eb2d) ^ salt) & 0xffffff) as f64
			/ 16777216.
	}
	fn interval(&self, spacing: f64) -> f64 {
		spacing * (0.85 + 0.3 * self.random(1))
	}
	pub fn sample(&mut self, time: f64, input: EmitterInput, spacing: f64, out: &mut Vec<Birth>) {
		let old = self.previous.replace((time, input));
		let spacing = spacing.clamp(0.15, 2.);
		let Some((before, previous)) = old else {
			self.remaining = self.interval(spacing) * (0.2 + 0.8 * self.random(9));
			return;
		};
		let dt = time - before;
		let delta = input.position - previous.position;
		let distance = delta.length();
		// A discontinuity consumes no distance and cannot leave an emission backlog.
		if input.discontinuous
			|| dt <= 0.
			|| dt > 0.1
			|| !distance.is_finite()
			|| distance > 8. * dt + 1e-6
		{
			self.remaining = self.interval(spacing);
			self.moving = false;
			return;
		}
		if delta.x * delta.x + delta.z * delta.z < 1e-20 {
			return;
		}
		let speed =
			(input.velocity.x * input.velocity.x + input.velocity.z * input.velocity.z).sqrt();
		self.moving = if self.moving { speed > 0.12 } else { speed >= 0.22 };
		if !self.moving || !input.ground.grounded || !previous.ground.grounded || distance < 1e-10 {
			return;
		}
		let direction = delta / distance;
		let normal = input.ground.normal.normalize_or(DVec3::Y);
		let side = normal.cross(direction).normalize_or(DVec3::X);
		let mut cursor = 0.;
		while distance - cursor + 1e-10 >= self.remaining {
			cursor += self.remaining;
			let f = (cursor / distance).clamp(0., 1.);
			let seed = mix32(self.seed ^ self.sequence.wrapping_mul(0x27d4eb2d));
			out.push(Birth {
				time: before + dt * f,
				position: previous.position.lerp(input.position, f) - direction * 0.14
					+ side * ((self.random(2) - 0.5) * 0.16)
					+ normal * 0.012,
				normal,
				velocity: input.velocity,
				seed,
			});
			self.sequence = self.sequence.wrapping_add(1);
			self.remaining = self.interval(spacing);
		}
		self.remaining -= distance - cursor;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	fn sample(n: u32) -> Vec<Birth> {
		let mut e = DistanceEmitter::new(13, 2);
		let mut out = Vec::new();
		for i in 0..=n {
			let t = i as f64 / n as f64;
			e.sample(
				t,
				EmitterInput {
					id: 2,
					position: DVec3::new(t, 0., 0.),
					velocity: DVec3::X,
					ground: Ground::default(),
					discontinuous: false,
				},
				0.3,
				&mut out,
			);
		}
		out
	}
	#[test]
	fn distance_partitions_agree() {
		let a = sample(30);
		for n in [60, 120, 240] {
			let b = sample(n);
			assert_eq!(a.len(), b.len());
			for (a, b) in a.iter().zip(b) {
				assert_eq!(a.seed, b.seed);
				assert!(a.position.distance(b.position) < 1e-10);
				assert!((a.time - b.time).abs() < 1e-10);
			}
		}
	}
	#[test]
	fn stop_teleport_airborne_do_not_emit() {
		let mut e = DistanceEmitter::new(13, 0);
		let mut out = Vec::new();
		let mut i = EmitterInput {
			id: 0,
			position: DVec3::ZERO,
			velocity: DVec3::ZERO,
			ground: Ground::default(),
			discontinuous: false,
		};
		e.sample(0., i, 0.2, &mut out);
		i.position.x = 2.;
		i.discontinuous = true;
		e.sample(0.01, i, 0.2, &mut out);
		i.discontinuous = false;
		e.sample(0.02, i, 0.2, &mut out);
		i.ground.grounded = false;
		i.velocity = DVec3::X;
		i.position.x += 0.03;
		e.sample(0.05, i, 0.2, &mut out);
		assert!(out.is_empty());
	}
	#[test]
	fn slope_spacing_world_heading_and_vertical_teleport_credit() {
		let q = bevy::math::DQuat::from_rotation_y(0.73);
		let offset = DVec3::new(21., 4., -15.);
		let mut a = DistanceEmitter::new(13, 7);
		let mut b = a.clone();
		let mut aa = Vec::new();
		let mut bb = Vec::new();
		for tick in 0..=240 {
			let t = tick as f64 / 120.;
			let mut e = EmitterInput {
				id: 7,
				position: DVec3::new(t, t * 0.1, 0.),
				velocity: DVec3::new(1., 0.1, 0.),
				ground: Ground { normal: DVec3::new(-0.1, 1., 0.).normalize(), grounded: true },
				discontinuous: false,
			};
			a.sample(t, e, 0.3, &mut aa);
			e.position = offset + q * e.position;
			e.velocity = q * e.velocity;
			e.ground.normal = q * e.ground.normal;
			b.sample(t, e, 0.3, &mut bb);
		}
		assert_eq!(aa.len(), bb.len());
		assert!(aa.len() > 5);
		for (a, b) in aa.iter().zip(&bb) {
			assert_eq!(a.seed, b.seed);
			assert!((a.time - b.time).abs() < 1e-10);
			assert!((offset + q * a.position - b.position).length() < 1e-10);
		}
		let sequence = a.sequence;
		let mut out = Vec::new();
		a.sample(
			2. + 1. / 120.,
			EmitterInput {
				id: 7,
				position: DVec3::new(2., 12., 0.),
				velocity: DVec3::ZERO,
				ground: Ground::default(),
				discontinuous: true,
			},
			0.3,
			&mut out,
		);
		assert!(out.is_empty());
		assert_eq!(a.sequence, sequence);
		assert_eq!(a.remaining, a.interval(0.3));
		assert!(!a.moving);
	}
}
