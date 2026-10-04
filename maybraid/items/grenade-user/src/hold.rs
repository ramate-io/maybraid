//! Hand-held pose and throw-arm overlay.

use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use character_inventory_user::InventoryUser;
use character_items::{Inventory, InventoryItem};
use character_rigs::articulation::TwoBoneAim;
use character_rigs::humanoid::HumanoidRig;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;
use characters::{
	AnimBone, AnimMailbox, AnimateBones, BoneMap, CharacterHeading, CharacterMembers, CharacterRig,
	CharacterRigRole, CharacterRoot, RigSkeletonKind, SuspendAnimation,
};
use firearm_user::{HoldingArms, WeaponSwap};
use grenades::grenade_mesh;
use player::{PlayerLook, PlayerUse};

use crate::throw::{GrenadePhase, GrenadeThrow, GrenadeUser, GrenadeUserSettings};
use character_rigs::Name as RigName;

const HELD_SCALE: f32 = 0.16;

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
	mut grenades: Query<&mut Transform, (With<HeldGrenade>, Without<CharacterRoot>)>,
	helper: TransformHelper,
) {
	for (visual, heading, members, child_of) in &visuals {
		let Ok((user, throw, look)) = users.get(child_of.parent()) else {
			continue;
		};
		if matches!(throw.phase, GrenadePhase::Recovery { .. }) {
			continue;
		}
		let Ok(mut transform) = grenades.get_mut(user.held) else {
			continue;
		};
		let facing = heading.0;
		let yaw = if look.first_person { look.yaw } else { yaw_xz(facing) };
		let origin = visual.translation;
		let hand = right_hand(members, &maps, &helper)
			.unwrap_or_else(|| fallback_hold(origin, facing, &user.settings));
		let point = hold_point(origin, look, throw.phase, hand, &user.settings);
		*transform = Transform {
			translation: point,
			rotation: Quat::from_rotation_y(yaw),
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
	users: Query<(&GrenadeUser, &GrenadeThrow, &PlayerLook, Has<WeaponSwap>)>,
	visuals: Query<(Entity, &CharacterMembers, &ChildOf), (With<CharacterRoot>, Without<AnimBone>)>,
	held: Query<&Transform, With<HeldGrenade>>,
	mut rigs: Query<
		(&mut HumanoidV0Rig, &BoneMap, &AnimMailbox),
		(With<HoldingGrenade>, With<AnimateBones>, Without<SuspendAnimation>),
	>,
	mut transforms: ParamSet<(
		TransformHelper,
		Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<CharacterRoot>)>,
	)>,
) {
	for (visual, members, child_of) in &visuals {
		let Ok((user, throw, look, swapping)) = users.get(child_of.parent()) else {
			continue;
		};
		if swapping || !throw.busy() {
			continue;
		}
		let Ok(held_transform) = held.get(user.held) else {
			continue;
		};
		let helper = transforms.p0();
		let Some(body_rot) = helper.compute_global_transform(visual).ok().map(|t| t.rotation())
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
			let target = bone_world(map, &helper, "humerus.R").and_then(|from| {
				let dir = held_transform.translation - from;
				(dir.length_squared() >= 1e-6).then(|| body_rot.inverse() * dir)
			});
			drop(helper);
			rig.pose.clone_from(&mailbox.output);
			if let Some(target) = target {
				let pole = Vec3::new(-1.0, -0.4, 0.2);
				let arm = rig.arm_pose(Side::Right);
				let length = arm.forearm.transform.translation.length();
				if let Some(reach) = TwoBoneAim::reach(target, pole, length, length) {
					let mut posed = rig.arm_pose(Side::Right);
					posed.humerus = rig.humerus_along_with_roll(Side::Right, reach.upper_along, 1.2);
					rig.pose_arm(posed);
					let mut posed = rig.arm_pose(Side::Right);
					posed.forearm = rig.articulate_on_rig(posed.forearm, 0.0, reach.flex);
					rig.pose_arm(posed);
				}
			} else {
				let _ = look;
			}
			let mut bones = transforms.p1();
			write_throw_bones(&rig, map, &mut bones);
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

fn fallback_hold(origin: Vec3, facing: Vec3, settings: &GrenadeUserSettings) -> Vec3 {
	origin + Vec3::Y * (1.1 + settings.hold_up) + facing * settings.hold_forward
}

fn hold_point(
	origin: Vec3,
	look: &PlayerLook,
	phase: GrenadePhase,
	hand: Vec3,
	settings: &GrenadeUserSettings,
) -> Vec3 {
	let yaw = Quat::from_rotation_y(look.yaw);
	let pitch = Quat::from_rotation_x(-look.pitch);
	let aim = (yaw * pitch) * -Vec3::Z;
	let right = yaw * Vec3::X;
	match phase {
		GrenadePhase::Ready => {
			hand + Vec3::Y * settings.hold_up + right * settings.hold_right
		}
		GrenadePhase::Windup { age } => {
			let t = (age / 0.25).clamp(0.0, 1.0);
			origin + Vec3::Y * (1.2 + 0.28 * t) + right * 0.12 + aim * (-0.04 + 0.18 * t)
		}
		GrenadePhase::Recovery { age } => {
			let t = ((age - 0.25) / 0.35).clamp(0.0, 1.0);
			origin + Vec3::Y * (1.35 - 0.2 * t) + right * 0.16 + aim * (0.15 + 0.25 * t)
		}
	}
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
		for name in ["forearm.R", "humerus.R"] {
			if let Some(&entity) = map.by_name.get(name) {
				if let Ok(global) = helper.compute_global_transform(entity) {
					return Some(global.translation());
				}
			}
		}
	}
	None
}

fn bone_world(map: &BoneMap, helper: &TransformHelper, name: &str) -> Option<Vec3> {
	let entity = *map.by_name.get(name)?;
	helper.compute_global_transform(entity).ok().map(|global| global.translation())
}

fn yaw_xz(dir: Vec3) -> f32 {
	let xz = Vec3::new(dir.x, 0.0, dir.z);
	if xz.length_squared() < 1e-8 {
		0.0
	} else {
		let n = xz.normalize();
		n.x.atan2(n.z)
	}
}

fn write_throw_bones(
	rig: &HumanoidV0Rig,
	map: &BoneMap,
	bones: &mut Query<(&AnimBone, &mut Transform), (Without<AnimMailbox>, Without<CharacterRoot>)>,
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
