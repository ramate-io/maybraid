//! Apply [`TerrainPitch`] from an [`ElevationProbe`]. No Durham types.
//!
//! Two downward rays (front / hind). Side rays only if `roll_weight > 0`.
//! The capsule stays upright and owns world Y. The visual child pitches about
//! the origin and takes a signed Y offset so the chord midpoint matches the
//! sampled slope, without max-lifting the rear into the air.
//!
//! Front/hind rays follow [`TerrainPitch::sagittal`] (live shoulder–hip) when
//! that axis is set, otherwise Bevy mesh `+Z`. Wheelbase is the first rest
//! measure, not the pitched chord. Capsule children sample from the parent
//! origin so support Y cannot lift the next ray. Locomotion, look, and combat
//! write [`CharacterHeading`], so pitch never infers yaw from its own output.
//! Pitch, roll, and support continuously follow valid samples.

use bevy::ecs::batching::BatchingStrategy;
use bevy::prelude::*;
use ground::ElevationProbe;

use crate::markers::{ApplyTerrainPitch, SuspendTerrainPitch};
use crate::pitch::{
	CharacterHeading, TerrainPitch, TerrainPitchProbe, MAX_TILT, SUPPORT_RATE, TILT_RATE,
};

/// Start the ray this far above the body so it clears the capsule.
pub const PROBE_LIFT: f32 = 2.0;
/// Extra downward reach after the uphill clearance, for downhill samples.
pub const PROBE_MAX_DISTANCE: f32 = 6.0;
/// Draw last-frame front/hind hits. Playgrounds leave this on; set `false` to hide.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawTerrainPitchProbes(pub bool);

impl Default for DrawTerrainPitchProbes {
	fn default() -> Self {
		Self(true)
	}
}

/// Extra height so an uphill sample still starts above the mesh.
pub fn probe_clearance(half_span: f32) -> f32 {
	PROBE_LIFT + half_span.max(0.0) * MAX_TILT.tan()
}

fn probe_from_y(origin_y: f32, half_span: f32) -> f32 {
	origin_y + probe_clearance(half_span)
}

fn probe_distance(half_span: f32) -> f32 {
	probe_clearance(half_span) + PROBE_MAX_DISTANCE
}

/// Local visual under a capsule sits near the parent origin. World-placed hosts
/// keep large local XZ when parented to an identity cell/group root.
pub fn is_local_visual_child(local_translation: Vec3) -> bool {
	Vec2::new(local_translation.x, local_translation.z).length_squared() < 4.0
}

/// Ray start: capsule world pose for a local visual child, else the visual.
pub fn probe_origin(visual_world: Vec3, parent_world: Option<Vec3>, local_child: bool) -> Vec3 {
	if local_child {
		parent_world.unwrap_or(visual_world)
	} else {
		visual_world
	}
}

fn ancestor_exclude(
	start: Option<Entity>,
	child_of: &Query<&ChildOf>,
	suspended: &Query<(), With<SuspendTerrainPitch>>,
) -> (Vec<Entity>, bool) {
	let mut exclude = Vec::new();
	let mut airborne = false;
	let mut current = start;
	for _ in 0..32 {
		let Some(entity) = current else {
			break;
		};
		exclude.push(entity);
		airborne |= suspended.contains(entity);
		current = child_of.get(entity).ok().map(ChildOf::parent);
	}
	(exclude, airborne)
}

/// Tilt visuals that carry both [`TerrainPitch`] and host [`ApplyTerrainPitch`].
///
/// `P` is typically [`ground_avian::AvianElevationProbe`]. Register after
/// physics / locomotion. Sample from the capsule parent when the visual is a
/// near-origin child; world-placed hosts still use the visual pose. Exclude
/// every ancestor so a cell/group root is never treated as the capsule.
pub fn apply_terrain_pitch<P>(
	time: Res<Time>,
	probe: P,
	mut visuals: Query<
		(Entity, &mut Transform, &GlobalTransform, &mut TerrainPitch, &mut CharacterHeading),
		With<ApplyTerrainPitch>,
	>,
	child_of: Query<&ChildOf>,
	parents: Query<&GlobalTransform, Without<ApplyTerrainPitch>>,
	suspended: Query<(), With<SuspendTerrainPitch>>,
) where
	P: ElevationProbe + Sync,
{
	let dt = time.delta_secs();
	visuals.par_iter_mut().batching_strategy(BatchingStrategy::fixed(16)).for_each(
		|(entity, mut visual, global, mut pitch, mut heading)| {
			let yaw_facing = heading.facing(&visual);
			if heading.0 != yaw_facing {
				heading.0 = yaw_facing;
			}
			let ray_facing = pitch.sample_facing(yaw_facing);
			let right = Vec3::new(yaw_facing.z, 0.0, -yaw_facing.x);

			let (exclude, suspend) = ancestor_exclude(Some(entity), &child_of, &suspended);
			let offset_local_y =
				child_of.get(entity).is_ok() && is_local_visual_child(visual.translation);
			let parent_world = child_of
				.get(entity)
				.ok()
				.and_then(|child| parents.get(child.parent()).ok())
				.map(GlobalTransform::translation);
			let origin = probe_origin(global.translation(), parent_world, offset_local_y);

			let from_y = probe_from_y(origin.y, pitch.half_span);
			let max_distance = probe_distance(pitch.half_span);
			let sample = |xz: Vec3| -> (Vec3, bool) {
				match probe.height_at(xz.x, xz.z, from_y, max_distance, &exclude) {
					Some(h) => (Vec3::new(xz.x, h, xz.z), true),
					None => (Vec3::new(xz.x, origin.y, xz.z), false),
				}
			};

			let (center, center_hit) = sample(origin);
			let center_h = center.y;
			let mut run = pitch.half_span;
			let mut front = center;
			let mut hind = center;
			let mut front_hit = false;
			let mut hind_hit = false;
			let mut left_h = center_h;
			let mut right_h = center_h;
			let mut support_hits = false;
			if !suspend {
				let (coarse_front, coarse_front_hit) =
					sample(origin + ray_facing * pitch.half_span);
				let (coarse_hind, coarse_hind_hit) = sample(origin - ray_facing * pitch.half_span);
				let coarse =
					TerrainPitch::observed_pitch(coarse_front.y, coarse_hind.y, pitch.half_span);
				run = pitch.pitched_half_run(coarse);
				(front, front_hit) = sample(origin + ray_facing * run);
				(hind, hind_hit) = sample(origin - ray_facing * run);
				support_hits =
					center_hit && coarse_front_hit && coarse_hind_hit && front_hit && hind_hit;
				if pitch.roll_weight > 0.0 {
					let (left, left_hit) = sample(origin - right * pitch.half_width);
					let (right, right_hit) = sample(origin + right * pitch.half_width);
					left_h = left.y;
					right_h = right.y;
					support_hits &= left_hit && right_hit;
				}
			}

			let targets = if suspend {
				Some((0.0, 0.0, 0.0))
			} else if support_hits {
				Some((
					TerrainPitch::observed_pitch(front.y, hind.y, run) * pitch.pitch_weight,
					TerrainPitch::observed_roll(left_h, right_h, pitch.half_width)
						* pitch.roll_weight,
					TerrainPitch::support_offset(center_h, front.y, hind.y),
				))
			} else {
				None
			};
			if let Some((target_pitch, target_roll, _)) = targets {
				let next_pitch =
					TerrainPitch::follow_target(pitch.pitch, target_pitch, dt, TILT_RATE);
				let next_roll = TerrainPitch::follow_target(pitch.roll, target_roll, dt, TILT_RATE);
				if pitch.pitch != next_pitch {
					pitch.pitch = next_pitch;
				}
				if pitch.roll != next_roll {
					pitch.roll = next_roll;
				}
			}
			let next_probe = TerrainPitchProbe {
				origin: Vec3::new(origin.x, center_h, origin.z),
				front,
				hind,
				front_hit,
				hind_hit,
				visual_facing: yaw_facing,
				sample_facing: ray_facing,
			};
			if pitch.probe != next_probe {
				pitch.probe = next_probe;
			}
			let next_rotation = pitch.facing_with_support_tilt(yaw_facing, ray_facing);
			if visual.rotation != next_rotation {
				visual.rotation = next_rotation;
			}
			if offset_local_y {
				if let Some((_, _, target_support)) = targets {
					let next_support = TerrainPitch::follow_target(
						pitch.support,
						target_support,
						dt,
						SUPPORT_RATE,
					);
					if pitch.support != next_support {
						pitch.support = next_support;
					}
				}
				if visual.translation.y != pitch.support {
					visual.translation.y = pitch.support;
				}
			}
		},
	);
}

/// Front = lime, hind = orange, misses = red. Yellow = sample axis, cyan = heading.
/// Magenta = live girdle bones/midpoints. A red ring means girdles were seen but
/// the XZ run was too short to accept as a sagittal axis.
pub fn draw_terrain_pitch_probes(
	draw: Option<Res<DrawTerrainPitchProbes>>,
	mut gizmos: Gizmos,
	pitched: Query<&TerrainPitch, With<ApplyTerrainPitch>>,
) {
	if !draw.map(|d| d.0).unwrap_or(true) {
		return;
	}
	for pitch in &pitched {
		draw_girdles(&mut gizmos, pitch);
		let probe = pitch.probe;
		if probe.sample_facing.length_squared() < 1e-6 {
			continue;
		}
		let span = pitch.half_span.max(0.2);
		let origin = probe.origin;
		gizmos.line(
			origin - probe.sample_facing * span,
			origin + probe.sample_facing * span,
			Color::srgb(1.0, 0.9, 0.15),
		);
		gizmos.line(origin, origin + probe.visual_facing * span, Color::srgb(0.25, 0.75, 1.0));
		draw_probe(&mut gizmos, probe.front, probe.front_hit, Color::srgb(0.25, 0.95, 0.35));
		draw_probe(&mut gizmos, probe.hind, probe.hind_hit, Color::srgb(0.95, 0.5, 0.12));
		gizmos.sphere(Isometry3d::from_translation(origin), 0.08, Color::srgb(1.0, 1.0, 1.0));
	}
}

fn draw_girdles(gizmos: &mut Gizmos, pitch: &TerrainPitch) {
	let g = pitch.girdles;
	let bone = Color::srgb(0.85, 0.2, 1.0);
	let chord = Color::srgb(1.0, 0.35, 0.85);
	for p in [g.shoulder_l, g.shoulder_r, g.hip_l, g.hip_r].into_iter().flatten() {
		gizmos.sphere(Isometry3d::from_translation(p), 0.1, bone);
		gizmos.line(p, p + Vec3::Y * 0.45, bone);
	}
	if let (Some(front), Some(hind)) = (g.front, g.hind) {
		gizmos.line(front, hind, chord);
		gizmos.sphere(Isometry3d::from_translation(front), 0.16, Color::srgb(0.4, 1.0, 0.85));
		gizmos.sphere(Isometry3d::from_translation(hind), 0.16, Color::srgb(1.0, 0.7, 0.2));
	}
	if !g.sagittal_ok {
		if let Some(at) = g.front.or(g.hind).or(g.shoulder_l).or(g.hip_l) {
			gizmos.sphere(Isometry3d::from_translation(at), 0.28, Color::srgb(0.95, 0.2, 0.55));
		}
	}
}

fn draw_probe(gizmos: &mut Gizmos, at: Vec3, hit: bool, hit_color: Color) {
	let color = if hit { hit_color } else { Color::srgb(0.95, 0.15, 0.15) };
	let radius = if hit { 0.14 } else { 0.2 };
	gizmos.sphere(Isometry3d::from_translation(at), radius, color);
	gizmos.line(at, at + Vec3::Y * 0.6, color);
}

#[cfg(test)]
mod tests {
	use super::*;
	use ground::GroundHit;

	#[test]
	fn probe_clearance_starts_above_a_max_tilt_uphill_sample() {
		let span = 1.2;
		let clearance = probe_clearance(span);
		assert!(clearance > span * MAX_TILT.tan());
		assert!(probe_distance(span) > clearance);
	}

	#[test]
	fn capsule_child_offsets_local_y_world_host_does_not() {
		assert!(is_local_visual_child(Vec3::ZERO));
		assert!(is_local_visual_child(Vec3::new(0.1, 0.0, -0.2)));
		assert!(!is_local_visual_child(Vec3::new(40.0, 12.0, -8.0)));
	}

	#[test]
	fn capsule_child_probes_from_parent_world_host_from_visual() {
		let visual = Vec3::new(10.0, 3.0, -4.0);
		let parent = Vec3::new(10.0, 1.5, -4.0);
		assert_eq!(probe_origin(visual, Some(parent), true), parent);
		assert_eq!(probe_origin(visual, Some(parent), false), visual);
		assert_eq!(probe_origin(visual, None, true), visual);
	}

	struct FlatProbe;

	impl ElevationProbe for FlatProbe {
		fn hit_down(
			&self,
			origin: Vec3,
			_max_distance: f32,
			_exclude: &[Entity],
		) -> Option<GroundHit> {
			Some(GroundHit {
				point: Vec3::new(origin.x, 0.0, origin.z),
				normal: Vec3::Y,
				entity: Entity::PLACEHOLDER,
			})
		}
	}

	fn apply_flat_pitch(
		time: Res<Time>,
		visuals: Query<
			(Entity, &mut Transform, &GlobalTransform, &mut TerrainPitch, &mut CharacterHeading),
			With<ApplyTerrainPitch>,
		>,
		child_of: Query<&ChildOf>,
		parents: Query<&GlobalTransform, Without<ApplyTerrainPitch>>,
		suspended: Query<(), With<SuspendTerrainPitch>>,
	) {
		apply_terrain_pitch(time, FlatProbe, visuals, child_of, parents, suspended);
	}

	fn change_tick<T: Component>(world: &World, entity: Entity) -> anyhow::Result<u32> {
		Ok(world
			.entity(entity)
			.get_change_ticks::<T>()
			.ok_or_else(|| anyhow::anyhow!("missing {entity}"))?
			.changed
			.get())
	}

	#[test]
	fn settled_flat_ground_does_not_rewrite_transform() -> anyhow::Result<()> {
		use crate::markers::ApplyTerrainPitch;
		use crate::pitch::{CharacterHeading, TerrainPitch};
		use crate::rig::RigSkeletonKind;
		use bevy::ecs::system::RunSystemOnce;

		bevy::tasks::ComputeTaskPool::get_or_init(|| {
			bevy::tasks::TaskPoolBuilder::default().num_threads(2).build()
		});
		let mut world = World::new();
		world.init_resource::<Time>();
		let visual = world
			.spawn((
				ApplyTerrainPitch,
				Transform::IDENTITY,
				GlobalTransform::IDENTITY,
				TerrainPitch::new(RigSkeletonKind::Humanoid, 0.22, 0.18),
				CharacterHeading::from_rotation(Quat::IDENTITY),
			))
			.id();

		anyhow::ensure!(world.run_system_once(apply_flat_pitch).is_ok());
		anyhow::ensure!(world.run_system_once(apply_flat_pitch).is_ok());
		let before = (
			change_tick::<Transform>(&world, visual)?,
			change_tick::<TerrainPitch>(&world, visual)?,
			change_tick::<CharacterHeading>(&world, visual)?,
		);
		anyhow::ensure!(world.run_system_once(apply_flat_pitch).is_ok());
		let after = (
			change_tick::<Transform>(&world, visual)?,
			change_tick::<TerrainPitch>(&world, visual)?,
			change_tick::<CharacterHeading>(&world, visual)?,
		);
		anyhow::ensure!(
			before == after,
			"settled pitch rewrote components: {before:?} -> {after:?}"
		);
		Ok(())
	}

	#[derive(Clone, Debug, PartialEq)]
	struct RecordedRay {
		origin: Vec3,
		max_distance: f32,
		exclude: Vec<Entity>,
	}

	#[derive(Clone, Copy)]
	struct SampleCase {
		origin: Vec3,
		kind: usize,
		miss_center: bool,
	}

	impl SampleCase {
		fn height(&self, at: Vec3) -> f32 {
			if self.kind == 0 {
				0.0
			} else {
				1.0 + 0.125 * (at.x - self.origin.x) + 0.25 * (at.z - self.origin.z)
			}
		}

		fn hit(&self, ray: usize) -> bool {
			self.kind != 2 && !(self.kind == 3 && ray == 6) && !(self.miss_center && ray == 0)
		}
	}

	#[derive(Resource, Default)]
	struct ProbeFixture {
		cases: std::collections::HashMap<Entity, SampleCase>,
		rays: std::sync::Mutex<std::collections::HashMap<Entity, Vec<RecordedRay>>>,
	}

	struct CheckedProbe<'a>(&'a ProbeFixture);

	impl ElevationProbe for CheckedProbe<'_> {
		fn hit_down(
			&self,
			origin: Vec3,
			max_distance: f32,
			exclude: &[Entity],
		) -> Option<GroundHit> {
			let entity = *exclude.first()?;
			let mut recordings = self.0.rays.lock().ok()?;
			let rays = recordings.entry(entity).or_default();
			let ray = rays.len();
			rays.push(RecordedRay { origin, max_distance, exclude: exclude.to_vec() });
			drop(recordings);
			let case = self.0.cases.get(&entity)?;
			case.hit(ray).then(|| GroundHit {
				point: Vec3::new(origin.x, case.height(origin), origin.z),
				normal: Vec3::new(-0.125, 1.0, -0.25).normalize(),
				entity: Entity::PLACEHOLDER,
			})
		}
	}

	fn apply_checked_pitch(
		time: Res<Time>,
		fixture: Res<ProbeFixture>,
		visuals: Query<
			(Entity, &mut Transform, &GlobalTransform, &mut TerrainPitch, &mut CharacterHeading),
			With<ApplyTerrainPitch>,
		>,
		child_of: Query<&ChildOf>,
		parents: Query<&GlobalTransform, Without<ApplyTerrainPitch>>,
		suspended: Query<(), With<SuspendTerrainPitch>>,
	) {
		apply_terrain_pitch(time, CheckedProbe(&fixture), visuals, child_of, parents, suspended);
	}

	#[test]
	fn batches_preserve_visual_geometry_exclusions_and_support() -> anyhow::Result<()> {
		use crate::rig::RigSkeletonKind;
		use bevy::ecs::system::RunSystemOnce;

		bevy::tasks::ComputeTaskPool::get_or_init(|| {
			bevy::tasks::TaskPoolBuilder::default().num_threads(2).build()
		});
		let mut world = World::new();
		world.init_resource::<Time>();
		let dt = 1.0 / 60.0;
		world.resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(dt));
		let mut fixture = ProbeFixture::default();
		let mut characters = Vec::new();
		// Mixed archetypes span multiple batches, including a parentless visual
		// and self/ancestor suspension. Kind: flat, banked, missing, missing side, suspended.
		for index in 0..48 {
			let placement = index % 3;
			let kind = (index / 3) % 5;
			let direction = (index / 5) % 6;
			let origin = Vec3::new(20.0 + index as f32 * 4.0, 2.0, index as f32 * 0.5);
			let case = SampleCase { origin, kind, miss_center: kind == 4 && direction % 2 == 0 };
			let ancestors = if placement < 2 {
				let root = world.spawn(GlobalTransform::IDENTITY).id();
				let parent =
					world.spawn((GlobalTransform::from_translation(origin), ChildOf(root))).id();
				Some((parent, root))
			} else {
				None
			};
			let mut pitch = TerrainPitch::new(RigSkeletonKind::Humanoid, 0.35, 0.2);
			pitch.pitch_weight = 1.0;
			pitch.roll_weight = if kind == 1 || kind == 3 || kind == 4 { 1.0 } else { 0.0 };
			pitch.pitch = 0.2;
			pitch.roll = -0.1;
			pitch.support = 0.1;
			pitch.sagittal = match (index / 5) % 3 {
				0 => Vec3::ZERO,
				1 => Vec3::X,
				_ => Vec3::new(-1.0, 0.0, 1.0),
			};
			let heading = CharacterHeading(match direction {
				0 => Vec3::Z,
				1 => Vec3::X,
				2 => Vec3::NEG_Z,
				3 => Vec3::NEG_X,
				4 => Vec3::new(1.0, 0.0, 1.0).normalize(),
				_ => Vec3::ZERO,
			});
			let visual =
				Transform::from_translation(if placement == 1 { origin } else { Vec3::Y * 0.5 })
					.with_rotation(Quat::from_rotation_y(0.37));
			let global = if placement == 0 { origin + Vec3::Y * 7.0 } else { origin };
			let entity = world
				.spawn((
					ApplyTerrainPitch,
					visual,
					GlobalTransform::from_translation(global),
					pitch,
					heading,
				))
				.id();
			let mut exclude = vec![entity];
			if let Some((parent, root)) = ancestors {
				world.entity_mut(entity).insert(ChildOf(parent));
				exclude.extend([parent, root]);
			}
			if kind == 4 {
				let suspended = if direction % 2 == 0 {
					entity
				} else {
					*exclude.last().ok_or_else(|| anyhow::anyhow!("empty exclusions"))?
				};
				world.entity_mut(suspended).insert(SuspendTerrainPitch);
			}
			fixture.cases.insert(entity, case);
			characters.push((entity, case, placement, visual, pitch, heading, exclude));
		}
		// A complete visual without the marker is outside the system's membership.
		let untouched = world
			.spawn((
				Transform::from_xyz(3.0, 4.0, 5.0),
				GlobalTransform::IDENTITY,
				TerrainPitch::new(RigSkeletonKind::Humanoid, 0.35, 0.2),
				CharacterHeading(Vec3::Z),
			))
			.id();
		let untouched_ticks = (
			change_tick::<Transform>(&world, untouched)?,
			change_tick::<TerrainPitch>(&world, untouched)?,
			change_tick::<CharacterHeading>(&world, untouched)?,
		);
		world.insert_resource(fixture);
		anyhow::ensure!(world.run_system_once(apply_checked_pitch).is_ok());
		let fixture = world.resource::<ProbeFixture>();
		let recordings =
			fixture.rays.lock().map_err(|_| anyhow::anyhow!("poisoned ray recorder"))?;
		assert_eq!(recordings.len(), characters.len());
		for (entity, case, placement, before, initial, heading, exclude) in characters {
			let pitch = world
				.get::<TerrainPitch>(entity)
				.ok_or_else(|| anyhow::anyhow!("missing pitch"))?;
			let visual = world
				.get::<Transform>(entity)
				.ok_or_else(|| anyhow::anyhow!("missing visual"))?;
			let facing = heading.facing(&before);
			let sample_facing = initial.sample_facing(facing);
			let right = Vec3::new(facing.z, 0.0, -facing.x);
			let mut points = vec![case.origin];
			if case.kind != 4 {
				let coarse = if case.kind == 0 || case.kind == 2 {
					0.0
				} else {
					TerrainPitch::observed_pitch(
						case.height(case.origin + sample_facing * initial.half_span),
						case.height(case.origin - sample_facing * initial.half_span),
						initial.half_span,
					)
				};
				let run = initial.pitched_half_run(coarse);
				points.extend([
					case.origin + sample_facing * initial.half_span,
					case.origin - sample_facing * initial.half_span,
					case.origin + sample_facing * run,
					case.origin - sample_facing * run,
				]);
				if initial.roll_weight > 0.0 {
					points.extend([
						case.origin - right * initial.half_width,
						case.origin + right * initial.half_width,
					]);
				}
			}
			let rays = recordings
				.get(&entity)
				.ok_or_else(|| anyhow::anyhow!("missing rays for {entity}"))?;
			assert_eq!(
				rays.len(),
				if case.kind == 4 {
					1
				} else if initial.roll_weight > 0.0 {
					7
				} else {
					5
				}
			);
			for (ray, point) in rays.iter().zip(&points) {
				let expected_origin =
					Vec3::new(point.x, probe_from_y(case.origin.y, initial.half_span), point.z);
				assert_eq!(
					ray,
					&RecordedRay {
						origin: expected_origin,
						max_distance: probe_distance(initial.half_span),
						exclude: exclude.clone(),
					},
					"ray signature differs for {entity}"
				);
			}
			assert_eq!(world.get::<CharacterHeading>(entity), Some(&CharacterHeading(facing)));
			assert_eq!(pitch.probe.visual_facing, facing);
			assert_eq!(pitch.probe.sample_facing, sample_facing);
			let sampled = |ray: usize| {
				let at = points[ray];
				Vec3::new(at.x, if case.hit(ray) { case.height(at) } else { case.origin.y }, at.z)
			};
			assert_eq!(pitch.probe.origin, sampled(0));
			let suspended = case.kind == 4;
			let front = if suspended { sampled(0) } else { sampled(3) };
			let hind = if suspended { sampled(0) } else { sampled(4) };
			assert_eq!(pitch.probe.front, front);
			assert_eq!(pitch.probe.hind, hind);
			assert_eq!(pitch.probe.front_hit, !suspended && case.hit(3));
			assert_eq!(pitch.probe.hind_hit, !suspended && case.hit(4));
			let (expected_pitch, expected_roll, expected_support) = if case.kind == 2
				|| case.kind == 3
			{
				(initial.pitch, initial.roll, initial.support)
			} else {
				let target_pitch = if case.kind == 1 {
					-(0.125 * sample_facing.x + 0.25 * sample_facing.z).atan()
				} else {
					0.0
				};
				let target_roll =
					if case.kind == 1 { (0.125 * right.x + 0.25 * right.z).atan() } else { 0.0 };
				(
					TerrainPitch::follow_target(initial.pitch, target_pitch, dt, TILT_RATE),
					TerrainPitch::follow_target(initial.roll, target_roll, dt, TILT_RATE),
					TerrainPitch::follow_target(initial.support, 0.0, dt, SUPPORT_RATE),
				)
			};
			assert!((pitch.pitch - expected_pitch).abs() < 1e-5);
			assert!((pitch.roll - expected_roll).abs() < 1e-5);
			assert!(
				(pitch.support - if placement == 0 { expected_support } else { initial.support })
					.abs() < 1e-5
			);
			assert_eq!(visual.translation.x, before.translation.x);
			assert_eq!(visual.translation.z, before.translation.z);
			assert_eq!(
				visual.translation.y,
				if placement == 0 { pitch.support } else { before.translation.y }
			);
			assert_eq!(visual.rotation, pitch.facing_with_support_tilt(facing, sample_facing));
		}
		assert_eq!(
			untouched_ticks,
			(
				change_tick::<Transform>(&world, untouched)?,
				change_tick::<TerrainPitch>(&world, untouched)?,
				change_tick::<CharacterHeading>(&world, untouched)?,
			)
		);
		assert_eq!(world.get::<Transform>(untouched), Some(&Transform::from_xyz(3.0, 4.0, 5.0)));
		Ok(())
	}
}
