//! 2D demo: local pathfinding steers a red agent around a vertical wall toward the cursor (blue).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use intelligence::local_pathfinding::{
	FindPath, LocalPathPlan, LocalPathfinding, LocalPathFindingFanout, LocalPathfindingSurface,
	respond_to_find_path_requests,
};

// --- Pathfinding surface: floor z = 0, finite wall as an axis-aligned box in XY ---

#[derive(Clone, Copy, Debug)]
struct PlaygroundSurface {
	/// Inclusive AABB in the z = 0 plane (min.x ≤ x ≤ max.x, min.y ≤ y ≤ max.y).
	wall_min: Vec2,
	wall_max: Vec2,
}

/// Segment `start + u (end - start)`, u ∈ [0, 1], vs closed AABB. Returns overlap of u with [0, 1].
fn segment_aabb_u_interval(start: Vec2, end: Vec2, min_b: Vec2, max_b: Vec2) -> Option<(f32, f32)> {
	let d = end - start;
	let mut u0 = 0.0_f32;
	let mut u1 = 1.0_f32;

	for axis in 0..2 {
		let (p, delta, mn, mx) = if axis == 0 {
			(start.x, d.x, min_b.x, max_b.x)
		} else {
			(start.y, d.y, min_b.y, max_b.y)
		};

		if delta.abs() < 1e-8 {
			if p < mn || p > mx {
				return None;
			}
			continue;
		}

		let inv = 1.0 / delta;
		let mut t0 = (mn - p) * inv;
		let mut t1 = (mx - p) * inv;
		if t0 > t1 {
			core::mem::swap(&mut t0, &mut t1);
		}
		u0 = u0.max(t0);
		u1 = u1.min(t1);
		if u0 > u1 {
			return None;
		}
	}

	Some((u0, u1))
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

		let s2 = start.xy();
		let e2 = end.xy();
		let Some((t0, t1)) = segment_aabb_u_interval(s2, e2, self.wall_min, self.wall_max) else {
			return len;
		};

		// No intersection with the actual segment.
		if t1 < 0.0 || t0 > 1.0 {
			return len;
		}

		// First contact along the segment from `start` (u = 0 at start, u = 1 at end).
		let u_hit = t0.max(0.0);
		if u_hit > 1.0 {
			return len;
		}

		let dist = u_hit * len;
		-dist
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
	// Centered vertical slab: finite in Y so the agent can plan over/under the ends visually.
	const WALL_HALF_W: f32 = 16.0;
	const WALL_HALF_H: f32 = 140.0;
	let wall_min = Vec2::new(-WALL_HALF_W, -WALL_HALF_H);
	let wall_max = Vec2::new(WALL_HALF_W, WALL_HALF_H);
	let surface = PlaygroundSurface { wall_min, wall_max };
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

	// Wall mesh matches `PlaygroundSurface` AABB (same center and width/height).
	let wall_w = wall_max.x - wall_min.x;
	let wall_h = wall_max.y - wall_min.y;
	let wall_cx = 0.5 * (wall_min.x + wall_max.x);
	let wall_cy = 0.5 * (wall_min.y + wall_max.y);
	let wall_mesh = meshes.add(Rectangle::new(wall_w, wall_h));
	let wall_mat = materials.add(Color::srgb(0.35, 0.35, 0.38));
	commands.spawn((
		Mesh2d(wall_mesh),
		MeshMaterial2d(wall_mat),
		Transform::from_xyz(wall_cx, wall_cy, 1.0),
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
