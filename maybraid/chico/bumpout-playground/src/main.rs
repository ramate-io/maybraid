use std::f32::consts::PI;

use bevy::prelude::*;
use chico_bumpout::{BumpOut, BumpOutNeighborhood, BumpOutStyle, ChicoBumpOutPlugin};
use lod_cascade::Chunk;
use procedural_common::NoiseParams;
use render_item::mesh::{IdentifiedMesh, MeshId};
use render_item::sdf::cpu_shot::CpuShotBuilder;
use render_item::NormalizeChunk;
use sdf::Sdf;
use terrain_chunk_ref::{TerrainChunkRef, TerrainChunkRefPlugin};

type PlaygroundTerrainBuilder = CpuShotBuilder<PlaygroundTerrain>;

#[derive(Clone, Debug)]
struct PlaygroundTerrain;

impl PlaygroundTerrain {
	fn height_at(x: f32, z: f32) -> f32 {
		3.8 * (x * 0.035).sin() * (z * 0.028).cos()
			+ 1.7 * ((x + z) * 0.075).sin()
			+ 0.8 * (z * 0.11).cos()
	}
}

impl Sdf for PlaygroundTerrain {
	fn distance(&self, p: Vec3) -> f32 {
		p.y - Self::height_at(p.x, p.z)
	}
}

impl NormalizeChunk for PlaygroundTerrain {}

impl IdentifiedMesh for PlaygroundTerrain {
	fn id(&self) -> MeshId {
		MeshId::new("chico-bumpout-playground-terrain-v1".into())
	}
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum PresenterLayer {
	Terrain,
	GroundCover,
	CanopyProxy,
}

#[derive(Component)]
struct FlyCamera {
	yaw: f32,
	pitch: f32,
	speed: f32,
	sensitivity: f32,
}

#[derive(Resource, Default)]
struct SharedHandleReport(bool);

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Chico Terrain-Mesh Bump Outs".into(),
				resolution: (1280, 720).into(),
				..default()
			}),
			..default()
		}))
		.insert_resource(ClearColor(Color::srgb(0.50, 0.72, 0.86)))
		.init_resource::<SharedHandleReport>()
		.add_plugins((
			ChicoBumpOutPlugin,
			TerrainChunkRefPlugin::<PlaygroundTerrainBuilder>::default(),
		))
		.add_systems(Startup, (setup_scene, setup_instructions))
		.add_systems(Update, (fly_camera, toggle_layers, report_shared_mesh_handle))
		.run();
}

fn setup_scene(mut commands: Commands, mut standard_materials: ResMut<Assets<StandardMaterial>>) {
	commands.insert_resource(GlobalAmbientLight {
		color: Color::srgb(0.72, 0.82, 1.0),
		brightness: 650.0,
		..default()
	});
	commands.spawn((
		DirectionalLight { illuminance: 14_000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.2, PI / 5.0, 0.0)),
	));

	let camera_transform =
		Transform::from_xyz(118.0, 82.0, 132.0).looking_at(Vec3::new(0.0, 12.0, 0.0), Vec3::Y);
	let (yaw, pitch, _) = camera_transform.rotation.to_euler(EulerRot::YXZ);
	commands.spawn((
		Camera3d::default(),
		camera_transform,
		Projection::Perspective(PerspectiveProjection { near: 0.1, far: 1200.0, ..default() }),
		FlyCamera { yaw, pitch, speed: 55.0, sensitivity: 0.0035 },
	));

	let chunk =
		Chunk::from_min_max(Vec3::new(-80.0, -18.0, -80.0), Vec3::new(80.0, 42.0, 80.0), None);
	let terrain_ref = TerrainChunkRef::new(CpuShotBuilder::new(PlaygroundTerrain), chunk, 6);
	let source_transform = terrain_ref.transform();
	let terrain_material = standard_materials.add(StandardMaterial {
		base_color: Color::srgb(0.33, 0.30, 0.20),
		perceptual_roughness: 0.98,
		..default()
	});

	commands.spawn((
		PresenterLayer::Terrain,
		terrain_ref.clone(),
		MeshMaterial3d(terrain_material),
		source_transform,
		Visibility::default(),
	));

	let ground_profile = BumpOutNeighborhood::new(
		[0.42, 0.58, 0.72, 0.52, 0.82, 0.92, 0.68, 0.88, 0.97],
		[0.25, 0.45, 0.70, 0.35, 0.65, 1.00, 0.55, 0.95, 1.35],
	);
	let ground = BumpOut::from_neighborhood(
		ground_profile,
		[
			Color::srgb(0.12, 0.28, 0.08),
			Color::srgb(0.22, 0.48, 0.12),
			Color::srgb(0.52, 0.66, 0.18),
		],
		NoiseParams::from_scalar(101.0, 0.085, 0.55, 2),
	)
	.with_style(BumpOutStyle::new(0.055, 0.96, 0.42));
	let ground_entity = ground.spawn(&mut commands, terrain_ref.clone());
	commands.entity(ground_entity).insert(PresenterLayer::GroundCover);

	let canopy_profile = BumpOutNeighborhood::new(
		[0.10, 0.28, 0.46, 0.22, 0.68, 0.90, 0.48, 0.84, 0.98],
		[12.0, 16.0, 21.0, 15.0, 24.0, 30.0, 20.0, 29.0, 35.0],
	);
	let canopy = BumpOut::from_neighborhood(
		canopy_profile,
		[
			Color::srgb(0.035, 0.16, 0.055),
			Color::srgb(0.08, 0.31, 0.10),
			Color::srgb(0.23, 0.48, 0.13),
		],
		NoiseParams::from_scalar(307.0, 0.045, 3.2, 3),
	)
	.with_style(BumpOutStyle::new(0.065, 0.88, 0.18));
	let canopy_entity = canopy.spawn(&mut commands, terrain_ref);
	commands.entity(canopy_entity).insert(PresenterLayer::CanopyProxy);
}

fn setup_instructions(mut commands: Commands) {
	commands.spawn((
		Text::new(
			"TerrainChunkRef<T> shared by three presenters\n\
			 RMB + mouse: look   WASD/QE: move   Shift: faster\n\
			 1: terrain   2: ground cover   3: canopy proxy",
		),
		TextFont { font_size: FontSize::Px(19.0), ..default() },
		TextColor(Color::WHITE),
		Node { position_type: PositionType::Absolute, top: px(16.0), left: px(18.0), ..default() },
	));
}

fn fly_camera(
	keys: Res<ButtonInput<KeyCode>>,
	mouse_buttons: Res<ButtonInput<MouseButton>>,
	mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
	time: Res<Time>,
	mut cameras: Query<(&mut Transform, &mut FlyCamera), With<Camera3d>>,
) {
	let Ok((mut transform, mut controller)) = cameras.single_mut() else {
		return;
	};

	let mut mouse_delta = Vec2::ZERO;
	for motion in mouse_motion.read() {
		mouse_delta += motion.delta;
	}
	if mouse_buttons.pressed(MouseButton::Right) {
		controller.yaw -= mouse_delta.x * controller.sensitivity;
		controller.pitch = (controller.pitch - mouse_delta.y * controller.sensitivity)
			.clamp(-PI * 0.49, PI * 0.49);
		transform.rotation = Quat::from_axis_angle(Vec3::Y, controller.yaw)
			* Quat::from_axis_angle(Vec3::X, controller.pitch);
	}

	let mut movement = Vec3::ZERO;
	let forward = transform.forward();
	let right = transform.right();
	if keys.pressed(KeyCode::KeyW) {
		movement += *forward;
	}
	if keys.pressed(KeyCode::KeyS) {
		movement -= *forward;
	}
	if keys.pressed(KeyCode::KeyA) {
		movement -= *right;
	}
	if keys.pressed(KeyCode::KeyD) {
		movement += *right;
	}
	if keys.pressed(KeyCode::KeyQ) {
		movement -= Vec3::Y;
	}
	if keys.pressed(KeyCode::KeyE) {
		movement += Vec3::Y;
	}
	if movement != Vec3::ZERO {
		let boost = if keys.pressed(KeyCode::ShiftLeft) { 3.0 } else { 1.0 };
		transform.translation +=
			movement.normalize() * controller.speed * boost * time.delta_secs();
	}
}

fn toggle_layers(
	keys: Res<ButtonInput<KeyCode>>,
	mut layers: Query<(&PresenterLayer, &mut Visibility)>,
) {
	let selected = if keys.just_pressed(KeyCode::Digit1) {
		Some(PresenterLayer::Terrain)
	} else if keys.just_pressed(KeyCode::Digit2) {
		Some(PresenterLayer::GroundCover)
	} else if keys.just_pressed(KeyCode::Digit3) {
		Some(PresenterLayer::CanopyProxy)
	} else {
		None
	};
	let Some(selected) = selected else {
		return;
	};

	for (layer, mut visibility) in &mut layers {
		if *layer == selected {
			*visibility = match *visibility {
				Visibility::Hidden => Visibility::Visible,
				_ => Visibility::Hidden,
			};
		}
	}
}

fn report_shared_mesh_handle(
	mut report: ResMut<SharedHandleReport>,
	layers: Query<(&PresenterLayer, &Mesh3d)>,
) {
	if report.0 {
		return;
	}
	let mut handles = Vec::new();
	for (layer, mesh) in &layers {
		handles.push((*layer, mesh.0.id()));
	}
	if handles.len() != 3 {
		return;
	}
	let shared = handles.windows(2).all(|pair| pair[0].1 == pair[1].1);
	info!("Terrain/GroundCover/CanopyProxy resolved one shared mesh handle: {shared}");
	report.0 = true;
}
