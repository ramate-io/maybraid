//! Hand-held pose and throw-arm overlay.

use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use character_inventory_user::InventoryUser;
use character_items::{GrenadeStats, Inventory, InventoryItem};
use character_rigs::articulation::{TwoBoneAim, BONE_LENGTH_AXIS};
use character_rigs::humanoid::HumanoidRig;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::{Name as RigName, Side};
use characters::{
	AnimBone, AnimMailbox, AnimateBones, BoneMap, CharacterHeading, CharacterMembers, CharacterRig,
	CharacterRigRole, CharacterRoot, RigSkeletonKind, SuspendAnimation,
};
use firearm_user::{HoldingArms, WeaponSwap};
use grenades::grenade_mesh;
use player::{PlayerLook, PlayerUse};
use std::f32::consts::FRAC_PI_2;

use crate::throw::{
	throw_aim, yaw_xz, GrenadePhase, GrenadeThrow, GrenadeUser, GrenadeUserSettings,
};

/// Preferred grip points. A later `hand_socket.R` on the body rig is picked first.
pub const RIGHT_HAND_SOCKETS: &[&str] = &["hand_socket.R", "hand.R", "palm.R"];

const HELD_SCALE: f32 = 0.2;
/// Elbow out to the right so the overhand does not fold over the head.
const THROW_POLE: Vec3 = Vec3::new(1.0, 0.4, -0.2);
const THROW_POLE_FALLBACK: Vec3 = Vec3::new(0.6, 0.8, 0.2);

#[derive(Component, Clone, Copy, Debug)]
pub struct HeldGrenade {
	pub scale: f32,
}

#[derive(Component, Debug)]
pub struct HoldingGrenade;

pub fn spawn_held_grenade(
	commands: &mut Commands,
	user: Entity,
	settings: GrenadeUserSettings,
) -> Entity {
	let held = commands
		.spawn((
			bevy::prelude::Name::new("held-grenade"),
			Transform::from_scale(Vec3::splat(HELD_SCALE)),
			Visibility::Visible,
			HeldGrenade { scale: HELD_SCALE },
		))
		.id();
	commands.entity(user).insert((
		GrenadeUser { held, settings },
		GrenadeThrow::default(),
		PlayerUse { driver: held },
	));
	held
}

pub fn realize_held_grenade_visuals(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	held: Query<Entity, (With<HeldGrenade>, Without<Mesh3d>)>,
) {
	for entity in &held {
		commands.entity(entity).insert((
			Mesh3d(meshes.add(grenade_mesh())),
			MeshMaterial3d(materials.add(StandardMaterial {
				base_color: Color::srgb(0.28, 0.34, 0.18),
				perceptual_roughness: 0.72,
				metallic: 0.18,
				..default()
			})),
		));
	}
}

pub fn stamp_holding_grenade(
	mut commands: Commands,
	users: Query<&GrenadeThrow, With<GrenadeUser>>,
	visuals: Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	rigs: Query<(Entity, &CharacterRig, Has<HoldingGrenade>, Has<HoldingArms>)>,
) {
	for (members, child_of) in &visuals {
		let overlay = users.get(child_of.parent()).ok().is_some_and(GrenadeThrow::busy);
		for member in members.iter() {
			let Ok((entity, rig, holding, armed)) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body || rig.skeleton != RigSkeletonKind::Humanoid {
				continue;
			}
			if overlay && !holding {
				commands.entity(entity).insert(HoldingGrenade);
			}
			if overlay && armed {
				commands.entity(entity).remove::<HoldingArms>();
			}
			if !overlay && holding {
				commands.entity(entity).remove::<HoldingGrenade>();
			}
		}
	}
}

pub fn pose_held_grenade(
	users: Query<(&GrenadeUser, &GrenadeThrow, &PlayerLook)>,
	visuals: Query<
		(&Transform, &CharacterHeading, &CharacterMembers, &ChildOf),
		(With<CharacterRoot>, Without<HeldGrenade>, Without<AnimBone>),
	>,
	maps: Query<&BoneMap, Without<HeldGrenade>>,
	mut transforms: ParamSet<(
		TransformHelper,
		Query<&mut Transform, (With<HeldGrenade>, Without<CharacterRoot>)>,
	)>,
) {
	for (visual, heading, members, child_of) in &visuals {
		let Ok((user, throw, look)) = users.get(child_of.parent()) else {
			continue;
		};
		if throw.busy() {
			continue;
		}
		let helper = transforms.p0();
		let Some(hand) = right_hand(members, &maps, &helper) else {
			drop(helper);
			continue;
		};
		drop(helper);
		let facing = heading.0;
		let yaw = if look.first_person { look.yaw } else { yaw_xz(facing) };
		let mut grenades = transforms.p1();
		let Ok(mut transform) = grenades.get_mut(user.held) else {
			continue;
		};
		*transform = Transform {
			translation: hand
				+ Vec3::Y * user.settings.hold_up
				+ Quat::from_rotation_y(yaw) * Vec3::X * user.settings.hold_right,
			rotation: Quat::from_rotation_y(yaw),
			scale: Vec3::splat(HELD_SCALE),
		};
		let _ = visual;
	}
}

pub fn apply_grenade_swap_pose(
	users: Query<(&GrenadeUser, &WeaponSwap)>,
	mut grenades: Query<&mut Transform, With<HeldGrenade>>,
) {
	for (user, swap) in &users {
		if let Ok(mut transform) = grenades.get_mut(user.held) {
			swap.apply_to(&mut transform);
		}
	}
}

pub fn sync_throw_arm(
	users: Query<(&GrenadeUser, &GrenadeThrow, &PlayerLook, Has<WeaponSwap>)>,
	carriers: Query<&InventoryUser>,
	bags: Query<&Inventory>,
	visuals: Query<
		(Entity, &CharacterHeading, &CharacterMembers, &ChildOf),
		(With<CharacterRoot>, Without<AnimBone>),
	>,
	mut rigs: Query<
		(&mut HumanoidV0Rig, &BoneMap, &AnimMailbox),
		(With<HoldingGrenade>, With<AnimateBones>, Without<SuspendAnimation>),
	>,
	mut transforms: ParamSet<(
		TransformHelper,
		Query<
			(&AnimBone, &mut Transform),
			(Without<AnimMailbox>, Without<CharacterRoot>, Without<HeldGrenade>),
		>,
		Query<&mut Transform, (With<HeldGrenade>, Without<CharacterRoot>)>,
	)>,
) {
	for (visual, heading, members, child_of) in &visuals {
		let Ok((user, throw, look, swapping)) = users.get(child_of.parent()) else {
			continue;
		};
		if swapping || !throw.busy() {
			continue;
		}
		let throw_secs = carriers
			.get(child_of.parent())
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_stats))
			.map(|stats| stats.throw_secs)
			.unwrap_or_else(|| GrenadeStats::standard().throw_secs);
		let t = throw_phase_t(throw.phase, throw_secs);
		let helper = transforms.p0();
		let Some(body_rot) = helper.compute_global_transform(visual).ok().map(|tf| tf.rotation())
		else {
			continue;
		};
		drop(helper);
		for member in members.iter() {
			let Ok((mut rig, map, mailbox)) = rigs.get_mut(member) else {
				continue;
			};
			if mailbox.output.is_empty() {
				continue;
			}
			let helper = transforms.p0();
			let target = bone_world(map, &helper, "humerus.R").and_then(|shoulder| {
				let world = throw_hand_from_shoulder(shoulder, heading.0, look, t);
				let dir = world - shoulder;
				(dir.length_squared() >= 1e-6).then(|| body_rot.inverse() * dir)
			});
			drop(helper);
			let mut bones = transforms.p1();
			rig.pose.clone_from(&mailbox.output);
			if let Some(target) = target {
				let arm = rig.arm_pose(Side::Right);
				let length = arm.forearm.transform.translation.length();
				if let Some(reach) = TwoBoneAim::reach(target, THROW_POLE, length, length)
					.or_else(|| TwoBoneAim::reach(target, THROW_POLE_FALLBACK, length, length))
				{
					reset_arm_to_rest(&mut rig, map, &bones, Side::Right);
					pose_throw_arm(&mut rig, reach);
				}
			}
			write_throw_bones(&rig, map, &mut bones);
			drop(bones);
			let helper = transforms.p0();
			let hand = named_hand(map, &helper).or_else(|| distal_forearm(map, &helper));
			drop(helper);
			if let Some(hand) = hand {
				let mut grenades = transforms.p2();
				if let Ok(mut transform) = grenades.get_mut(user.held) {
					*transform = Transform {
						translation: hand,
						rotation: Quat::from_rotation_y(yaw_xz(heading.0)),
						scale: Vec3::splat(HELD_SCALE),
					};
				}
			}
		}
	}
}

pub fn sync_held_visibility(
	carriers: Query<&InventoryUser>,
	bags: Query<&Inventory>,
	users: Query<(Entity, &GrenadeUser, &GrenadeThrow)>,
	mut held: Query<&mut Visibility, With<HeldGrenade>>,
) {
	for (entity, user, throw) in &users {
		let Ok(mut visibility) = held.get_mut(user.held) else {
			continue;
		};
		let ready = carriers
			.get(entity)
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(Inventory::primary_weapon)
			.is_some_and(|item| match item {
				InventoryItem::Grenade { recharge, .. } => recharge.ready(),
				_ => false,
			});
		*visibility = if ready && !matches!(throw.phase, GrenadePhase::Recovery { .. }) {
			Visibility::Visible
		} else {
			Visibility::Hidden
		};
	}
}

fn throw_phase_t(phase: GrenadePhase, throw_secs: f32) -> f32 {
	match phase {
		GrenadePhase::Ready => 0.0,
		GrenadePhase::Windup { age } | GrenadePhase::Recovery { age } => {
			(age / throw_secs.max(1e-3)).clamp(0.0, 1.0)
		}
	}
}

/// Shoulder-local overhand: `(right, up, forward)` in heading space.
fn throw_reach_from_shoulder(t: f32) -> Vec3 {
	const KEYS: [(f32, Vec3); 6] = [
		(0.00, Vec3::new(0.20, -0.22, 0.22)),
		(0.22, Vec3::new(0.24, 0.02, 0.16)),
		(0.45, Vec3::new(0.28, 0.32, 0.10)),
		(0.61, Vec3::new(0.12, 0.24, 0.40)),
		(0.82, Vec3::new(0.10, 0.02, 0.42)),
		(1.00, Vec3::new(0.14, -0.16, 0.24)),
	];
	let t = t.clamp(0.0, 1.0);
	for window in KEYS.windows(2) {
		let (t0, a) = window[0];
		let (t1, b) = window[1];
		if t <= t1 {
			let u = ((t - t0) / (t1 - t0).max(1e-4)).clamp(0.0, 1.0);
			return a.lerp(b, u);
		}
	}
	KEYS[KEYS.len() - 1].1
}

fn throw_hand_from_shoulder(shoulder: Vec3, facing: Vec3, look: &PlayerLook, t: f32) -> Vec3 {
	let aim = throw_aim(facing, look);
	let flat = Vec3::new(aim.x, 0.0, aim.z).normalize_or(Vec3::Z);
	let right = Vec3::Y.cross(flat).normalize_or(Vec3::X);
	let reach = throw_reach_from_shoulder(t);
	let mut world = shoulder + right * reach.x + Vec3::Y * reach.y + flat * reach.z;
	let along = (world - shoulder).dot(flat);
	if along < 0.08 {
		world += flat * (0.08 - along);
	}
	world
}

fn pose_throw_arm(rig: &mut HumanoidV0Rig, reach: TwoBoneAim) {
	let mut posed = rig.arm_pose(Side::Right);
	posed.humerus = rig.humerus_along_with_roll(Side::Right, reach.upper_along, FRAC_PI_2);
	rig.pose_arm(posed);
	let mut posed = rig.arm_pose(Side::Right);
	posed.forearm = rig.articulate_on_rig(posed.forearm, 0.0, reach.flex);
	rig.pose_arm(posed);
}

fn reset_arm_to_rest(
	rig: &mut HumanoidV0Rig,
	map: &BoneMap,
	bones: &Query<
		(&AnimBone, &mut Transform),
		(Without<AnimMailbox>, Without<CharacterRoot>, Without<HeldGrenade>),
	>,
	side: Side,
) {
	let mut arm = rig.arm(side);
	for pose in [&mut arm.shoulder, &mut arm.humerus, &mut arm.forearm] {
		let Some(&entity) = map.by_name.get(pose.name.as_str()) else {
			continue;
		};
		let Ok((bone, _)) = bones.get(entity) else {
			continue;
		};
		pose.transform = bone.rest;
		pose.swing = 0.0;
		pose.flex = 0.0;
		pose.twist = 0.0;
	}
	rig.pose_arm(arm);
}

fn right_hand(
	members: &CharacterMembers,
	maps: &Query<&BoneMap, Without<HeldGrenade>>,
	helper: &TransformHelper,
) -> Option<Vec3> {
	for member in members.iter() {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		if let Some(hand) = named_hand(map, helper).or_else(|| distal_forearm(map, helper)) {
			return Some(hand);
		}
	}
	None
}

fn named_hand(map: &BoneMap, helper: &TransformHelper) -> Option<Vec3> {
	for name in RIGHT_HAND_SOCKETS {
		if let Some(&entity) = map.by_name.get(*name) {
			if let Ok(global) = helper.compute_global_transform(entity) {
				return Some(global.translation());
			}
		}
	}
	None
}

fn distal_forearm(map: &BoneMap, helper: &TransformHelper) -> Option<Vec3> {
	let &forearm = map.by_name.get("forearm.R")?;
	let global = helper.compute_global_transform(forearm).ok()?;
	let length = map
		.by_name
		.get("humerus.R")
		.and_then(|&humerus| helper.compute_global_transform(humerus).ok())
		.map(|humerus| humerus.translation().distance(global.translation()))
		.unwrap_or(0.28);
	Some(global.translation() + global.rotation() * (BONE_LENGTH_AXIS * length))
}

fn bone_world(map: &BoneMap, helper: &TransformHelper, name: &str) -> Option<Vec3> {
	let entity = *map.by_name.get(name)?;
	helper.compute_global_transform(entity).ok().map(|global| global.translation())
}

fn write_throw_bones(
	rig: &HumanoidV0Rig,
	map: &BoneMap,
	bones: &mut Query<
		(&AnimBone, &mut Transform),
		(Without<AnimMailbox>, Without<CharacterRoot>, Without<HeldGrenade>),
	>,
) {
	let arm = rig.arm(Side::Right);
	for name in [arm.shoulder.name.as_str(), arm.humerus.name.as_str(), arm.forearm.name.as_str()] {
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		let Some(pose) = rig.pose.get(&RigName::from(name)) else {
			continue;
		};
		let Ok((_, mut transform)) = bones.get_mut(entity) else {
			continue;
		};
		*transform = pose.transform;
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use super::*;

	#[test]
	fn pose_queries_do_not_alias_transform() {
		let mut world = World::new();
		world.run_system_once(pose_held_grenade).expect("pose");
	}

	#[test]
	fn throw_arm_queries_do_not_alias_transform() {
		let mut world = World::new();
		world.run_system_once(sync_throw_arm).expect("overlay");
	}

	#[test]
	fn throw_target_stays_in_front_of_heading() {
		let look = PlayerLook::default();
		let shoulder = Vec3::new(0.2, 1.4, 0.0);
		for t in [0.0, 0.22, 0.45, 0.61, 0.82, 1.0] {
			let target = throw_hand_from_shoulder(shoulder, Vec3::Z, &look, t);
			assert!(target.z > shoulder.z, "t={t} must stay in front, got {target:?}");
		}
	}

	#[test]
	fn throw_swing_raises_then_snaps_forward() {
		let start = throw_reach_from_shoulder(0.0);
		let cock = throw_reach_from_shoulder(0.45);
		let release = throw_reach_from_shoulder(0.61);
		assert!(cock.y > start.y + 0.35, "must raise, start={start:?} cock={cock:?}");
		assert!(release.z > cock.z + 0.2, "must snap forward, cock={cock:?} release={release:?}");
	}

	#[test]
	fn throw_phase_uses_full_throw_window() {
		let stats = GrenadeStats::standard();
		assert!(
			(throw_phase_t(GrenadePhase::Windup { age: stats.release_at }, stats.throw_secs)
				- stats.release_at / stats.throw_secs)
				.abs() < 1e-4
		);
		assert_eq!(
			throw_phase_t(GrenadePhase::Recovery { age: stats.throw_secs }, stats.throw_secs),
			1.0
		);
	}
}
