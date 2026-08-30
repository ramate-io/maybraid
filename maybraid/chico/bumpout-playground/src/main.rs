use std::f32::consts::PI;

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use chico_bumpout::{BumpOut, BumpOutNeighborhood, BumpOutStyle, ChicoBumpOutPlugin};
use lod_cascade::Chunk;
use material_ref::MaterialRefRoot;
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

impl PresenterLayer {
	fn label(self) -> &'static str {
		match self {
			Self::Terrain => "Terrain",
			Self::GroundCover => "GroundCover",
			Self::CanopyProxy => "CanopyProxy",
		}
	}
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

#[derive(Resource)]
struct NeighborhoodControls {
	layer: PresenterLayer,
	row: usize,
	column: usize,
}

impl NeighborhoodControls {
	fn sample_index(&self) -> usize {
		self.row * 3 + self.column
	}
}

impl Default for NeighborhoodControls {
	fn default() -> Self {
		Self { layer: PresenterLayer::GroundCover, row: 1, column: 1 }
	}
}

#[derive(Component)]
struct NeighborhoodControlsText;

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
		.init_resource::<NeighborhoodControls>()
		.add_plugins((
			ChicoBumpOutPlugin,
			TerrainChunkRefPlugin::<PlaygroundTerrainBuilder>::default(),
		))
		.add_systems(Startup, (setup_scene, setup_instructions))
		.add_systems(
			Update,
			(
				fly_camera,
				toggle_layers,
				edit_neighborhood,
				update_neighborhood_hud.after(edit_neighborhood),
				report_shared_mesh_handle,
			),
		)
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
	.with_style(
		BumpOutStyle::new(0.055, 0.96, 0.42)
			.with_cheese(0.72, 1.35)
			.with_fragment_height(5.0, 0.14),
	);
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
	.with_style(
		BumpOutStyle::new(0.065, 0.88, 0.18)
			.with_cheese(0.88, 1.0)
			.with_fragment_height(4.5, 0.85),
	);
	let canopy_entity = canopy.spawn(&mut commands, terrain_ref);
	commands.entity(canopy_entity).insert(PresenterLayer::CanopyProxy);
}

fn setup_instructions(mut commands: Commands) {
	commands.spawn((
		Text::new(
			"TerrainChunkRef<T> shared by three presenters\n\
			 RMB + mouse: look   WASD/QE: move   Shift: faster\n\
			 1: terrain   2: ground cover   3: canopy proxy\n\
			 Tab: edit layer   Arrows: select neighbor\n\
			 Z/X: density -/+   N/M: height -/+   V/B: cheese -/+",
		),
		TextFont { font_size: FontSize::Px(19.0), ..default() },
		TextColor(Color::WHITE),
		Node { position_type: PositionType::Absolute, top: px(16.0), left: px(18.0), ..default() },
		NeighborhoodControlsText,
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

type EditableBumpOut<'a> = (
	&'a PresenterLayer,
	&'a TerrainChunkRef<PlaygroundTerrainBuilder>,
	&'a mut BumpOut,
	&'a mut MaterialRefRoot,
	&'a mut Aabb,
);

fn edit_neighborhood(
	keys: Res<ButtonInput<KeyCode>>,
	mut controls: ResMut<NeighborhoodControls>,
	mut layers: Query<EditableBumpOut>,
) {
	if keys.just_pressed(KeyCode::Tab) {
		controls.layer = match controls.layer {
			PresenterLayer::GroundCover => PresenterLayer::CanopyProxy,
			_ => PresenterLayer::GroundCover,
		};
	}
	if keys.just_pressed(KeyCode::ArrowLeft) {
		controls.column = controls.column.saturating_sub(1);
	}
	if keys.just_pressed(KeyCode::ArrowRight) {
		controls.column = (controls.column + 1).min(2);
	}
	if keys.just_pressed(KeyCode::ArrowUp) {
		controls.row = controls.row.saturating_sub(1);
	}
	if keys.just_pressed(KeyCode::ArrowDown) {
		controls.row = (controls.row + 1).min(2);
	}

	let density_delta = if keys.just_pressed(KeyCode::KeyZ) {
		-0.05
	} else if keys.just_pressed(KeyCode::KeyX) {
		0.05
	} else {
		0.0
	};
	let height_step = if controls.layer == PresenterLayer::CanopyProxy { 1.0 } else { 0.25 };
	let height_delta = if keys.just_pressed(KeyCode::KeyN) {
		-height_step
	} else if keys.just_pressed(KeyCode::KeyM) {
		height_step
	} else {
		0.0
	};
	let cheese_delta = if keys.just_pressed(KeyCode::KeyV) {
		-0.1
	} else if keys.just_pressed(KeyCode::KeyB) {
		0.1
	} else {
		0.0
	};
	if density_delta == 0.0 && height_delta == 0.0 && cheese_delta == 0.0 {
		return;
	}

	let sample = controls.sample_index();
	for (layer, terrain_ref, mut bump_out, mut material_root, mut aabb) in &mut layers {
		if *layer != controls.layer {
			continue;
		}
		let mut neighborhood = bump_out.neighborhood();
		if density_delta != 0.0 {
			neighborhood.set_density(sample, neighborhood.densities[sample] + density_delta);
		}
		if height_delta != 0.0 {
			neighborhood.set_height(sample, neighborhood.heights[sample] + height_delta);
		}
		let mut style = bump_out.style();
		if cheese_delta != 0.0 {
			style.cheese_amount = (style.cheese_amount + cheese_delta).clamp(0.0, 1.0);
		}

		bump_out.set_neighborhood(neighborhood);
		bump_out.set_style(style);
		material_root.0 = bump_out.material.clone();
		*aabb = bump_out.aabb(terrain_ref);
	}
}

fn update_neighborhood_hud(
	controls: Res<NeighborhoodControls>,
	layers: Query<(&PresenterLayer, &BumpOut)>,
	mut hud: Query<&mut Text, With<NeighborhoodControlsText>>,
) {
	let Ok(mut text) = hud.single_mut() else {
		return;
	};
	let Some((_, bump_out)) = layers.iter().find(|(layer, _)| **layer == controls.layer) else {
		return;
	};
	let neighborhood = bump_out.neighborhood();
	let style = bump_out.style();
	let selected = controls.sample_index();
	let cell = |index: usize, value: f32| {
		if index == selected {
			format!("[{value:5.2}]")
		} else {
			format!(" {value:5.2} ")
		}
	};

	text.0 = format!(
		"TerrainChunkRef<T> shared by three presenters\n\
		 RMB + mouse: look   WASD/QE: move   Shift: faster\n\
		 1: terrain   2: ground cover   3: canopy proxy\n\
		 Tab: edit layer   Arrows: select neighbor\n\
		 Z/X: density -/+   N/M: height -/+   V/B: cheese -/+\n\n\
		 Editing {} sample ({}, {})   cheese={:.2}\n\
		 density  {} {} {}\n\
		          {} {} {}\n\
		          {} {} {}\n\
		 height   {} {} {}\n\
		          {} {} {}\n\
		          {} {} {}",
		controls.layer.label(),
		controls.column,
		controls.row,
		style.cheese_amount,
		cell(0, neighborhood.densities[0]),
		cell(1, neighborhood.densities[1]),
		cell(2, neighborhood.densities[2]),
		cell(3, neighborhood.densities[3]),
		cell(4, neighborhood.densities[4]),
		cell(5, neighborhood.densities[5]),
		cell(6, neighborhood.densities[6]),
		cell(7, neighborhood.densities[7]),
		cell(8, neighborhood.densities[8]),
		cell(0, neighborhood.heights[0]),
		cell(1, neighborhood.heights[1]),
		cell(2, neighborhood.heights[2]),
		cell(3, neighborhood.heights[3]),
		cell(4, neighborhood.heights[4]),
		cell(5, neighborhood.heights[5]),
		cell(6, neighborhood.heights[6]),
		cell(7, neighborhood.heights[7]),
		cell(8, neighborhood.heights[8]),
	);
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
