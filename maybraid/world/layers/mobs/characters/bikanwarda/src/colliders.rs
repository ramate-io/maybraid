//! Motor + query hit colliders sized to the visual hull.
//!
//! The walking motor stays one vertical capsule. Hits use a compound capsule
//! set (body + outer flanks + four legs) authored from the same
//! [`LocomotionCapsule`] that [`crate::spec::BikanwardaSpec::uniform_scale`]
//! produces, so the 1/3 shrink of [`crate::ENCOUNTER_LINEAR_SCALE`] is shared.

use avian3d::prelude::Collider;
use bevy::prelude::*;
use characters::components::{HitCapsule, LocomotionCapsule};

/// Assembly-local collider shapes (motor vertical capsule + pronograde body).
///
/// Legs and outer flanks live on the query-only [`hit_complex_collider`], not
/// the walking motor.
pub fn assembly_collider_shapes(capsule: LocomotionCapsule) -> Vec<(Vec3, Quat, Collider)> {
	let mut shapes = Vec::with_capacity(2);
	shapes.push((
		Vec3::ZERO,
		Quat::IDENTITY,
		Collider::capsule(capsule.radius, capsule.length.max(1e-3)),
	));
	if let Some(hit) = capsule.hit_capsule() {
		let rest = hit.local_transform();
		shapes.push((
			rest.translation,
			rest.rotation,
			Collider::capsule(hit.radius, hit.length.max(1e-3)),
		));
	}
	shapes
}

pub fn motor_collider(capsule: LocomotionCapsule) -> Collider {
	Collider::compound(assembly_collider_shapes(capsule))
}

/// Query-only compound covering the pronograde body, outer flanks, and legs.
pub fn hit_complex_collider(capsule: LocomotionCapsule) -> Collider {
	Collider::compound(hit_complex_shapes(capsule))
}

/// Motor-local capsule parts for the query hull. Empty when the hull is not
/// pronograde.
pub fn hit_complex_shapes(capsule: LocomotionCapsule) -> Vec<(Vec3, Quat, Collider)> {
	let Some(hit) = capsule.hit_capsule() else {
		return Vec::new();
	};
	let mut shapes = Vec::with_capacity(7);
	let rest = hit.local_transform();
	shapes.push((
		rest.translation,
		rest.rotation,
		Collider::capsule(hit.radius, hit.length.max(1e-3)),
	));
	let layout = HitComplexLayout::from_hull(capsule, hit);
	for translation in layout.leg_centers() {
		shapes.push((
			translation,
			Quat::IDENTITY,
			Collider::capsule(layout.leg_radius, layout.leg_length),
		));
	}
	for translation in layout.outer_centers() {
		shapes.push((
			translation,
			Quat::IDENTITY,
			Collider::capsule(layout.outer_radius, layout.outer_length),
		));
	}
	shapes
}

/// Aim points in motor-local space that sit on the legs / outer flanks.
pub fn leg_aim_point(capsule: LocomotionCapsule) -> Vec3 {
	let layout =
		HitComplexLayout::from_hull(capsule, capsule.hit_capsule().expect("pronograde hull"));
	let center = layout.leg_centers()[0];
	// Bottom hemisphere of the vertical leg, below the pronograde body.
	Vec3::new(center.x, center.y - layout.leg_length * 0.5, center.z)
}

pub fn outer_body_aim_point(capsule: LocomotionCapsule) -> Vec3 {
	let layout =
		HitComplexLayout::from_hull(capsule, capsule.hit_capsule().expect("pronograde hull"));
	layout.outer_centers()[0] + Vec3::X * layout.outer_radius * 0.4
}

#[derive(Clone, Copy)]
struct HitComplexLayout {
	half_span: f32,
	half_width: f32,
	along: f32,
	leg_radius: f32,
	leg_length: f32,
	leg_center_y: f32,
	outer_radius: f32,
	outer_length: f32,
}

impl HitComplexLayout {
	fn from_hull(capsule: LocomotionCapsule, hit: HitCapsule) -> Self {
		let stock = LocomotionCapsule::QUADRUPED.radius;
		let scale = if stock <= 1e-5 { 1.0 } else { (capsule.radius / stock).max(0.0) };
		let half_span = HitCapsule::REST_HALF_SPAN * scale;
		let half_width = hit.radius.max(HitCapsule::REST_HALF_WIDTH * scale);
		let leg_radius = (capsule.radius * 0.62).max(0.08 * scale.max(1e-3));
		let foot_y = -capsule.half_height() + leg_radius;
		let hip_y = -leg_radius * 0.15;
		let total = (hip_y - foot_y).abs().max(leg_radius * 2.0 + 1e-3);
		let leg_length = (total - 2.0 * leg_radius).max(1e-3);
		let outer_radius = (hit.radius * 0.55).max(leg_radius);
		let outer_length = (capsule.length * 0.35).max(1e-3);
		Self {
			half_span,
			half_width,
			along: hit.along,
			leg_radius,
			leg_length,
			leg_center_y: (hip_y + foot_y) * 0.5,
			outer_radius,
			outer_length,
		}
	}

	fn leg_centers(self) -> [Vec3; 4] {
		let w = self.half_width * 0.85;
		let s = self.half_span;
		let y = self.leg_center_y;
		[Vec3::new(w, y, s), Vec3::new(-w, y, s), Vec3::new(w, y, -s), Vec3::new(-w, y, -s)]
	}

	fn outer_centers(self) -> [Vec3; 2] {
		[Vec3::new(self.half_width, 0.0, self.along), Vec3::new(-self.half_width, 0.0, self.along)]
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::recipe::BikanwardaActorRecipe;
	use avian3d::prelude::{
		PhysicsPlugins, RigidBody, ShapeCastConfig, SpatialQuery, SpatialQueryFilter,
	};
	use bevy::ecs::system::RunSystemOnce;
	use bevy::shape::Aabb3d;
	use characters::{CharacterComponents, CharacterRecipe};
	use std::time::Instant;

	fn encounter_hull() -> LocomotionCapsule {
		BikanwardaActorRecipe::default().components().scaled_locomotion_capsule()
	}

	#[test]
	fn collider_shapes_track_encounter_scale() {
		let actor = BikanwardaActorRecipe::default().components();
		let ordinary = actor.locomotion_capsule();
		let encounter = actor.scaled_locomotion_capsule();
		assert!(encounter.radius > ordinary.radius * 3.0);
		assert!(encounter.radius < ordinary.radius * 4.0);
		assert_eq!(assembly_collider_shapes(encounter).len(), 2, "motor + pronograde body");
		assert_eq!(hit_complex_shapes(encounter).len(), 7, "body + four legs + two outer flanks");
	}

	#[test]
	fn motor_aabb_fits_inside_visual_bounds() {
		let capsule = BikanwardaActorRecipe::default().components().scaled_locomotion_capsule();
		let visual = capsule.preview_aabb();
		let motor = capsule.preview_aabb();
		let vmin = Vec3::from(visual.min);
		let vmax = Vec3::from(visual.max);
		let mmin = Vec3::from(motor.min);
		let mmax = Vec3::from(motor.max);
		assert!(mmin.x >= vmin.x - 1e-3 && mmin.y >= vmin.y - 1e-3);
		assert!(mmax.x <= vmax.x + 1e-3 && mmax.y <= vmax.y + 1e-3);
		let _ = Aabb3d::from_min_max(vmin, vmax);
	}

	fn hit_capsule_contains(hit: HitCapsule, point: Vec3) -> bool {
		let rest = hit.local_transform();
		let axis = rest.rotation * Vec3::Y;
		let half = hit.length * 0.5;
		let a = rest.translation - axis * half;
		let b = rest.translation + axis * half;
		let ab = b - a;
		let denom = ab.length_squared().max(1e-8);
		let t = ((point - a).dot(ab) / denom).clamp(0.0, 1.0);
		(a + ab * t).distance(point) <= hit.radius + 1e-3
	}

	#[test]
	fn hit_complex_covers_legs_and_outer_flanks() {
		let hull = encounter_hull();
		let shapes = hit_complex_shapes(hull);
		assert_eq!(shapes.len(), 7);
		let hit = hull.hit_capsule().unwrap();
		let leg = leg_aim_point(hull);
		let outer = outer_body_aim_point(hull);
		assert!(
			!hit_capsule_contains(hit, leg),
			"leg sample must sit outside the pronograde body, {leg:?} radius={}",
			hit.radius
		);
		assert!(
			!hit_capsule_contains(hit, outer),
			"outer sample must sit outside the pronograde body, {outer:?} radius={}",
			hit.radius
		);
		assert!(
			outer.x.abs() > hit.radius,
			"outer sample must sit past the flank, x={} radius={}",
			outer.x,
			hit.radius
		);
	}

	fn tessellated_box_trimesh(min: Vec3, max: Vec3, divisions: u32) -> Collider {
		let n = divisions.max(1);
		let size = max - min;
		let mut vertices = Vec::new();
		let mut indices = Vec::<[u32; 3]>::new();
		let mut add_face = |origin: Vec3, u_dir: Vec3, v_dir: Vec3| {
			let first = vertices.len() as u32;
			for y in 0..=n {
				let v = y as f32 / n as f32;
				for x in 0..=n {
					let u = x as f32 / n as f32;
					vertices.push(origin + u_dir * u + v_dir * v);
				}
			}
			let stride = n + 1;
			for y in 0..n {
				for x in 0..n {
					let i0 = first + y * stride + x;
					let i1 = i0 + 1;
					let i2 = i0 + stride;
					let i3 = i2 + 1;
					indices.push([i0, i2, i1]);
					indices.push([i1, i2, i3]);
				}
			}
		};
		add_face(min, Vec3::X * size.x, Vec3::Y * size.y);
		add_face(Vec3::new(min.x, min.y, max.z), Vec3::X * size.x, Vec3::Y * size.y);
		add_face(min, Vec3::X * size.x, Vec3::Z * size.z);
		add_face(Vec3::new(min.x, max.y, min.z), Vec3::X * size.x, Vec3::Z * size.z);
		add_face(min, Vec3::Y * size.y, Vec3::Z * size.z);
		add_face(Vec3::new(max.x, min.y, min.z), Vec3::Y * size.y, Vec3::Z * size.z);
		Collider::trimesh(vertices, indices)
	}

	fn shapecast_hits(collider: Collider, origin: Vec3, direction: Dir3) -> bool {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
		));
		app.world_mut().spawn((
			Transform::IDENTITY,
			GlobalTransform::IDENTITY,
			collider,
			RigidBody::Static,
		));
		app.update();
		app.update();
		app.world_mut()
			.run_system_once(move |spatial: SpatialQuery| {
				spatial
					.cast_shape(
						&Collider::capsule(0.05, 0.2),
						origin,
						Quat::IDENTITY,
						direction,
						&ShapeCastConfig::from_max_distance(24.0),
						&SpatialQueryFilter::default(),
					)
					.is_some()
			})
			.unwrap_or(false)
	}

	#[test]
	fn shapecast_hits_leg_and_outer_samples() {
		let hull = encounter_hull();
		let complex = hit_complex_collider(hull);
		let body_only = hull
			.hit_capsule()
			.map(|hit| {
				Collider::compound(vec![(
					hit.local_transform().translation,
					hit.local_transform().rotation,
					Collider::capsule(hit.radius, hit.length.max(1e-3)),
				)])
			})
			.expect("pronograde hull");
		let leg = leg_aim_point(hull);
		let outer = outer_body_aim_point(hull);
		let leg_origin = Vec3::new(-8.0, leg.y, leg.z);
		let outer_origin = Vec3::new(outer.x, outer.y, -8.0);
		assert!(
			!shapecast_hits(body_only.clone(), leg_origin, Dir3::X),
			"the single pronograde capsule must miss the leg sample at {leg:?}"
		);
		assert!(
			!shapecast_hits(body_only, outer_origin, Dir3::Z),
			"the single pronograde capsule must miss the outer-flank sample at {outer:?}"
		);
		assert!(
			shapecast_hits(complex.clone(), leg_origin, Dir3::X),
			"compound must register a leg sample at {leg:?}"
		);
		assert!(
			shapecast_hits(complex, outer_origin, Dir3::Z),
			"compound must register an outer-flank sample at {outer:?}"
		);
	}

	#[test]
	fn compound_shapecast_is_cheaper_than_a_hull_trimesh() {
		let hull = encounter_hull();
		let aabb = hull.preview_aabb();
		let min = Vec3::from(aabb.min);
		let max = Vec3::from(aabb.max);
		let compound = hit_complex_collider(hull);
		let trimesh = tessellated_box_trimesh(min, max, 8);
		let origin = Vec3::new(-12.0, 0.0, 0.0);
		let warm = |collider: Collider| shapecast_hits(collider, origin, Dir3::X);
		assert!(warm(compound.clone()));
		assert!(warm(trimesh.clone()));
		const N: u32 = 80;
		let compound_ns = time_shapecasts(compound, origin, N);
		let trimesh_ns = time_shapecasts(trimesh, origin, N);
		eprintln!("hit-complex shapecast: compound={compound_ns}ns trimesh={trimesh_ns}ns n={N}");
		assert!(
			compound_ns < trimesh_ns.saturating_mul(3),
			"capsule compound should stay in the same class as a hull trimesh, compound={compound_ns}ns trimesh={trimesh_ns}ns"
		);
	}

	fn time_shapecasts(collider: Collider, origin: Vec3, n: u32) -> u128 {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
		));
		app.world_mut().spawn((
			Transform::IDENTITY,
			GlobalTransform::IDENTITY,
			collider,
			RigidBody::Static,
		));
		app.update();
		app.update();
		app.world_mut()
			.run_system_once(move |spatial: SpatialQuery| {
				let probe = Collider::capsule(0.05, 0.2);
				let config = ShapeCastConfig::from_max_distance(24.0);
				let filter = SpatialQueryFilter::default();
				let started = Instant::now();
				for i in 0..n {
					let at = origin + Vec3::Z * (i as f32) * 0.01;
					let _ =
						spatial.cast_shape(&probe, at, Quat::IDENTITY, Dir3::X, &config, &filter);
				}
				started.elapsed().as_nanos()
			})
			.unwrap_or(u128::MAX)
	}
}
