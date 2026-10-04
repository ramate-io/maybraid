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
use grenades::{grenade_material, grenade_mesh, GrenadeMaterial};
use player::{PlayerLook, PlayerUse};

use crate::throw::{
	look_forward, yaw_xz, GrenadePhase, GrenadeThrow, GrenadeUser, GrenadeUserSettings,
};

/// Preferred grip points. A later `hand_socket.R` on the body rig is picked first.
pub const RIGHT_HAND_SOCKETS: &[&str] = &["hand_socket.R", "hand.R", "palm.R"];

const HELD_SCALE: f32 = 0.2;
/// Body +X is left. Negative X wings the right elbow out laterally.
const THROW_POLE: Vec3 = Vec3::new(-1.0, 0.45, -0.2);
const THROW_POLE_FALLBACK: Vec3 = Vec3::new(-0.6, 0.8, 0.2);

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
	mut materials: ResMut<Assets<GrenadeMaterial>>,
	held: Query<Entity, (With<HeldGrenade>, Without<Mesh3d>)>,
) {
	for entity in &held {
		commands.entity(entity).insert((
			Mesh3d(meshes.add(grenade_mesh())),
			MeshMaterial3d(materials.add(grenade_material())),
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
		let overlay = users.get(child_of.parent()).is_ok();
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
		(&Transform, &CharacterHeading, &ChildOf),
		(With<CharacterRoot>, Without<HeldGrenade>, Without<AnimBone>),
	>,
	mut grenades: Query<&mut Transform, (With<HeldGrenade>, Without<CharacterRoot>)>,
) {
	for (visual, heading, child_of) in &visuals {
		let Ok((user, throw, look)) = users.get(child_of.parent()) else {
			continue;
		};
		if throw.busy() {
			continue;
		}
		let forward = primed_forward(heading.0, look);
		let right = Vec3::Y.cross(forward).normalize_or(Vec3::X);
		let Ok(mut transform) = grenades.get_mut(user.held) else {
			continue;
		};
		*transform = Transform {
			translation: visual.translation
				+ Vec3::Y * (1.28 + user.settings.hold_up)
				+ right * (0.08 + user.settings.hold_right)
				+ forward * (0.44 + user.settings.hold_forward),
			rotation: Quat::from_rotation_y(yaw_xz(forward)),
			scale: Vec3::splat(HELD_SCALE),
		};
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
	users: Query<(&GrenadeUser, &GrenadeThrow, Has<WeaponSwap>)>,
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
		let Ok((user, throw, swapping)) = users.get(child_of.parent()) else {
			continue;
		};
		if swapping {
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
		let _ = visual;
		for member in members.iter() {
			let Ok((mut rig, map, mailbox)) = rigs.get_mut(member) else {
				continue;
			};
			if mailbox.output.is_empty() {
				continue;
			}
			let target = clamp_throw_reach(throw_reach_from_shoulder(t));
			let mut bones = transforms.p1();
			rig.pose.clone_from(&mailbox.output);
			let arm = rig.arm_pose(Side::Right);
			let length = arm.forearm.transform.translation.length();
			if let Some(reach) = TwoBoneAim::reach(target, THROW_POLE, length, length)
				.or_else(|| TwoBoneAim::reach(target, THROW_POLE_FALLBACK, length, length))
			{
				reset_arm_to_rest(&mut rig, map, &bones, Side::Right);
				pose_throw_arm(&mut rig, reach);
			}
			write_throw_bones(&rig, map, &mut bones);
			drop(bones);
			let helper = transforms.p0();
			let hand = named_hand(map, &helper).or_else(|| distal_forearm(map, &helper));
			drop(helper);
			if let Some(hand) = hand {
				let forward = Vec3::new(heading.0.x, 0.0, heading.0.z).normalize_or(Vec3::Z);
				let mut grenades = transforms.p2();
				if let Ok(mut transform) = grenades.get_mut(user.held) {
					*transform = Transform {
						translation: hand + forward * 0.06 + Vec3::Y * 0.02,
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

fn primed_forward(facing: Vec3, look: &PlayerLook) -> Vec3 {
	if look.first_person {
		let aim = look_forward(look);
		return Vec3::new(aim.x, 0.0, aim.z).normalize_or(-Vec3::Z);
	}
	Vec3::new(facing.x, 0.0, facing.z).normalize_or(Vec3::Z)
}

/// Body-local reach from the right shoulder. `-X` is lateral, `+Z` is fight-forward.
fn throw_reach_from_shoulder(t: f32) -> Vec3 {
	const KEYS: [(f32, Vec3); 6] = [
		(0.00, Vec3::new(-0.10, 0.14, 0.42)),
		(0.22, Vec3::new(-0.16, 0.22, 0.28)),
		(0.45, Vec3::new(-0.22, 0.36, 0.16)),
		(0.61, Vec3::new(-0.08, 0.18, 0.46)),
		(0.82, Vec3::new(-0.08, 0.02, 0.42)),
		(1.00, Vec3::new(-0.10, 0.10, 0.38)),
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

fn clamp_throw_reach(mut reach: Vec3) -> Vec3 {
	reach.x = reach.x.min(-0.06);
	reach.z = reach.z.max(0.08);
	reach
}

fn pose_throw_arm(rig: &mut HumanoidV0Rig, reach: TwoBoneAim) {
	let roll = humerus_roll_for_reach(rig, reach);
	let mut posed = rig.arm_pose(Side::Right);
	posed.humerus = rig.humerus_along_with_roll(Side::Right, reach.upper_along, roll);
	rig.pose_arm(posed);
	let mut posed = rig.arm_pose(Side::Right);
	posed.forearm = rig.articulate_on_rig(posed.forearm, 0.0, reach.flex);
	rig.pose_arm(posed);
}

fn humerus_roll_for_reach(rig: &HumanoidV0Rig, reach: TwoBoneAim) -> f32 {
	let arm = rig.arm_pose(Side::Right);
	let humerus = rig.humerus_along_with_roll(Side::Right, reach.upper_along, 0.0);
	let forearm = rig.articulate_on_rig(arm.forearm, 0.0, reach.flex);
	let humerus_world = rig.parent_world_rotation(&humerus.name) * humerus.transform.rotation;
	let zero_roll_lower = forearm.transform.rotation * BONE_LENGTH_AXIS;
	let desired_lower = humerus_world.inverse() * reach.lower_along;
	signed_angle_about_axis(zero_roll_lower, desired_lower, BONE_LENGTH_AXIS).unwrap_or(0.0)
}

fn signed_angle_about_axis(from: Vec3, to: Vec3, axis: Vec3) -> Option<f32> {
	let axis = axis.try_normalize()?;
	let from = (from - axis * from.dot(axis)).try_normalize()?;
	let to = (to - axis * to.dot(axis)).try_normalize()?;
	Some(axis.dot(from.cross(to)).atan2(from.dot(to)))
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
	fn primed_hold_is_high_and_in_front() {
		let primed = throw_reach_from_shoulder(0.0);
		assert!(
			primed.x < -0.06 && primed.x > -0.16,
			"primed hold stays slightly out, got {primed:?}"
		);
		assert!(primed.y > 0.0, "primed hold is chest-high, got {primed:?}");
		assert!(primed.z > 0.36, "primed hold is in front, got {primed:?}");
	}

	#[test]
	fn throw_reach_stays_lateral_and_forward() {
		for t in [0.0, 0.22, 0.45, 0.61, 0.82, 1.0] {
			let reach = clamp_throw_reach(throw_reach_from_shoulder(t));
			assert!(reach.x < 0.0, "t={t} crossed the body, got {reach:?}");
			assert!(reach.z > 0.0, "t={t} went behind, got {reach:?}");
		}
	}

	#[test]
	fn throw_swing_raises_then_snaps_forward() {
		let start = throw_reach_from_shoulder(0.0);
		let cock = throw_reach_from_shoulder(0.45);
		let release = throw_reach_from_shoulder(0.61);
		assert!(cock.y > start.y + 0.15, "must raise, start={start:?} cock={cock:?}");
		assert!(release.z > cock.z + 0.2, "must snap forward, cock={cock:?} release={release:?}");
	}

	#[test]
	fn right_elbow_pole_wings_out() {
		let reach = TwoBoneAim::reach(throw_reach_from_shoulder(0.0), THROW_POLE, 0.35, 0.35)
			.expect("primed reach");
		assert!(reach.upper_along.x < 0.0, "elbow must wing out, got {:?}", reach.upper_along);
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
