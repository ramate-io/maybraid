//! Apply [`SkillMapFlick`] throws to the map camera and bead the stroke.

use bevy::prelude::*;

use crate::controller::SkillMapFlick;
use crate::map::{render_layer, SkillMapId};
use crate::tile_material::SkillMapTileAssets;
use crate::user::{SkillMapHeld, SkillMapMember, SkillMapSteerLock, SkillMapUser};
use crate::viewport::SkillMapViewportCamera;
use crate::SkillMapEnabled;

pub const CURSOR_SPEED: f32 = 64.0;
pub const WATER_LOCK_SECS: f32 = 2.0;
/// World units a unit-length flick travels.
pub const FLICK_TRAVEL: f32 = 72.0;
const BEAD_SPACING: f32 = 5.5;
const BEAD_LIFE: f32 = 0.55;
const BEAD_START: f32 = 0.42;
const BEAD_END: f32 = 0.14;

#[derive(Component)]
pub struct SkillMapCursor;

#[derive(Component)]
pub(crate) struct FlickBead {
	age: f32,
	max_age: f32,
	start_scale: f32,
}

pub fn sync_skill_map_held(
	enabled: Res<SkillMapEnabled>,
	mut users: Query<&mut SkillMapHeld, With<SkillMapUser>>,
) {
	for mut held in &mut users {
		held.0 = enabled.0;
	}
}

pub fn tick_steer_lock(
	time: Res<Time>,
	mut locks: Query<&mut SkillMapSteerLock, With<SkillMapUser>>,
) {
	let dt = time.delta_secs();
	for mut lock in &mut locks {
		if lock.remaining > 0.0 {
			lock.remaining = (lock.remaining - dt).max(0.0);
		}
	}
}

pub fn apply_flicks(
	mut commands: Commands,
	assets: Option<Res<SkillMapTileAssets>>,
	enabled: Res<SkillMapEnabled>,
	mut flicks: MessageReader<SkillMapFlick>,
	users: Query<(&SkillMapUser, &SkillMapSteerLock)>,
	mut cameras: Query<
		(&SkillMapId, &SkillMapMember, &mut Transform),
		(With<SkillMapViewportCamera>, Without<SkillMapCursor>),
	>,
) {
	if !enabled.0 {
		for _ in flicks.read() {}
		return;
	}
	let Some(assets) = assets else {
		for _ in flicks.read() {}
		return;
	};
	for flick in flicks.read() {
		if flick.0 == Vec2::ZERO {
			continue;
		}
		for (map, member, mut transform) in &mut cameras {
			let Some((user, lock)) = users.iter().find(|(user, _)| user.maps == member.session)
			else {
				continue;
			};
			if lock.locked() {
				continue;
			}
			let start = transform.translation.xy();
			let delta = flick.0 * user.settings.flick_travel;
			transform.translation.x += delta.x;
			transform.translation.y += delta.y;
			bead_flick(
				&mut commands,
				&assets,
				*map,
				*member,
				start,
				start + delta,
				transform.translation.z,
			);
		}
	}
}

fn bead_flick(
	commands: &mut Commands,
	assets: &SkillMapTileAssets,
	map: SkillMapId,
	member: SkillMapMember,
	start: Vec2,
	end: Vec2,
	z: f32,
) {
	let span = end - start;
	let length = span.length();
	if length < 0.5 {
		return;
	}
	let dir = span / length;
	let count = ((length / BEAD_SPACING).floor() as u32).saturating_add(1).max(2);
	for i in 0..count {
		let t = i as f32 / (count - 1) as f32;
		let p = start + dir * (length * t);
		commands.spawn((
			Name::new("skill-map-flick-bead"),
			FlickBead { age: 0.0, max_age: BEAD_LIFE, start_scale: BEAD_START },
			map,
			member,
			Mesh2d(assets.bead_mesh.clone()),
			MeshMaterial2d(assets.cursor.clone()),
			Transform::from_xyz(p.x, p.y, z - 0.2).with_scale(Vec3::splat(BEAD_START)),
			render_layer(map),
		));
	}
}

pub fn tick_flick_beads(
	time: Res<Time>,
	mut commands: Commands,
	mut beads: Query<(Entity, &mut FlickBead, &mut Transform)>,
) {
	let dt = time.delta_secs();
	for (entity, mut bead, mut transform) in &mut beads {
		bead.age = (bead.age + dt).min(bead.max_age);
		let u = bead.age / bead.max_age;
		let scale = bead.start_scale.lerp(BEAD_END, u);
		transform.scale = Vec3::splat(scale);
		if bead.age >= bead.max_age {
			commands.entity(entity).despawn();
		}
	}
}

