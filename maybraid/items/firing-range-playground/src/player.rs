//! Capsule player + third-person follow. Intents come from the character controller.

use avian3d::prelude::*;
use bevy::ecs::query::Has;
use bevy::prelude::*;
use crozon_characters::{BoneMap, CharacterMembers, CharacterPartSlot, PartNode};
use firearms::{FirearmMembers, FirearmRoot};
use lod_avian::PhysicsInteractionLayer;
use std::f32::consts::PI;

use crate::camera::{CameraController, CameraPov};
use crate::character::{HeldFirearm, PlayerVisual};

pub(crate) const CAPSULE_RADIUS: f32 = 0.4;
pub(crate) const CAPSULE_LENGTH: f32 = 1.0;
const MOVE_ACCEL: f32 = 40.0;
const MOVE_DAMPING: f32 = 0.92;
const JUMP_IMPULSE: f32 = 8.0;
const MAX_SLOPE_ANGLE: f32 = PI * 0.45;
pub(crate) const CAMERA_DISTANCE: f32 = 3.6;
pub(crate) const CAMERA_HEIGHT: f32 = 1.1;
pub(crate) const CAMERA_LOOK_HEIGHT: f32 = 0.65;
/// Shift the look target right so the character composes left of the reticle.
const CAMERA_SHOULDER_OFFSET: f32 = 0.7;
const FIRST_PERSON_EYE_FORWARD: f32 = 0.04;
const FOCUS_BLEND_SPEED: f32 = 12.0;
/// Sit behind the sight so the camera is not inside the optic.
const SIGHT_CAMERA_BACK: f32 = 0.05;
const GROUND_CAST_DISTANCE: f32 = 0.45;
const GROUND_SNAP_SPEED: f32 = 1.5;

#[derive(Component, Default)]
pub(crate) struct MoveWish(pub Vec3);

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PlayerControlSystems;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub(crate) struct CameraFollow;

#[derive(Component)]
pub struct PlayerCapsule;

#[derive(Component)]
pub(crate) struct CharacterController;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub(crate) struct Grounded;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub(crate) struct Jumping {
	left_ground: bool,
}

#[derive(Component)]
struct MovementAcceleration(f32);

#[derive(Component)]
struct MovementDampingFactor(f32);

#[derive(Component)]
struct JumpImpulse(f32);

#[derive(Component)]
struct MaxSlopeAngle(f32);

#[derive(Message)]
pub(crate) enum MovementAction {
	Move(Vec2),
	Jump,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<MovementAction>().add_systems(
			Update,
			(update_grounded, apply_character_movement, apply_movement_damping)
				.chain()
				.in_set(PlayerControlSystems),
		);
	}
}

pub(crate) fn spawn_player(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let spawn = Vec3::new(0.0, CAPSULE_RADIUS + CAPSULE_LENGTH * 0.5 + 0.15, 0.0);
	let player = spawn_character_controller(&mut commands, spawn);
	commands.entity(player).insert((Name::new("Player"), Player, CameraFollow));
	commands.spawn((
		Name::new("PlayerCapsule"),
		PlayerCapsule,
		ChildOf(player),
		Visibility::Hidden,
		Mesh3d(meshes.add(Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH))),
		MeshMaterial3d(materials.add(Color::srgb(0.85, 0.55, 0.35))),
	));
}

fn spawn_character_controller(commands: &mut Commands, translation: Vec3) -> Entity {
	let collider = Collider::capsule(CAPSULE_RADIUS, CAPSULE_LENGTH);
	let mut caster_shape = collider.clone();
	caster_shape.set_scale(Vec3::splat(0.99), 10);
	commands
		.spawn((
			CharacterController,
			Transform::from_translation(translation),
			Visibility::default(),
			RigidBody::Dynamic,
			collider,
			PhysicsInteractionLayer::animated_layers(),
			ShapeCaster::new(caster_shape, Vec3::ZERO, Quat::IDENTITY, Dir3::NEG_Y)
				.with_max_distance(GROUND_CAST_DISTANCE)
				.with_query_filter(SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)),
			LockedAxes::ROTATION_LOCKED,
		))
		.insert((
			MovementAcceleration(MOVE_ACCEL),
			MovementDampingFactor(MOVE_DAMPING),
			JumpImpulse(JUMP_IMPULSE),
			MaxSlopeAngle(MAX_SLOPE_ANGLE),
			MoveWish::default(),
			Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
			Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
			GravityScale(1.25),
		))
		.id()
}

fn update_grounded(
	mut commands: Commands,
	mut query: Query<
		(
			Entity,
			&ShapeHits,
			&LinearVelocity,
			Option<&MaxSlopeAngle>,
			Has<Grounded>,
			Option<&mut Jumping>,
		),
		With<CharacterController>,
	>,
) {
	for (entity, hits, velocity, max_slope_angle, was_grounded, jumping) in &mut query {
		let mut is_grounded = hits.iter().any(|hit| {
			if let Some(angle) = max_slope_angle {
				(-hit.normal2).angle_between(Vec3::Y).abs() <= angle.0
			} else {
				true
			}
		});
		if !is_grounded
			&& was_grounded
			&& jumping.is_none()
			&& velocity.y > -GROUND_SNAP_SPEED
			&& velocity.y < GROUND_SNAP_SPEED
		{
			is_grounded = true;
		}
		let landed = jumping.as_ref().is_some_and(|jump| jump.left_ground);
		if is_grounded {
			commands.entity(entity).insert(Grounded);
			if landed {
				commands.entity(entity).remove::<Jumping>();
			}
		} else {
			commands.entity(entity).remove::<Grounded>();
			if let Some(mut jump) = jumping {
				jump.left_ground = true;
			}
		}
	}
}

fn apply_character_movement(
	mut commands: Commands,
	time: Res<Time>,
	cameras: Query<&CameraController, With<Camera3d>>,
	mut reader: MessageReader<MovementAction>,
	mut controllers: Query<
		(Entity, &MovementAcceleration, &JumpImpulse, &mut LinearVelocity, Has<Grounded>),
		With<CharacterController>,
	>,
) {
	let Ok(camera) = cameras.single() else {
		for _ in reader.read() {}
		return;
	};

	let yaw = Quat::from_axis_angle(Vec3::Y, camera.yaw);
	let forward = yaw * -Vec3::Z;
	let right = yaw * Vec3::X;
	let dt = time.delta_secs();

	for action in reader.read() {
		for (entity, accel, jump, mut velocity, grounded) in &mut controllers {
			match action {
				MovementAction::Move(direction) => {
					let wish = (right * direction.x + forward * direction.y).normalize_or_zero();
					velocity.x += wish.x * accel.0 * dt;
					velocity.z += wish.z * accel.0 * dt;
				}
				MovementAction::Jump => {
					if grounded {
						velocity.y = jump.0;
						commands.entity(entity).insert(Jumping { left_ground: false });
					}
				}
			}
		}
	}
}

fn apply_movement_damping(
	mut query: Query<(&MovementDampingFactor, &mut LinearVelocity), With<CharacterController>>,
) {
	for (damping, mut velocity) in &mut query {
		velocity.x *= damping.0;
		velocity.z *= damping.0;
	}
}

#[derive(Debug, Clone, Copy)]
struct CameraPose {
	translation: Vec3,
	rotation: Quat,
}

impl CameraPose {
	fn interpolate(self, other: Self, amount: f32) -> Self {
		Self {
			translation: self.translation.lerp(other.translation, amount),
			rotation: self.rotation.slerp(other.rotation, amount),
		}
	}

	fn transform(self) -> Transform {
		Transform::from_translation(self.translation).with_rotation(self.rotation)
	}
}

pub(crate) fn follow_character_camera(
	time: Res<Time>,
	players: Query<&Transform, (With<CameraFollow>, Without<Camera3d>)>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	guns: Query<
		(&FirearmMembers, &Transform, &GlobalTransform),
		(With<HeldFirearm>, With<FirearmRoot>, Without<Camera3d>),
	>,
	maps: Query<&BoneMap>,
	globals: Query<&GlobalTransform, Without<Camera3d>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	let Ok(player) = players.single() else {
		return;
	};
	let Ok((mut camera_transform, mut controller)) = cameras.single_mut() else {
		return;
	};

	let yaw = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch = Quat::from_axis_angle(Vec3::X, controller.pitch);
	let rotation = yaw * pitch;
	let offset = rotation * Vec3::new(0.0, 0.0, CAMERA_DISTANCE) + Vec3::Y * CAMERA_HEIGHT;
	let target =
		player.translation + Vec3::Y * CAMERA_LOOK_HEIGHT + yaw * Vec3::X * CAMERA_SHOULDER_OFFSET;
	let mut third_person = Transform::from_translation(target + offset);
	third_person.look_at(target, Vec3::Y);
	let third_person =
		CameraPose { translation: third_person.translation, rotation: third_person.rotation };

	let head_translation = head_camera_translation(&visuals, &maps, &globals)
		.unwrap_or(player.translation + Vec3::Y * (CAMERA_HEIGHT + CAMERA_LOOK_HEIGHT));
	let head = CameraPose {
		translation: head_translation + rotation * -Vec3::Z * FIRST_PERSON_EYE_FORWARD,
		rotation,
	};
	let sight = sight_camera_pose(&guns, &maps, &globals);
	let focus_target = if controller.pov == CameraPov::FirstPerson && sight.is_some() {
		controller.focus
	} else {
		0.0
	};
	let blend_step = 1.0 - (-FOCUS_BLEND_SPEED * time.delta_secs()).exp();
	controller.focus_blend += (focus_target - controller.focus_blend) * blend_step;

	let pose = match controller.pov {
		CameraPov::ThirdPerson => third_person,
		CameraPov::FirstPerson => {
			sight.map_or(head, |sight| head.interpolate(sight, controller.focus_blend))
		}
	};
	*camera_transform = pose.transform();
}

/// Hide face meshes in first person; leave body / neck / clothing / weapon.
pub(crate) fn sync_first_person_head_visibility(
	cameras: Query<&CameraController, With<Camera3d>>,
	visuals: Query<&CharacterMembers, With<PlayerVisual>>,
	parts: Query<&PartNode>,
	mut visibilities: Query<&mut Visibility>,
) {
	let Ok(controller) = cameras.single() else {
		return;
	};
	let hide = controller.pov == CameraPov::FirstPerson;
	let visibility = if hide { Visibility::Hidden } else { Visibility::Inherited };
	for members in &visuals {
		for member in members.iter() {
			let Ok(part) = parts.get(member) else {
				continue;
			};
			if !is_first_person_hidden_slot(part.slot) {
				continue;
			};
			if let Ok(mut vis) = visibilities.get_mut(member) {
				*vis = visibility;
			}
		}
	}
}

fn is_first_person_hidden_slot(slot: CharacterPartSlot) -> bool {
	matches!(
		slot,
		CharacterPartSlot::HeadMesh
			| CharacterPartSlot::Nose
			| CharacterPartSlot::Mouth
			| CharacterPartSlot::EyeLeft
			| CharacterPartSlot::EyeRight
			| CharacterPartSlot::EarLeft
			| CharacterPartSlot::EarRight
			| CharacterPartSlot::Hair
			| CharacterPartSlot::Horns
	)
}

fn head_camera_translation(
	visuals: &Query<&CharacterMembers, With<PlayerVisual>>,
	maps: &Query<&BoneMap>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
) -> Option<Vec3> {
	let members = visuals.single().ok()?;
	let left = member_landmark_translation(members.iter(), maps, globals, "eye_socket.L")?;
	let right = member_landmark_translation(members.iter(), maps, globals, "eye_socket.R")?;
	Some((left + right) * 0.5)
}

fn sight_camera_pose(
	guns: &Query<
		(&FirearmMembers, &Transform, &GlobalTransform),
		(With<HeldFirearm>, With<FirearmRoot>, Without<Camera3d>),
	>,
	maps: &Query<&BoneMap>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
) -> Option<CameraPose> {
	let (members, current_root, previous_root) = guns.single().ok()?;
	let previous_socket =
		member_landmark_global(members.iter(), maps, globals, "sight_camera_socket")?;
	let socket_local = previous_root.affine().inverse() * previous_socket.affine();
	let socket_current = current_root.compute_affine() * socket_local;
	let (_, _, translation) = socket_current.to_scale_rotation_translation();
	let bore = (current_root.rotation * Vec3::Z).normalize_or(Vec3::Z);
	let look = sight_look_direction(bore);
	let mut aimed = Transform::from_translation(translation - look * SIGHT_CAMERA_BACK);
	aimed.look_to(look, Vec3::Y);
	Some(CameraPose { translation: aimed.translation, rotation: aimed.rotation })
}

/// Firearm rest bore is root +Z after the armature's glTF +90° X.
fn sight_look_direction(bore: Vec3) -> Vec3 {
	bore.normalize_or(Vec3::Z)
}

fn member_landmark_translation(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
	name: &str,
) -> Option<Vec3> {
	member_landmark_global(members, maps, globals, name).map(|global| global.translation())
}

fn member_landmark_global(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
	name: &str,
) -> Option<GlobalTransform> {
	for member in members {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		if let Ok(global) = globals.get(entity) {
			return Some(*global);
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use crate::player::{follow_character_camera, CameraPose};

	use super::*;

	#[test]
	fn camera_pose_interpolates_position_and_rotation() {
		let head = CameraPose { translation: Vec3::ZERO, rotation: Quat::IDENTITY };
		let sight = CameraPose {
			translation: Vec3::new(2.0, 0.0, 0.0),
			rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
		};
		let middle = head.interpolate(sight, 0.5);
		assert!((middle.translation - Vec3::X).length() < 1e-5);
		assert!((middle.rotation * -Vec3::Z).x < -0.6);
	}

	#[test]
	fn camera_queries_are_disjoint() -> Result<(), bevy::ecs::system::RunSystemError> {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.run_system_once(follow_character_camera)?;
		Ok(())
	}

	#[test]
	fn first_person_hides_face_not_body() {
		assert!(is_first_person_hidden_slot(CharacterPartSlot::HeadMesh));
		assert!(is_first_person_hidden_slot(CharacterPartSlot::Nose));
		assert!(!is_first_person_hidden_slot(CharacterPartSlot::BodyMesh));
		assert!(!is_first_person_hidden_slot(CharacterPartSlot::NeckMesh));
		assert!(!is_first_person_hidden_slot(CharacterPartSlot::Clothing));
	}

	#[test]
	fn sight_camera_looks_along_bore_and_sits_behind_socket() {
		let bore = Vec3::X;
		let look = sight_look_direction(bore);
		assert!(look.dot(bore) > 0.99, "{look:?}");
		let socket = Vec3::new(1.0, 1.5, 0.0);
		let camera = socket - look * SIGHT_CAMERA_BACK;
		assert!((camera.x - (1.0 - SIGHT_CAMERA_BACK)).abs() < 1e-4, "{camera}");
		let mut aimed = Transform::from_translation(camera);
		aimed.look_to(look, Vec3::Y);
		assert!((aimed.forward().dot(look) - 1.0).abs() < 1e-4, "{:?}", aimed.forward());
	}
}
