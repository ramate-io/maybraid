//! Bounded regional formation from measured collision snapshots, never actor paths.
use crate::parts::{
	combined::{
		interaction::{Interactor, Shape, DT},
		simulation::mix32,
	},
	herd::emission::{Birth, EmitterInput},
};
use bevy::math::{DVec3, Mat4};
#[derive(Clone, Copy, Debug)]
/// World-space support extents. Collider snapshots already include collision scale.
pub struct BodyExtent {
	pub height: f64,
	pub footprint: f64,
}
impl BodyExtent {
	pub fn transformed_capsule(
		radius: f64,
		half_segment: f64,
		world: Mat4,
		normal: DVec3,
		shoulder: Option<f64>,
	) -> Self {
		let n = normal.normalize_or(DVec3::Y);
		let a = world.x_axis.truncate().as_dvec3();
		let b = world.y_axis.truncate().as_dvec3();
		let c = world.z_axis.truncate().as_dvec3();
		let support = DVec3::new(a.dot(n), b.dot(n), c.dot(n)).length() * radius
			+ half_segment * b.dot(n).abs();
		Self {
			height: shoulder.filter(|h| h.is_finite() && *h > 0.).unwrap_or(support * 2.),
			footprint: radius
				* (a.length_squared() + b.length_squared() + c.length_squared()).sqrt()
				+ half_segment * (b - n * b.dot(n)).length(),
		}
	}
	pub fn measured(c: &Interactor, normal: DVec3, shoulder: Option<f64>) -> Self {
		let n = normal.normalize_or(DVec3::Y);
		let (height, footprint) = match c.shape {
			Shape::Capsule { radius, half_segment } => {
				let axis = c.rotation * DVec3::Y;
				(
					2. * (radius + half_segment * axis.dot(n).abs()),
					radius + half_segment * (1. - axis.dot(n).powi(2)).max(0.).sqrt(),
				)
			}
			Shape::Box { half } => {
				let axes = [c.rotation * DVec3::X, c.rotation * DVec3::Y, c.rotation * DVec3::Z];
				(
					2. * (axes[0].dot(n).abs() * half.x
						+ axes[1].dot(n).abs() * half.y
						+ axes[2].dot(n).abs() * half.z),
					half.length(),
				)
			}
		};
		Self { height: shoulder.filter(|h| h.is_finite() && *h > 0.).unwrap_or(height), footprint }
	}
}
#[derive(Default)]
pub struct Group {
	previous: Vec<(u32, DVec3, bool)>,
	direction: DVec3,
	credit: f64,
	sequence: u32,
	pub representative_height: f64,
}
pub struct Formation {
	pub birth: Birth,
	pub size: f64,
	pub life: f64,
}
impl Group {
	pub fn sample(
		&mut self,
		time: f64,
		emitters: &[EmitterInput],
		colliders: &[Interactor],
		heights: &[(u32, f64)],
		seed: u32,
		enabled: bool,
		coverage: f64,
	) -> Vec<Formation> {
		self.sample_with_bounds(time, emitters, colliders, heights, seed, enabled, coverage, true)
	}
	pub fn sample_with_bounds(
		&mut self,
		time: f64,
		emitters: &[EmitterInput],
		colliders: &[Interactor],
		heights: &[(u32, f64)],
		seed: u32,
		enabled: bool,
		coverage: f64,
		bounded: bool,
	) -> Vec<Formation> {
		let mut members = Vec::with_capacity(20);
		let mut velocity = DVec3::ZERO;
		let mut distance = 0.;
		for e in emitters {
			let previous =
				self.previous.iter().find(|(id, _, _)| *id == e.id).map(|(_, p, m)| (*p, *m));
			let n = e.ground.normal.normalize_or(DVec3::Y);
			let horizontal = e.velocity - n * e.velocity.dot(n);
			let Some(c) = colliders.iter().find(|c| c.id == e.id) else {
				continue;
			};
			if e.discontinuous
				|| !e.ground.grounded
				|| horizontal.length() < if previous.is_some_and(|(_, m)| m) { 0.12 } else { 0.22 }
			{
				continue;
			}
			let Some((old, _)) = previous else {
				continue;
			};
			let delta = e.position - old;
			if delta.length() > 8. * DT + 1e-6 {
				continue;
			}
			let extent = BodyExtent::measured(
				c,
				e.ground.normal,
				heights.iter().find(|(id, _)| *id == e.id).map(|(_, h)| *h),
			);
			members.push((*e, extent));
			velocity += horizontal;
			distance += (delta - n * delta.dot(n)).length();
		}
		let previous = std::mem::take(&mut self.previous);
		self.previous = emitters
			.iter()
			.map(|e| {
				let moving =
					previous.iter().find(|(id, _, _)| *id == e.id).is_some_and(|(_, _, m)| *m);
				(
					e.id,
					e.position,
					!e.discontinuous
						&& e.ground.grounded
						&& (e.velocity
							- e.ground.normal.normalize_or(DVec3::Y)
								* e.velocity.dot(e.ground.normal.normalize_or(DVec3::Y)))
						.length() >= if moving { 0.12 } else { 0.22 },
				)
			})
			.collect();
		if members.is_empty() || !enabled {
			self.credit = 0.;
			return Vec::new();
		}
		let target =
			(velocity * DVec3::new(1., 0., 1.)).normalize_or(self.direction.normalize_or(DVec3::Z));
		self.direction = self.direction.lerp(target, 1. - (-3. * DT).exp()).normalize_or(target);
		let n = members[0].0.ground.normal.normalize_or(DVec3::Y);
		let side = n.cross(self.direction).normalize_or(DVec3::X);
		let mut heights = members.iter().map(|(_, b)| b.height).collect::<Vec<_>>();
		heights.sort_by(f64::total_cmp);
		let height = heights[heights.len() / 2];
		self.representative_height = height;
		let centre = members.iter().map(|(e, _)| e.position).sum::<DVec3>() / members.len() as f64;
		let mut projections = members
			.iter()
			.map(|(e, _)| (e.position - centre).dot(self.direction))
			.collect::<Vec<_>>();
		projections.sort_by(f64::total_cmp);
		let rear_cut = projections[(projections.len() - 1) / 2];
		let mut regions = [(DVec3::ZERO, 0.); 4];
		for (e, body) in &members {
			let rel = e.position - centre;
			if rel.dot(self.direction) > rear_cut + body.footprint * 0.25 {
				continue;
			}
			let bin = if rel.dot(side) < 0. { 0 } else { 1 }
				+ if rel.dot(self.direction) < -height * 0.45 { 0 } else { 2 };
			let weight = (e.velocity.length() / height.max(0.1)).clamp(0.2, 2.);
			regions[bin].0 +=
				(e.position - self.direction * (body.footprint + height * 0.22)) * weight;
			regions[bin].1 += weight;
		}
		// Saturating concentration: doubling actors reinforces coverage, never doubles births.
		self.credit += (distance / members.len() as f64) / height.max(0.1)
			* 5. * coverage
			* (1. + (members.len() as f64).sqrt() * 0.12);
		self.credit = self.credit.min(2.);
		let mut out = Vec::with_capacity(3);
		if self.credit < 1. {
			return out;
		}
		self.credit -= 1.;
		let index = self.sequence as usize % 4;
		let mut region = regions[index];
		if region.1 == 0. {
			if let Some(r) = regions.iter().find(|r| r.1 > 0.) {
				region = *r;
			} else {
				return out;
			}
		}
		let key = mix32(seed ^ self.sequence.wrapping_mul(0x9e3779b9));
		self.sequence = self.sequence.wrapping_add(1);
		let p = region.0 / region.1;
		let size = height * 0.48 * coverage.sqrt();
		for (i, factor, offset, life) in [
			(0, 1., DVec3::ZERO, 1.8),
			(1, 0.72, side * height * 0.48 + n * height * 0.12, 1.45),
			(2, 0.48, -side * height * 0.45 - self.direction * height * 0.25, 2.3),
		] {
			if i == 2 && key % 3 != 0 {
				continue;
			}
			let position = p + offset + n * height * 0.025;
			// Keep the entire conservative deformed contour envelope inside the fixed field.
			let margin = size * factor * 3.2 + height * 0.25;
			if bounded
				&& (position.x - margin < -2.
					|| position.x + margin > 4.
					|| position.z - margin < -3.
					|| position.z + margin > 3.)
			{
				continue;
			}
			out.push(Formation {
				birth: Birth {
					time,
					position,
					normal: n,
					velocity: velocity / members.len() as f64 * 0.15,
					seed: mix32(key ^ i),
				},
				size: size * factor,
				life,
			});
		}
		out
	}
}
#[cfg(test)]
mod tests {
	use crate::parts::combined::interaction::{Interactor, Shape};
	use crate::parts::herd::group::*;
	use bevy::math::DVec3;
	#[test]
	fn dimensions_use_rotation_and_explicit_height() {
		let c = Interactor {
			id: 0,
			shape: Shape::Capsule { radius: 0.2, half_segment: 0.4 },
			position: DVec3::ZERO,
			rotation: bevy::math::DQuat::IDENTITY,
			linear: DVec3::ZERO,
			angular: DVec3::ZERO,
			influence: 0.,
			strength: 1.,
			entrainment: 0.,
			slide: 0.,
		};
		let b = BodyExtent::measured(&c, DVec3::Y, None);
		assert!((b.height - 1.2).abs() < 1e-12);
		assert_eq!(BodyExtent::measured(&c, DVec3::Y, Some(0.9)).height, 0.9);
	}
}
#[cfg(test)]
mod formation_tests {
	use crate::parts::combined::interaction::{Interactor, Shape, DT};
	use crate::parts::herd::emission::EmitterInput;
	use crate::parts::herd::emission::Ground;
	use crate::parts::herd::group::*;
	use bevy::math::{DVec3, Mat4};
	#[test]
	fn scale_and_regional_births_are_bounded_and_stop() {
		let transform = Mat4::from_scale(bevy::math::Vec3::splat(2.));
		assert!(
			(BodyExtent::transformed_capsule(0.1, 0.2, transform, DVec3::Y, None).height - 1.2)
				.abs() < 1e-6
		);
		let mut group = Group::default();
		let mut count = 0;
		for tick in 1..=240 {
			let time = tick as f64 * DT;
			let emitters = (0..10)
				.map(|id| EmitterInput {
					id,
					position: DVec3::new(
						(id % 3) as f64 * 0.2,
						0.,
						time * 0.5 + (id / 3) as f64 * 0.15,
					),
					velocity: DVec3::Z * 0.5,
					ground: Ground::default(),
					discontinuous: tick == 1,
				})
				.collect::<Vec<_>>();
			let colliders = emitters
				.iter()
				.map(|e| Interactor {
					id: e.id,
					shape: Shape::Capsule { radius: 0.11, half_segment: 0.19 },
					position: e.position + DVec3::Y * 0.3,
					rotation: bevy::math::DQuat::IDENTITY,
					linear: e.velocity,
					angular: DVec3::ZERO,
					influence: 0.4,
					strength: 1.,
					entrainment: 0.65,
					slide: 0.12,
				})
				.collect::<Vec<_>>();
			let births = group.sample(time, &emitters, &colliders, &[], 13, tick < 200, 1.);
			assert!(births.len() <= 3);
			if tick >= 200 {
				assert!(births.is_empty());
			}
			count += births.len();
			for b in births {
				assert!(b.size < 0.4 && b.life <= 2.3 && b.birth.position.is_finite());
			}
		}
		assert!(count > 5);
		assert!((group.representative_height - 0.6).abs() < 1e-12);
	}
}

#[cfg(test)]
mod input_tests {
	use crate::parts::{
		combined::interaction::{Interactor, Shape, DT},
		herd::{
			emission::{EmitterInput, Ground},
			group::{BodyExtent, Group},
		},
	};
	use bevy::math::{DQuat, DVec3, Mat4, Quat, Vec3};
	#[test]
	fn measured_support_handles_nonuniform_transform_and_ground() {
		let world = Mat4::from_scale_rotation_translation(
			Vec3::new(2., 3., 0.5),
			Quat::from_rotation_z(0.7),
			Vec3::new(3., 2., 1.),
		);
		let n = DVec3::new(0.2, 1., 0.1).normalize();
		let b = BodyExtent::transformed_capsule(0.11, 0.19, world, n, None);
		let mut maximum: f64 = 0.;
		for i in 0..1000 {
			let angle = i as f64 * 2.399963229728653;
			let y = 1. - 2. * (i as f64 + 0.5) / 1000.;
			let r = (1. - y * y).sqrt();
			let d = DVec3::new(r * angle.cos(), y, r * angle.sin());
			for sign in [-1., 1.] {
				let p = world
					.transform_vector3((d * 0.11 + DVec3::Y * 0.19 * sign).as_vec3())
					.as_dvec3();
				maximum = maximum.max(p.dot(n).abs());
			}
		}
		assert!(maximum * 2. <= b.height + 1e-6 && maximum * 2. > b.height * 0.99);
	}
	#[test]
	fn regional_gating_replay_and_robust_height() {
		let mut a = Group::default();
		let mut b = Group::default();
		let mut accepted = 0;
		for tick in 1..=360 {
			let t = tick as f64 * DT;
			let blocked = (80..160).contains(&tick);
			let es = (0..10)
				.map(|id| EmitterInput {
					id,
					position: DVec3::new(
						(id % 3) as f64 * 0.2,
						0.,
						t * 0.5 + (id / 3) as f64 * 0.15,
					),
					velocity: if blocked { DVec3::ZERO } else { DVec3::Z * 0.5 },
					ground: Ground { normal: DVec3::Y, grounded: tick != 180 },
					discontinuous: tick == 1 || tick == 200,
				})
				.collect::<Vec<_>>();
			let cs = es
				.iter()
				.map(|e| Interactor {
					id: e.id,
					shape: Shape::Capsule { radius: 0.11, half_segment: 0.19 },
					position: e.position + DVec3::Y * 0.3,
					rotation: DQuat::IDENTITY,
					linear: e.velocity,
					angular: DVec3::ZERO,
					influence: 0.4,
					strength: 1.,
					entrainment: 0.65,
					slide: 0.12,
				})
				.collect::<Vec<_>>();
			let heights = vec![(9, 6.)];
			let x = a.sample(t, &es, &cs, &heights, 13, true, 1.);
			let y = b.sample(t, &es, &cs, &heights, 13, true, 1.);
			assert_eq!(x.len(), y.len());
			for (x, y) in x.iter().zip(&y) {
				assert_eq!(x.birth.seed, y.birth.seed);
				assert_eq!(x.birth.position, y.birth.position);
				assert_eq!(x.size, y.size);
			}
			if blocked || tick == 1 || tick == 180 || tick == 200 {
				assert!(x.is_empty());
			}
			accepted += x.len();
			assert!(a.representative_height <= 0.600001);
		}
		assert!(accepted > 5);
	}
}
