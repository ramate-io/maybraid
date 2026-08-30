mod commands;
mod ui;

use std::f32::consts::PI;

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use chico_bumpout::{BumpOut, BumpOutNeighborhood, BumpOutStyle, ChicoBumpOutPlugin};
use commands::{NeighborhoodValues, PlaygroundCommand};
use game_commands::command::{GameCommandPlugin, PendingStartupCommand, TextEntryFocus};
use lod_cascade::Chunk;
use material_ref::MaterialRefRoot;
use procedural_common::NoiseParams;
use render_item::mesh::{IdentifiedMesh, MeshId};
use render_item::sdf::cpu_shot::CpuShotBuilder;
use render_item::NormalizeChunk;
use sdf::Sdf;
use terrain_chunk_ref::{TerrainChunkRef, TerrainChunkRefPlugin};

type PlaygroundTerrainBuilder = CpuShotBuilder<PlaygroundTerrain>;

const TILE_RADIUS: i32 = 2;
const TILE_SIZE: f32 = 52.0;
const TERRAIN_MIN_Y: f32 = -18.0;
const TERRAIN_MAX_Y: f32 = 42.0;
const TILE_RES_2: u8 = 5;

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
pub(crate) enum PresenterLayer {
	Terrain,
	GroundCover,
	CanopyProxy,
}

#[derive(Debug, Clone, Copy)]
struct NeighborhoodSample {
	density: f32,
	bite_size: f32,
	bite_size_deviation: f32,
	average_height: f32,
	height_deviation: f32,
}

impl PresenterLayer {
	fn label(self) -> &'static str {
		match self {
			Self::Terrain => "Terrain",
			Self::GroundCover => "GroundCover",
			Self::CanopyProxy => "CanopyProxy",
		}
	}

	fn sample(self, coordinate: IVec2) -> NeighborhoodSample {
		let x = coordinate.x as f32;
		let z = coordinate.y as f32;
		match self {
			Self::GroundCover => {
				let density = (0.62
					+ 0.20 * (x * 0.93 + z * 0.37).sin()
					+ 0.15 * (z * 1.11 - x * 0.29).cos())
				.clamp(0.08, 0.98);
				NeighborhoodSample {
					density,
					bite_size: 4.5 + 3.5 * (0.5 + 0.5 * (x * 0.71 + z * 0.53).sin()),
					bite_size_deviation: 0.2 + 0.65 * (0.5 + 0.5 * (x * 0.43 - z * 0.97).cos()),
					average_height: (1.8
						+ 1.65 * (x * 1.17 - z * 0.63).sin()
						+ 0.75 * (z * 1.31 + x * 0.41).cos())
					.clamp(0.1, 4.2),
					height_deviation: 0.15 + 0.8 * (0.5 + 0.5 * (x * 0.83 + z * 0.59).cos()),
				}
			}
			Self::CanopyProxy => {
				let density = (0.50
					+ 0.32 * (x * 0.78 + z * 0.44).sin()
					+ 0.20 * (z * 0.91 - x * 0.32).cos())
				.clamp(0.03, 0.98);
				NeighborhoodSample {
					density,
					bite_size: 9.0 + 15.0 * (0.5 + 0.5 * (x * 0.57 - z * 0.81).sin()),
					bite_size_deviation: 0.25 + 1.0 * (0.5 + 0.5 * (x * 0.37 + z * 0.73).cos()),
					average_height: (25.0
						+ 14.0 * (x * 1.17 - z * 0.63).sin()
						+ 10.0 * (z * 1.03 + x * 0.41).cos())
					.clamp(5.0, 48.0),
					height_deviation: 2.0 + 7.0 * (0.5 + 0.5 * (x * 0.69 + z * 0.47).cos()),
				}
			}
			Self::Terrain => NeighborhoodSample {
				density: 0.0,
				bite_size: 1.0,
				bite_size_deviation: 0.0,
				average_height: 0.0,
				height_deviation: 0.0,
			},
		}
	}

	fn neighborhood(self, center: IVec2) -> BumpOutNeighborhood {
		let mut densities = [0.0; 9];
		let mut bite_sizes = [0.0; 9];
		let mut bite_size_deviations = [0.0; 9];
		let mut average_heights = [0.0; 9];
		let mut height_deviations = [0.0; 9];
		for row in 0..3 {
			for column in 0..3 {
				let coordinate = center + IVec2::new(column as i32 - 1, row as i32 - 1);
				let index = row * 3 + column;
				let sample = self.sample(coordinate);
				densities[index] = sample.density;
				bite_sizes[index] = sample.bite_size;
				bite_size_deviations[index] = sample.bite_size_deviation;
				average_heights[index] = sample.average_height;
				height_deviations[index] = sample.height_deviation;
			}
		}
		BumpOutNeighborhood::new(
			densities,
			bite_sizes,
			bite_size_deviations,
			average_heights,
			height_deviations,
		)
	}

	fn bump_out(self, tile: IVec2) -> BumpOut {
		match self {
			Self::GroundCover => BumpOut::from_neighborhood(
				self.neighborhood(tile),
				[
					Color::srgb(0.12, 0.28, 0.08),
					Color::srgb(0.22, 0.48, 0.12),
					Color::srgb(0.52, 0.66, 0.18),
				],
				NoiseParams::from_scalar(101.0, 0.085, 0.0, 2),
			)
			.with_style(
				BumpOutStyle::new(0.055, 0.96, 0.42)
					.with_cheese(0.72, 1.35)
					.with_fragment_height(5.0, 0.14),
			),
			Self::CanopyProxy => BumpOut::from_neighborhood(
				self.neighborhood(tile),
				[
					Color::srgb(0.035, 0.16, 0.055),
					Color::srgb(0.08, 0.31, 0.10),
					Color::srgb(0.23, 0.48, 0.13),
				],
				NoiseParams::from_scalar(307.0, 0.045, 0.0, 3),
			)
			.with_style(
				BumpOutStyle::new(0.065, 0.88, 0.18)
					.with_cheese(0.88, 1.0)
					.with_fragment_height(4.5, 0.85),
			),
			Self::Terrain => unreachable!("terrain does not use a bump-out material"),
		}
	}
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct TileCoordinate(IVec2);

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

	fn selected_coordinate(&self) -> IVec2 {
		IVec2::new(self.column as i32 - 1, self.row as i32 - 1)
	}
}

impl Default for NeighborhoodControls {
	fn default() -> Self {
		Self { layer: PresenterLayer::GroundCover, row: 1, column: 1 }
	}
}

fn main() {
	let startup = PlaygroundCommand::parse_startup_command().unwrap_or_else(|error| {
		eprintln!("{error}");
		std::process::exit(2);
	});
	if startup.is_some() {
		println!("Startup command from argv (same as in-game / text).");
	} else {
		println!("Chico bump-out playground — press / for commands.");
	}

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
		.insert_resource(PendingStartupCommand(startup))
		.init_resource::<SharedHandleReport>()
		.init_resource::<NeighborhoodControls>()
		.add_plugins((
			ChicoBumpOutPlugin,
			TerrainChunkRefPlugin::<PlaygroundTerrainBuilder>::default(),
			GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()),
		))
		.add_systems(Startup, setup_scene)
		.add_systems(
			Update,
			(
				fly_camera,
				report_shared_mesh_handle,
				ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
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
		Transform::from_xyz(178.0, 142.0, 196.0).looking_at(Vec3::new(0.0, 12.0, 0.0), Vec3::Y);
	let (yaw, pitch, _) = camera_transform.rotation.to_euler(EulerRot::YXZ);
	commands.spawn((
		Camera3d::default(),
		camera_transform,
		Projection::Perspective(PerspectiveProjection { near: 0.1, far: 1200.0, ..default() }),
		FlyCamera { yaw, pitch, speed: 55.0, sensitivity: 0.0035 },
	));

	let terrain_material = standard_materials.add(StandardMaterial {
		base_color: Color::srgb(0.33, 0.30, 0.20),
		perceptual_roughness: 0.98,
		..default()
	});
	for z in -TILE_RADIUS..=TILE_RADIUS {
		for x in -TILE_RADIUS..=TILE_RADIUS {
			let tile = IVec2::new(x, z);
			let horizontal_min =
				Vec2::new((x as f32 - 0.5) * TILE_SIZE, (z as f32 - 0.5) * TILE_SIZE);
			let chunk = Chunk::from_min_max(
				Vec3::new(horizontal_min.x, TERRAIN_MIN_Y, horizontal_min.y),
				Vec3::new(
					horizontal_min.x + TILE_SIZE,
					TERRAIN_MAX_Y,
					horizontal_min.y + TILE_SIZE,
				),
				None,
			);
			let terrain_ref =
				TerrainChunkRef::new(CpuShotBuilder::new(PlaygroundTerrain), chunk, TILE_RES_2);
			let source_transform = terrain_ref.transform();

			commands.spawn((
				PresenterLayer::Terrain,
				TileCoordinate(tile),
				terrain_ref.clone(),
				MeshMaterial3d(terrain_material.clone()),
				source_transform,
				Visibility::default(),
			));

			for layer in [PresenterLayer::GroundCover, PresenterLayer::CanopyProxy] {
				let entity = layer.bump_out(tile).spawn(&mut commands, terrain_ref.clone());
				commands.entity(entity).insert((layer, TileCoordinate(tile)));
			}
		}
	}
}

fn fly_camera(
	keys: Res<ButtonInput<KeyCode>>,
	mouse_buttons: Res<ButtonInput<MouseButton>>,
	mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
	time: Res<Time>,
	text_entry: Res<TextEntryFocus>,
	mut cameras: Query<(&mut Transform, &mut FlyCamera), With<Camera3d>>,
) {
	let Ok((mut transform, mut controller)) = cameras.single_mut() else {
		return;
	};

	let mut mouse_delta = Vec2::ZERO;
	for motion in mouse_motion.read() {
		mouse_delta += motion.delta;
	}
	if text_entry.0 {
		return;
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

type EditableBumpOut<'a> = (
	&'a PresenterLayer,
	&'a TileCoordinate,
	&'a TerrainChunkRef<PlaygroundTerrainBuilder>,
	&'a mut BumpOut,
	&'a mut MaterialRefRoot,
	&'a mut Aabb,
);

pub(crate) fn apply_neighborhood_edit(
	world: &mut World,
	values: &NeighborhoodValues,
	adjust: bool,
) {
	let (layer, selected_coordinate) = {
		let controls = world.resource::<NeighborhoodControls>();
		(controls.layer, controls.selected_coordinate())
	};
	let mut layers = world.query::<EditableBumpOut>();
	for (candidate, tile, terrain_ref, mut bump_out, mut material_root, mut aabb) in
		layers.iter_mut(world)
	{
		if *candidate != layer {
			continue;
		}
		let relative = selected_coordinate - tile.0;
		if relative.x.abs() > 1 || relative.y.abs() > 1 {
			continue;
		}

		let sample = ((relative.y + 1) * 3 + relative.x + 1) as usize;
		let mut neighborhood = bump_out.neighborhood();
		if let Some(value) = values.density {
			neighborhood
				.set_density(sample, edited_value(neighborhood.densities[sample], value, adjust));
		}
		if let Some(value) = values.bite_size {
			neighborhood.set_bite_size(
				sample,
				edited_value(neighborhood.bite_sizes[sample], value, adjust),
			);
		}
		if let Some(value) = values.bite_size_deviation {
			neighborhood.set_bite_size_deviation(
				sample,
				edited_value(neighborhood.bite_size_deviations[sample], value, adjust),
			);
		}
		if let Some(value) = values.average_height {
			neighborhood.set_average_height(
				sample,
				edited_value(neighborhood.average_heights[sample], value, adjust),
			);
		}
		if let Some(value) = values.height_deviation {
			neighborhood.set_height_deviation(
				sample,
				edited_value(neighborhood.height_deviations[sample], value, adjust),
			);
		}

		bump_out.set_neighborhood(neighborhood);
		material_root.0 = bump_out.material.clone();
		*aabb = bump_out.aabb(terrain_ref);
	}
}

pub(crate) fn change_layer_visibility(
	world: &mut World,
	selected: PresenterLayer,
	visible: Option<bool>,
) {
	let mut layers = world.query::<(&PresenterLayer, &mut Visibility)>();
	for (layer, mut visibility) in layers.iter_mut(world) {
		if *layer != selected {
			continue;
		}
		let show = visible.unwrap_or(matches!(*visibility, Visibility::Hidden));
		*visibility = if show { Visibility::Visible } else { Visibility::Hidden };
	}
}

fn edited_value(current: f32, requested: f32, adjust: bool) -> f32 {
	if adjust {
		current + requested
	} else {
		requested
	}
}

fn report_shared_mesh_handle(
	mut report: ResMut<SharedHandleReport>,
	layers: Query<(&TileCoordinate, &PresenterLayer, &Mesh3d)>,
) {
	if report.0 {
		return;
	}
	let mut handles = Vec::new();
	for (tile, layer, mesh) in &layers {
		handles.push((tile.0, *layer, mesh.0.id()));
	}
	let diameter = (TILE_RADIUS * 2 + 1) as usize;
	if handles.len() != diameter * diameter * 3 {
		return;
	}

	let mut shared = true;
	for z in -TILE_RADIUS..=TILE_RADIUS {
		for x in -TILE_RADIUS..=TILE_RADIUS {
			let coordinate = IVec2::new(x, z);
			let terrain_handle = handles
				.iter()
				.find(|(tile, layer, _)| *tile == coordinate && *layer == PresenterLayer::Terrain)
				.map(|(_, _, handle)| *handle);
			for layer in [PresenterLayer::GroundCover, PresenterLayer::CanopyProxy] {
				let bump_handle = handles
					.iter()
					.find(|(tile, candidate, _)| *tile == coordinate && *candidate == layer)
					.map(|(_, _, handle)| *handle);
				shared &= terrain_handle.is_some() && terrain_handle == bump_handle;
			}
		}
	}
	info!("Every tile shares one mesh handle across its three presenters: {shared}");
	report.0 = true;
}
