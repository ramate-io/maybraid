//! Fixed contact frames and bounded, effect-clock world inputs. No actor choreography.
use crate::parts::combined::interaction::{
	InteractionScene, Interactor, Shape, MAX_COLLIDERS, MAX_INPUTS,
};
use bevy::{math::DVec3, prelude::*};
use std::{collections::VecDeque, sync::Arc};

/// Shared admission budget; changing it never evicts admitted effects.
#[derive(Resource, Clone)]
pub struct ContactRegionBudget {
	pub maximum: usize,
}
impl Default for ContactRegionBudget {
	fn default() -> Self {
		Self { maximum: 2 }
	}
}

/// Rigid world placement captured at admission. Scale, pitch, roll and shear are rejected.
#[derive(Clone, Copy, Debug)]
pub struct ContactFrame {
	pub world_from_local: Mat4,
	pub local_from_world: Mat4,
}
impl ContactFrame {
	pub fn new(matrix: Mat4) -> Result<Self, &'static str> {
		if !matrix.is_finite() {
			return Err("non-finite contact placement");
		}
		let (s, r, p) = matrix.to_scale_rotation_translation();
		if !s.abs_diff_eq(Vec3::ONE, 1e-5)
			|| !(r * Vec3::Y).abs_diff_eq(Vec3::Y, 1e-5)
			|| !matrix.abs_diff_eq(Mat4::from_rotation_translation(r, p), 1e-5)
		{
			return Err("contact placement requires translation/yaw and scale one");
		}
		Ok(Self { world_from_local: matrix, local_from_world: matrix.inverse() })
	}
	fn canonical(v: DVec3) -> DVec3 {
		(v * 1e12).round() / 1e12
	}
	pub fn local_vector(self, v: DVec3) -> DVec3 {
		Self::canonical(
			self.world_from_local
				.to_scale_rotation_translation()
				.1
				.as_dquat()
				.normalize()
				.conjugate() * v,
		)
	}
	pub fn world_vector(self, v: DVec3) -> DVec3 {
		self.world_from_local.to_scale_rotation_translation().1.as_dquat().normalize() * v
	}
	pub fn local_collider(self, mut c: Interactor) -> Interactor {
		let (_, q, p) = self.world_from_local.to_scale_rotation_translation();
		let q = q.as_dquat().normalize().conjugate();
		c.position = q * (c.position - p.as_dvec3());
		c.rotation = (q * c.rotation).normalize();
		c.linear = q * c.linear;
		c.angular = q * c.angular;
		// Rigid inverse roundoff at exact MAC face/solid boundaries must not flip
		// occupancy. Canonicalize inputs only, at 1 picometre / 1e-12 radians.
		c.position = Self::canonical(c.position);
		c.linear = Self::canonical(c.linear);
		c.angular = Self::canonical(c.angular);
		let v = c.rotation.to_array().map(|x| (x * 1e12).round() / 1e12);
		c.rotation = bevy::math::DQuat::from_array(v).normalize();
		c
	}
	/// Conservative world-space broad phase for this region's fixed field.
	pub fn relevant(self, collider: Interactor) -> bool {
		let c = self.local_collider(collider);
		let radius = match c.shape {
			Shape::Capsule { radius, half_segment } => radius + half_segment,
			Shape::Box { half } => half.length(),
		} + c.influence
			+ 0.5;
		(c.position - DVec3::new(1., 1.5, 0.))
			.abs()
			.cmple(DVec3::new(3., 1.5, 3.) + DVec3::splat(radius))
			.all()
	}
	pub fn world_collider(self, mut c: Interactor) -> Interactor {
		let (_, q, p) = self.world_from_local.to_scale_rotation_translation();
		let q = q.as_dquat().normalize();
		c.position = q * c.position + p.as_dvec3();
		c.rotation = q * c.rotation;
		c.linear = q * c.linear;
		c.angular = q * c.angular;
		c
	}
}
#[derive(Clone)]
pub struct WorldContactSnapshot {
	/// Seconds on this effect's playback-scaled VfxInstance.age clock, NOT wall time.
	/// Callers using wall time convert once; runtime never multiplies this again.
	pub age: f64,
	/// First relevant solid supplies the retained primary local response/wake.
	/// All solids affect MAC boundaries, reconstruction and optical occupancy.
	pub colliders: Vec<Interactor>,
	/// Rates (including collider linear/angular velocities) are per effect-age second.
	pub wind: DVec3,
	/// No interpolation or inherited velocities across a teleport/reset boundary.
	pub discontinuous: bool,
}
impl WorldContactSnapshot {
	/// Supply an already playback-scaled effect age with ordinary world/game-clock
	/// velocities. Convert rates ONCE to metres/radians per effect-age second.
	/// Callers already using age-clock rates should construct the snapshot directly.
	pub fn from_world_rates(
		age: f64,
		playback: f64,
		mut colliders: Vec<Interactor>,
		wind: DVec3,
		discontinuous: bool,
	) -> Result<Self, &'static str> {
		if !playback.is_finite() || playback <= 0. {
			return Err("contact playback must be finite and positive");
		}
		for c in &mut colliders {
			c.linear /= playback;
			c.angular /= playback;
		}
		Ok(Self { age, colliders, wind: wind / playback, discontinuous })
	}
}
/// Insert on the root returned by typed spawn_scene. Root Entity is the region identity.
/// With no component, origin-preview scripts remain available for regression purposes.
#[derive(Component, Clone, Default)]
pub struct ContactInputs {
	snapshots: VecDeque<Arc<WorldContactSnapshot>>,
	pub rejected: u64,
	pub revision: u64,
}
impl ContactInputs {
	/// Input is transactional: invalid/over-budget snapshots are rejected, never truncated.
	/// Out-of-order time starts a new branch (seek/reset); equal times replace the latest.
	pub fn submit(&mut self, input: WorldContactSnapshot) -> Result<(), &'static str> {
		let valid = input.age.is_finite()
			&& input.age >= 0.
			&& input.wind.is_finite()
			&& input.colliders.len() <= MAX_COLLIDERS
			&& input.colliders.iter().all(|c| {
				c.position.is_finite()
					&& c.rotation.is_finite()
					&& (c.rotation.length_squared() - 1.).abs() < 1e-5
					&& c.linear.is_finite()
					&& c.angular.is_finite()
					&& c.influence.is_finite()
					&& c.influence >= 0.
					&& c.strength.is_finite()
					&& c.entrainment.is_finite()
					&& c.slide.is_finite()
					&& match c.shape {
						Shape::Capsule { radius, half_segment } => {
							radius.is_finite()
								&& radius > 0. && half_segment.is_finite()
								&& half_segment >= 0.
						}
						Shape::Box { half } => half.is_finite() && half.min_element() > 0.,
					}
			}) && !input
			.colliders
			.iter()
			.enumerate()
			.any(|(i, c)| input.colliders[..i].iter().any(|d| d.id == c.id));
		if !valid {
			self.rejected += 1;
			return Err("invalid contact snapshot or more than 20 colliders");
		}
		while self.snapshots.back().is_some_and(|s| s.age >= input.age) {
			self.snapshots.pop_back();
		}
		self.snapshots.push_back(Arc::new(input));
		while self.snapshots.len() > MAX_INPUTS {
			self.snapshots.pop_front();
		}
		self.revision = self.revision.wrapping_add(1);
		Ok(())
	}
	/// Clear/reseed history before an explicit replay/seek older than its retained window.
	pub fn clear(&mut self) {
		self.snapshots.clear();
		self.revision = self.revision.wrapping_add(1);
	}
	pub fn retained_ages(&self) -> Option<(f64, f64)> {
		Some((self.snapshots.front()?.age, self.snapshots.back()?.age))
	}
	/// Submitted vs conservatively selected geometry. Outside-domain omission is deliberate.
	pub fn collider_counts(&self, frame: ContactFrame, age: f64) -> (usize, usize) {
		let submitted = self
			.snapshots
			.iter()
			.rev()
			.find(|s| s.age <= age + 1e-9)
			.map_or(0, |s| s.colliders.len());
		(submitted, self.sample(frame, age, &InteractionScene::default()).collider_count())
	}
	fn sample(&self, frame: ContactFrame, time: f64, base: &InteractionScene) -> InteractionScene {
		let mut scene = base.clone();
		scene.external = Some(Vec::new());
		scene.manual.clear();
		scene.version = self.revision;
		let Some(a) = self.snapshots.iter().rev().find(|s| s.age <= time + 1e-9) else {
			return scene;
		};
		let next = self.snapshots.iter().find(|s| s.age > a.age);
		let next = next.filter(|s| !s.discontinuous && s.age - a.age <= 0.25);
		let u = next.map_or(0., |b| ((time - a.age) / (b.age - a.age)).clamp(0., 1.));
		let wind = next.map_or(a.wind, |b| a.wind.lerp(b.wind, u));
		scene.wind = frame.local_vector(wind);
		// Missing future data: hold position with zero velocity rather than extrapolating.
		let fresh = (time - a.age).abs() < 1e-8;
		let mut colliders = Vec::with_capacity(a.colliders.len());
		for source in &a.colliders {
			let mut c = *source;
			if let Some(b) = next.and_then(|b| b.colliders.iter().find(|b| b.id == c.id)) {
				c.position = c.position.lerp(b.position, u);
				c.rotation = c.rotation.slerp(b.rotation, u);
				c.linear = c.linear.lerp(b.linear, u);
				c.angular = c.angular.lerp(b.angular, u);
			} else if !fresh || a.discontinuous {
				c.linear = DVec3::ZERO;
				c.angular = DVec3::ZERO;
			}
			if fresh && a.discontinuous {
				c.linear = DVec3::ZERO;
				c.angular = DVec3::ZERO;
			}
			c = frame.local_collider(c);
			let support = match c.shape {
				Shape::Capsule { radius, half_segment } => radius + half_segment,
				Shape::Box { half } => half.length(),
			} + c.influence
				+ 0.5;
			// Spherical support around rotated solids conservatively covers the full MAC domain.
			if (c.position - DVec3::new(1., 1.5, 0.))
				.abs()
				.cmple(DVec3::new(3., 1.5, 3.) + DVec3::splat(support))
				.all()
			{
				colliders.push(c);
			}
		}
		scene.external = Some(colliders);
		scene
	}
}
#[derive(Clone)]
pub struct ContactRegion {
	pub frame: ContactFrame,
	pub root_transform: Mat4,
	pub inputs: Option<ContactInputs>,
	pub base: InteractionScene,
}
impl ContactRegion {
	pub fn scene(&self, time: f64) -> InteractionScene {
		self.inputs
			.as_ref()
			.map_or_else(|| self.base.clone(), |i| i.sample(self.frame, time, &self.base))
	}
	pub fn world_colliders(&self, time: f64) -> Vec<Interactor> {
		let scene = self.scene(time);
		scene
			.colliders(time)
			.into_iter()
			.take(scene.collider_count())
			.map(|c| self.frame.world_collider(c))
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn rigid_inputs_distances_vectors_and_rejection() -> Result<(), String> {
		let source = InteractionScene::default().colliders(0.3)[0];
		for (p, yaw) in [(Vec3::new(20., 0., -15.), 90.), (Vec3::new(-12., 8., 18.), 37.)] {
			let f = ContactFrame::new(Mat4::from_rotation_translation(
				Quat::from_rotation_y(f32::to_radians(yaw)),
				p,
			))?;
			let c = f.local_collider(f.world_collider(source));
			assert!((c.position - source.position).length() < 1e-12);
			assert!((c.linear - source.linear).length() < 1e-12);
			let local = DVec3::new(0.1, 0.6, 0.2);
			let world = f.world_from_local.transform_point3(local.as_vec3()).as_dvec3();
			let a = source.query(local);
			let b = f.world_collider(source).query(world);
			assert!((a.distance - b.distance).abs() < 3e-6);
			assert!(f
				.local_from_world
				.transform_vector3(b.normal.as_vec3())
				.abs_diff_eq(a.normal.as_vec3(), 3e-6));
		}
		for m in [
			Mat4::from_scale(Vec3::splat(2.)),
			Mat4::from_scale(Vec3::new(1., 2., 1.)),
			Mat4::from_rotation_x(0.1),
			Mat4::from_rotation_z(0.1),
			Mat4::from_translation(Vec3::splat(f32::NAN)),
		] {
			assert!(ContactFrame::new(m).is_err());
		}
		Ok(())
	}
	#[test]
	fn world_rates_convert_once_without_scaling_age() -> Result<(), String> {
		let mut collider = InteractionScene::default().colliders(0.)[0];
		collider.linear = DVec3::X * 2.;
		collider.angular = DVec3::Y * 0.6;
		let snapshot =
			WorldContactSnapshot::from_world_rates(2., 2., vec![collider], DVec3::Z * 0.4, false)?;
		assert_eq!(snapshot.age, 2.);
		assert_eq!(snapshot.colliders[0].linear, DVec3::X);
		assert_eq!(snapshot.colliders[0].angular, DVec3::Y * 0.3);
		assert_eq!(snapshot.wind, DVec3::Z * 0.2);
		assert!(WorldContactSnapshot::from_world_rates(0., 0., vec![], DVec3::ZERO, false).is_err());
		Ok(())
	}

	#[test]
	fn timestamp_interpolation_missing_history_discontinuity_and_budgets() -> Result<(), String> {
		let f = ContactFrame::new(Mat4::IDENTITY)?;
		let base = InteractionScene::default();
		let mut a = base.colliders(0.)[0];
		a.position = DVec3::ZERO;
		a.linear = DVec3::X;
		let mut b = a;
		b.position = DVec3::X * 0.1;
		let mut input = ContactInputs::default();
		input.submit(WorldContactSnapshot {
			age: 0.,
			colliders: vec![a],
			wind: DVec3::X,
			discontinuous: false,
		})?;
		input.submit(WorldContactSnapshot {
			age: 0.1,
			colliders: vec![b],
			wind: DVec3::X,
			discontinuous: false,
		})?;
		assert!((input.sample(f, 0.05, &base).capsule(0.).position.x - 0.05).abs() < 1e-12);
		assert_eq!(input.sample(f, 0.2, &base).capsule(0.).linear, DVec3::ZERO);
		let other = input.clone();
		input.submit(WorldContactSnapshot {
			age: 0.1,
			colliders: vec![b],
			wind: DVec3::X,
			discontinuous: true,
		})?;
		assert_eq!(input.sample(f, 0.05, &base).capsule(0.).position, a.position);
		assert_ne!(other.sample(f, 0.05, &base).capsule(0.).position, a.position);
		assert!(input
			.submit(WorldContactSnapshot {
				age: 0.2,
				colliders: vec![a; 21],
				wind: DVec3::X,
				discontinuous: false
			})
			.is_err());
		assert_eq!(input.rejected, 1);
		assert_eq!(input.snapshots.len(), 2);
		assert_eq!(input.sample(f, -0.1, &base).collider_count(), 0);
		let colliders = (0..20).map(|id| Interactor { id, ..a }).collect::<Vec<_>>();
		input.submit(WorldContactSnapshot {
			age: 0.2,
			colliders,
			wind: DVec3::X,
			discontinuous: false,
		})?;
		assert_eq!(input.collider_counts(f, 0.2), (20, 20));
		let far = ContactFrame::new(Mat4::from_translation(Vec3::splat(100.)))?;
		assert_eq!(input.collider_counts(far, 0.2), (20, 0));
		assert!(input.rejected > 0);

		Ok(())
	}
}
