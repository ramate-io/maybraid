//! Terrain pitch and roll on the character visual. Facing and the capsule stay as they are.

use bevy::ecs::query::Has;
use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;
use crozon_characters::{
	terrain_pitch::{
		facing_with_tilt, girdle_midpoint, half_span_from_girdles, half_width_from_sides,
		observed_pitch, observed_roll, step_toward, support_lift, QUADRUPED_FRONT, QUADRUPED_HIND,
		QUADRUPED_LEFT, QUADRUPED_RIGHT,
	},
	BoneMap, CharacterMembers, CharacterRig, CharacterRigRole, CharacterRoot, RigSkeletonKind,
	TerrainPitch,
};
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};

use crate::character::PlayerVisual;
use crate::player::{Jumping, Player};
use crate::WorldBaseTerrain;

pub(crate) fn prepare_terrain_pitch(
	mut commands: Commands,
	visuals: Query<
		(Entity, &CharacterMembers),
		(With<PlayerVisual>, With<CharacterRoot>, Without<TerrainPitch>),
	>,
	rigs: Query<(&CharacterRig, &BoneMap)>,
	globals: Query<&GlobalTransform>,
) {
	for (entity, members) in &visuals {
		let Some(kind) = body_skeleton(members, &rigs) else {
			continue;
		};
		let (front, hind, left, right) = supports(kind, members, &rigs, &globals);
		commands.entity(entity).insert(TerrainPitch::new(
			kind,
			half_span_from_girdles(kind, front, hind),
			half_width_from_sides(kind, left, right),
		));
	}
}

pub(crate) fn apply_terrain_pitch(
	time: Res<Time>,
	layout: Res<TerrainCellLayout>,
	store: Res<TerrainEntryStore>,
	base: Res<WorldBaseTerrain>,
	players: Query<(&Transform, Has<Jumping>), With<Player>>,
	mut visuals: Query<
		(&mut Transform, &mut TerrainPitch),
		(With<PlayerVisual>, With<CharacterRoot>, Without<Player>),
	>,
) {
	let Ok((player, jumping)) = players.single() else {
		return;
	};
	let Ok((mut visual, mut pitch)) = visuals.single_mut() else {
		return;
	};

	let facing = {
		let f = -visual.forward();
		Vec3::new(f.x, 0.0, f.z)
	};
	if facing.length_squared() < 1e-6 {
		return;
	}
	let facing = facing.normalize();
	let right = Vec3::new(facing.z, 0.0, -facing.x);
	let origin = player.translation;
	let front_xz = origin + facing * pitch.half_span;
	let hind_xz = origin - facing * pitch.half_span;
	let left_xz = origin - right * pitch.half_width;
	let right_xz = origin + right * pitch.half_width;

	let center_h = height_at(&store, &layout, &base, origin.x, origin.z);
	let front_h = height_at(&store, &layout, &base, front_xz.x, front_xz.z);
	let hind_h = height_at(&store, &layout, &base, hind_xz.x, hind_xz.z);
	let left_h = height_at(&store, &layout, &base, left_xz.x, left_xz.z);
	let right_h = height_at(&store, &layout, &base, right_xz.x, right_xz.z);

	let (target_pitch, target_roll) = if jumping {
		(0.0, 0.0)
	} else {
		(
			observed_pitch(front_h, hind_h, pitch.half_span) * pitch.weight,
			observed_roll(left_h, right_h, pitch.half_width) * pitch.weight,
		)
	};
	let dt = time.delta_secs();
	pitch.pitch = step_toward(pitch.pitch, target_pitch, dt);
	pitch.roll = step_toward(pitch.roll, target_roll, dt);
	visual.rotation = facing_with_tilt(facing, pitch.pitch, pitch.roll);
	visual.translation.y = if jumping {
		0.0
	} else {
		support_lift(
			origin.y,
			center_h,
			front_h,
			hind_h,
			left_h,
			right_h,
			pitch.half_span,
			pitch.half_width,
			pitch.pitch,
			pitch.roll,
		)
	};
}

fn body_skeleton(
	members: &CharacterMembers,
	rigs: &Query<(&CharacterRig, &BoneMap)>,
) -> Option<RigSkeletonKind> {
	for member in members.iter() {
		let Ok((rig, _)) = rigs.get(member) else {
			continue;
		};
		if rig.role == CharacterRigRole::Body {
			return Some(rig.skeleton);
		}
	}
	None
}

fn supports(
	kind: RigSkeletonKind,
	members: &CharacterMembers,
	rigs: &Query<(&CharacterRig, &BoneMap)>,
	globals: &Query<&GlobalTransform>,
) -> (Option<Vec3>, Option<Vec3>, Option<Vec3>, Option<Vec3>) {
	if kind != RigSkeletonKind::Quadruped {
		return (None, None, None, None);
	}
	for member in members.iter() {
		let Ok((rig, bones)) = rigs.get(member) else {
			continue;
		};
		if rig.role != CharacterRigRole::Body {
			continue;
		}
		return (
			girdle_midpoint(named_world(bones, QUADRUPED_FRONT, globals)),
			girdle_midpoint(named_world(bones, QUADRUPED_HIND, globals)),
			girdle_midpoint(named_world(bones, QUADRUPED_LEFT, globals)),
			girdle_midpoint(named_world(bones, QUADRUPED_RIGHT, globals)),
		);
	}
	(None, None, None, None)
}

fn named_world(bones: &BoneMap, names: &[&str], globals: &Query<&GlobalTransform>) -> Vec<Vec3> {
	names
		.iter()
		.filter_map(|name| {
			let entity = bones.by_name.get(*name)?;
			globals.get(*entity).ok().map(|g| g.translation())
		})
		.collect()
}

fn height_at(
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	base: &WorldBaseTerrain,
	x: f32,
	z: f32,
) -> f32 {
	store.composed_height_at(layout, x, z).unwrap_or_else(|| base.0.height_at(x, z))
}
