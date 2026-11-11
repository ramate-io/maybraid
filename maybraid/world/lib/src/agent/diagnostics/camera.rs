//! Explicit agent camera requests; no scans occur without a queued command.
use bevy::prelude::*;
use maybraid_agent::{
	AgentCameraProjection, AgentCameraSnapshot, AgentEvent, AgentEventBody, CommandStatus,
};
use player_camera::{CameraController, CameraPov, FollowCamera};
use world_player::PlaygroundMode;

use crate::agent::{emit, AgentRuntime};

#[derive(Resource, Default)]
pub(in crate::agent) struct CameraRequests {
	pending: Option<(u32, String, Option<[f32; 5]>)>,
}
impl CameraRequests {
	pub fn queue(&mut self, id: u32, line: String, pose: Option<[f32; 5]>) -> Result<(), String> {
		if self.pending.is_some() {
			return Err("a camera request is already pending this frame".into());
		}
		self.pending = Some((id, line, pose));
		Ok(())
	}
}

pub(super) fn install(app: &mut App) {
	app.init_resource::<CameraRequests>().add_systems(Last, complete_request);
}

fn complete_request(world: &mut World) {
	let Some((id, line, pose)) = world.resource_mut::<CameraRequests>().pending.take() else {
		return;
	};
	let result = CameraFixture::apply(world, pose);
	let runtime = world.resource::<AgentRuntime>();
	let t = world.resource::<Time>().elapsed_secs();
	let frame = runtime.frame;
	let (status, detail) = match result {
		Ok(state) => {
			emit(
				&runtime.sink,
				AgentEvent {
					t,
					frame,
					body: AgentEventBody::CameraSnapshot {
						request_id: id,
						applied_local: pose.is_some(),
						global_pose_pre_apply: pose.is_some(),
						state,
					},
				},
			);
			(CommandStatus::Finished, "actual camera snapshot recorded".to_owned())
		}
		Err(error) => (CommandStatus::Failed, error),
	};
	emit(
		&runtime.sink,
		AgentEvent {
			t,
			frame,
			body: AgentEventBody::Command { id, status, cmd: line, detail: Some(detail) },
		},
	);
}

pub(super) struct CameraFixture;
impl CameraFixture {
	pub fn apply(world: &mut World, pose: Option<[f32; 5]>) -> Result<AgentCameraSnapshot, String> {
		if world.get_resource::<PlaygroundMode>() != Some(&PlaygroundMode::Free) {
			return Err("camera diagnostics require mode free".into());
		}
		if world
			.get_resource::<crate::map_view::WorldMapView>()
			.is_some_and(|map| map.open)
		{
			return Err("close the map before camera diagnostics".into());
		}
		if pose.is_some_and(|pose| pose.iter().any(|value| !value.is_finite())) {
			return Err("camera pose must contain finite numbers".into());
		}
		let camera = {
			let mut query =
				world.query_filtered::<(Entity, &Camera), (With<Camera3d>, With<FollowCamera>)>();
			let mut active = query.iter(world).filter(|(_, camera)| camera.is_active);
			let (entity, _) = active.next().ok_or("requires one active gameplay follow camera")?;
			if active.next().is_some() {
				return Err("requires one active gameplay follow camera".into());
			}
			entity
		};
		if world.get::<ChildOf>(camera).is_some() {
			return Err(
				"parented gameplay cameras are unsupported by the absolute pose fixture".into()
			);
		}
		// Validate the complete readback contract before any mutation.
		Self::snapshot(world, camera)?;
		if let Some([x, y, z, yaw, pitch]) = pose {
			let yaw = player::wrap_pi(yaw);
			let pitch = player::clamp_aim_pitch(pitch);
			{
				let mut controller = world
					.get_mut::<CameraController>(camera)
					.ok_or("camera lacks shared controller")?;
				controller.yaw = yaw;
				controller.pitch = pitch;
				// The map branch writes poses even without a followed player.
				controller.pov = CameraPov::ThirdPerson;
				controller.resume_pov = CameraPov::ThirdPerson;
				controller.focus = 0.0;
				controller.ads = 0.0;
				controller.focus_blend = 0.0;
			}
			*world.get_mut::<Transform>(camera).ok_or("camera lacks local transform")? =
				Transform::from_translation(Vec3::new(x, y, z))
					.with_rotation(Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch));
		}
		// Do not synthesize GlobalTransform: the set event labels its previous
		// propagation. A later explicit readback observes the actual world pose.
		Self::snapshot(world, camera)
	}

	fn snapshot(world: &World, camera: Entity) -> Result<AgentCameraSnapshot, String> {
		let local = world.get::<Transform>(camera).ok_or("camera lacks local transform")?;
		let global = world.get::<GlobalTransform>(camera).ok_or("camera lacks global transform")?;
		let (_, global_rotation, global_translation) = global.to_scale_rotation_translation();
		let controller =
			world.get::<CameraController>(camera).ok_or("camera lacks shared controller")?;
		if !local.translation.is_finite()
			|| !local.rotation.is_finite()
			|| !global_translation.is_finite()
			|| !global_rotation.is_finite()
			|| !controller.yaw.is_finite()
			|| !controller.pitch.is_finite()
		{
			return Err("camera pose is non-finite".into());
		}
		let projection = match world.get::<Projection>(camera).ok_or("camera lacks projection")? {
			Projection::Perspective(p) => AgentCameraProjection::Perspective {
				fov: p.fov,
				near: p.near,
				far: p.far,
				aspect_ratio: p.aspect_ratio,
			},
			Projection::Orthographic(p) => {
				AgentCameraProjection::Orthographic { near: p.near, far: p.far, scale: p.scale }
			}
			Projection::Custom(_) => {
				return Err("custom camera projection is unsupported by the fixture".into());
			}
		};
		Ok(AgentCameraSnapshot {
			entity: camera.to_bits().to_string(),
			local_translation: local.translation.to_array(),
			local_rotation: local.rotation.to_array(),
			global_translation: global_translation.to_array(),
			global_rotation: global_rotation.to_array(),
			controller_yaw: controller.yaw,
			controller_pitch: controller.pitch,
			projection,
		})
	}
}

#[cfg(test)]
mod tests {
	use crate::agent::diagnostics::camera::CameraFixture;
	use bevy::prelude::*;
	use player_camera::{CameraController, FollowCamera};
	use world_player::PlaygroundMode;
	#[test]
	fn actual_shared_camera_requires_free_and_propagates_before_readback() -> Result<(), String> {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			bevy::transform::TransformPlugin,
			bevy::input::InputPlugin,
			player_camera::PlayerCameraPlugin,
		));
		app.add_message::<bevy::window::WindowFocused>();
		app.insert_resource(PlaygroundMode::Character);
		app.insert_resource(game_commands::command::TextEntryFocus(false));
		app.insert_resource(maybraid_session::WorldGameplayEnabled(true));
		let primary = app.world_mut().spawn_empty().id();
		app.insert_resource(maybraid_virtual_pad_controller::PrimaryVirtualPad(primary));
		let actor = app.world_mut().spawn(world_player::Player).id();
		app.init_resource::<maybraid_controllers_core::InputRouter>();
		let source = maybraid_controllers_core::InputSource::virtual_pad(primary);
		app.world_mut()
			.resource_mut::<maybraid_controllers_core::InputRouter>()
			.assign_actor(source, maybraid_controllers_core::ActorTarget::new(actor));
		let idle = app
			.world()
			.resource::<maybraid_controllers_core::InputRouter>()
			.route_actor(source, maybraid_character_controller::ActorControlSample::default())
			.ok_or("missing routed idle input")?;
		app.world_mut()
			.entity_mut(actor)
			.insert(maybraid_character_controller::RoutedActorControl::new(idle));

		let camera = app
			.world_mut()
			.spawn((
				Camera3d::default(),
				Camera::default(),
				FollowCamera::default(),
				player_camera::CameraTarget(None),
				CameraController::default(),
				Transform::from_xyz(9., 8., 7.),
				GlobalTransform::default(),
				Projection::Perspective(PerspectiveProjection::default()),
			))
			.id();
		crate::camera::add_free_camera_fixture(&mut app, camera);
		app.update();
		assert!(CameraFixture::apply(app.world_mut(), Some([1., 2., 3., 0.4, -0.2])).is_err());
		app.insert_resource(PlaygroundMode::Free);
		{
			let mut controller = app
				.world_mut()
				.get_mut::<CameraController>(camera)
				.ok_or("missing actual shared controller")?;
			controller.pov = player_camera::CameraPov::Map;
			controller.resume_pov = player_camera::CameraPov::Map;
			controller.focus = 1.;
			controller.ads = 1.;
			controller.focus_blend = 1.;
		}
		let set = CameraFixture::apply(app.world_mut(), Some([1., 2., 3., 0.4, -0.2]))?;
		assert_eq!(set.local_translation, [1., 2., 3.]);
		assert_eq!(set.global_translation, [9., 8., 7.]);
		assert_eq!(
			app.world().get::<CameraController>(camera).map(|c| c.pov),
			Some(player_camera::CameraPov::ThirdPerson)
		);
		// A routed neutral sample must reach the production free-fly write and
		// restore orientation from the shared controller, not just early-return.
		app.world_mut()
			.get_mut::<Transform>(camera)
			.ok_or("missing camera transform")?
			.rotation = Quat::IDENTITY;
		app.update();
		let read = CameraFixture::apply(app.world_mut(), None)?;
		assert_eq!(read.local_translation, read.global_translation);
		assert!(
			Quat::from_array(read.local_rotation)
				.dot(Quat::from_array(set.local_rotation))
				.abs() > 1. - 1e-6
		);
		assert!(
			Quat::from_array(read.local_rotation)
				.dot(Quat::from_array(read.global_rotation))
				.abs() > 1. - 1e-6
		);
		app.update();
		assert_eq!(CameraFixture::apply(app.world_mut(), None)?, read);
		let parent = app.world_mut().spawn_empty().id();
		app.world_mut().entity_mut(camera).insert(ChildOf(parent));
		assert!(CameraFixture::apply(app.world_mut(), None).is_err());
		Ok(())
	}
	#[test]
	fn ambiguous_inactive_and_missing_components_fail_without_mutation() -> Result<(), String> {
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Free);
		assert!(CameraFixture::apply(&mut world, None).is_err());
		let first = world
			.spawn((
				Camera3d::default(),
				FollowCamera::default(),
				CameraController::default(),
				Transform::default(),
				GlobalTransform::default(),
				Projection::Perspective(PerspectiveProjection::default()),
			))
			.id();
		let second = world.spawn((Camera3d::default(), FollowCamera::default())).id();
		assert!(CameraFixture::apply(&mut world, Some([1., 2., 3., 0., 0.])).is_err());
		world.get_mut::<Camera>(second).ok_or("missing required camera")?.is_active = false;
		assert!(CameraFixture::apply(&mut world, Some([f32::NAN, 2., 3., 0., 0.])).is_err());
		let snapshot = CameraFixture::apply(&mut world, None)?;
		assert_eq!(snapshot.entity, first.to_bits().to_string());
		assert_eq!(snapshot.local_translation, [0.; 3]);
		Ok(())
	}
}
