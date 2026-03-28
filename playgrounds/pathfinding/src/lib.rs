//! 2D demo: local pathfinding steers a red agent around a vertical wall toward the cursor (blue).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use intelligence::local_pathfinding::{
	FindPath, LocalPathPlan, LocalPathfinding, LocalPathFindingFanout, LocalPathfindingSurface,
	respond_to_find_path_requests,
};

// --- Pathfinding surface: floor z = 0, infinite wall on the Y axis at x = 0 ---

#[derive(Clone, Copy, Debug)]
struct PlaygroundSurface {
	wall_x: f32,
}

impl LocalPathfindingSurface for PlaygroundSurface {
	fn snap_for_local_pathfinding(&self, position: Vec3) -> Vec3 {
		Vec3::new(position.x, position.y, 0.0)
	}

	fn path_ray_trace_distance(&self, start: Vec3, end: Vec3) -> f32 {
		let d = end - start;
		let len = d.length();
		if len < 1e-12 {
			return len;
		}
		if d.x.abs() < 1e-6 {
			return len;
		}
		let u = (self.wall_x - start.x) / d.x;
		if u <= 0.0 || u >= 1.0 {
			return len;
		}
		-(u * len)
	}
}

#[derive(Clone, Copy, Debug)]
struct PlaygroundFanout {
	step: f32,
}

impl LocalPathFindingFanout for PlaygroundFanout {
	fn local_path_fanout(&self, position: Vec3) -> Vec<Vec3> {
		let s = self.step;
		vec![
			position + Vec3::X * s,
			position - Vec3::X * s,
			position + Vec3::Y * s,
			position - Vec3::Y * s,
		]
	}
}

// --- Markers ---

#[derive(Component)]
struct Chaser;

#[derive(Component)]
struct CursorVisual;

#[derive(Component)]
struct WallVisual;

#[derive(Resource, Default)]
struct CursorWorld(Vec3);

pub struct PathfindingPlaygroundPlugin;

impl Plugin for PathfindingPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<CursorWorld>()
			.add_systems(Startup, setup_scene)
			.add_systems(
				Update,
				(
					update_cursor_world,
					sync_cursor_visual,
					queue_find_path_to_cursor,
					respond_to_find_path_requests::<PlaygroundFanout, PlaygroundSurface>,
					move_chaser_toward_plan,
				)
					.chain(),
			);
	}
}

fn setup_scene(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<ColorMaterial>>,
) {
	commands.spawn(Camera2d);

	let fanout = PlaygroundFanout { step: 28.0 };
	let surface = PlaygroundSurface { wall_x: 0.0 };
	let mut pathfinder = LocalPathfinding::new(fanout, surface);
	pathfinder.depth = 4;
	pathfinder.agent_radius = 12.0;

	let chaser_mesh = meshes.add(Circle::new(14.0));
	let chaser_mat = materials.add(Color::srgb(0.95, 0.15, 0.12));

	commands.spawn((
		Mesh2d(chaser_mesh),
		MeshMaterial2d(chaser_mat),
		Transform::from_xyz(-220.0, 80.0, 2.0),
		Chaser,
		pathfinder,
	));

	let cursor_mesh = meshes.add(Circle::new(12.0));
	let cursor_mat = materials.add(Color::srgb(0.2, 0.45, 0.95));
	commands.spawn((
		Mesh2d(cursor_mesh),
		MeshMaterial2d(cursor_mat),
		Transform::from_xyz(200.0, 120.0, 3.0),
		CursorVisual,
	));

	// Vertical wall slab (visual only; collision matches surface.wall_x)
	let wall_mesh = meshes.add(Rectangle::new(18.0, 520.0));
	let wall_mat = materials.add(Color::srgb(0.35, 0.35, 0.38));
	commands.spawn((
		Mesh2d(wall_mesh),
		MeshMaterial2d(wall_mat),
		Transform::from_xyz(0.0, 0.0, 1.0),
		WallVisual,
	));
}

fn update_cursor_world(
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
	mut cursor_world: ResMut<CursorWorld>,
) {
	let Ok(window) = windows.single() else {
		return;
	};
	let Ok((camera, cam_gt)) = camera_q.single() else {
		return;
	};
	let Some(cursor) = window.cursor_position() else {
		return;
	};
	let Ok(ray) = camera.viewport_to_world(cam_gt, cursor) else {
		return;
	};
	let o = ray.origin;
	let d = *ray.direction;
	if d.z.abs() < 1e-5 {
		return;
	}
	let t = -o.z / d.z;
	let p = o + d * t;
	cursor_world.0 = p;
}

fn sync_cursor_visual(
	cursor_world: Res<CursorWorld>,
	mut q: Query<&mut Transform, With<CursorVisual>>,
) {
	let Ok(mut tf) = q.single_mut() else {
		return;
	};
	tf.translation.x = cursor_world.0.x;
	tf.translation.y = cursor_world.0.y;
}

fn queue_find_path_to_cursor(
	mut commands: Commands,
	cursor_world: Res<CursorWorld>,
	chasers: Query<Entity, With<Chaser>>,
) {
	let Ok(entity) = chasers.single() else {
		return;
	};
	commands.entity(entity).insert(FindPath {
		to_position: Vec3::new(cursor_world.0.x, cursor_world.0.y, 0.0),
	});
}

fn move_chaser_toward_plan(
	time: Res<Time>,
	mut q: Query<(&mut Transform, &LocalPathPlan), With<Chaser>>,
) {
	let speed = 220.0_f32;
	let dt = time.delta_secs();

	for (mut transform, plan) in &mut q {
		let Some(target) = plan.path.positions.get(1).copied().or_else(|| {
			plan.path
				.positions
				.last()
				.copied()
		}) else {
			continue;
		};
		let current = transform.translation;
		let flat_target = Vec3::new(target.x, target.y, current.z);
		let delta = flat_target - current;
		let dist = delta.length();
		if dist < 1.0 {
			continue;
		}
		let step = (speed * dt).min(dist);
		transform.translation += delta.normalize() * step;
	}
}
