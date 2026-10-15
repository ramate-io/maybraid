//! Runtime GPU captures (primary window + Bevy `Screenshot`).

use std::fs;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::time::Virtual;

use crate::camera::{manifest, view_transform, CaptureViewDef};
use crate::export::{collect_groups, write_obj_bundle};
use crate::obj_preview::load_obj_mesh;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CapturePhase {
	Runtime,
	ObjReimport,
}

#[derive(Resource)]
pub struct GpuCaptureJob {
	pub dir: PathBuf,
	pub obj_path: PathBuf,
	pub views: Vec<CaptureViewDef>,
	pub phase: CapturePhase,
	pub index: usize,
	pub settle_frames: u32,
	pub readback_frames: u32,
	pub done: bool,
	/// Set when the last view of a phase finished readback; handled before shooting.
	phase_switch_pending: bool,
	character_root: Entity,
	obj_entity: Option<Entity>,
}

#[derive(Resource)]
struct CaptureRig {
	camera: Entity,
}

pub fn init_gpu_capture(world: &mut World, root: Entity, dir: PathBuf, obj_path: PathBuf) {
	if world.get_resource::<GpuCaptureJob>().is_some() {
		return;
	}
	fs::create_dir_all(&dir).ok();
	if let Some(mut time) = world.get_resource_mut::<Time<Virtual>>() {
		time.pause();
	}
	reveal_character_meshes(world, root);
	// Re-bake OBJ at the frozen pose used for GPU runtime captures.
	let groups = collect_groups(world, root);
	write_obj_bundle(&obj_path, &groups);
	let mut min = Vec3::splat(f32::MAX);
	let mut max = Vec3::splat(f32::MIN);
	for g in &groups {
		for p in &g.positions {
			min = min.min(*p);
			max = max.max(*p);
		}
	}
	let cam_manifest = manifest(min, max);
	let json = serde_json::to_string_pretty(&cam_manifest).expect("cameras json");
	fs::write(dir.join("cameras.json"), json).expect("write cameras.json");

	world.insert_resource(ClearColor(Color::srgb(0.12, 0.13, 0.16)));
	world.insert_resource(GpuCaptureJob {
		dir,
		obj_path,
		views: cam_manifest.views,
		phase: CapturePhase::Runtime,
		index: 0,
		settle_frames: 50,
		readback_frames: 0,
		done: false,
		phase_switch_pending: false,
		character_root: root,
		obj_entity: None,
	});
}

fn reveal_character_meshes(world: &mut World, root: Entity) {
	let mut stack = vec![root];
	while let Some(entity) = stack.pop() {
		world.entity_mut(entity).insert(Visibility::Inherited);
		if let Some(children) = world.get::<Children>(entity) {
			stack.extend(children.iter());
		}
	}
}

fn ensure_capture_rig(world: &mut World, view: &CaptureViewDef) {
	if world.get_resource::<CaptureRig>().is_some() {
		return;
	}
	let camera = world
		.spawn((
			Camera3d::default(),
			Projection::Perspective(PerspectiveProjection {
				fov: crate::camera::CAPTURE_YFOV_RAD,
				..default()
			}),
			view_transform(view),
		))
		.id();
	world.spawn(AmbientLight {
		color: Color::WHITE,
		brightness: 1_200.0,
		..default()
	});
	world.spawn((
		DirectionalLight {
			illuminance: 80_000.0,
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.65, 0.55, 0.0)),
	));
	world.spawn((
		DirectionalLight {
			illuminance: 40_000.0,
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0.35, -2.2, 0.0)),
	));
	world.spawn((
		DirectionalLight {
			illuminance: 25_000.0,
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0.15, 1.05, 0.0)),
	));
	world.insert_resource(CaptureRig { camera });
}

fn spawn_obj_reimport_preview(
	world: &mut World,
	character_root: Entity,
	obj_path: &std::path::Path,
) -> Option<Entity> {
	write_obj_bundle(obj_path, &collect_groups(world, character_root));
	world.entity_mut(character_root).insert(Visibility::Hidden);
	let Some(mesh) = load_obj_mesh(obj_path) else {
		warn!("failed to load OBJ for GPU re-import preview");
		return None;
	};
	let handle = world.resource_mut::<Assets<Mesh>>().add(mesh);
	let mat = world.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
		base_color: Color::srgb(0.78, 0.78, 0.82),
		unlit: true,
		..default()
	});
	Some(
		world
			.spawn((
				Mesh3d(handle),
				MeshMaterial3d(mat),
				Transform::IDENTITY,
				Visibility::Inherited,
			))
			.id(),
	)
}

fn shot_prefix(phase: CapturePhase) -> &'static str {
	match phase {
		CapturePhase::Runtime => "runtime",
		CapturePhase::ObjReimport => "obj_reimport",
	}
}

pub fn tick_gpu_capture(world: &mut World) {
	let mut shoot: Option<(PathBuf, CaptureViewDef)> = None;
	let mut switch_obj = false;
	{
		let Some(mut job) = world.get_resource_mut::<GpuCaptureJob>() else {
			return;
		};
		if job.done {
			return;
		}
		if job.phase_switch_pending {
			if job.phase == CapturePhase::Runtime {
				switch_obj = true;
			} else {
				job.done = true;
			}
			job.phase_switch_pending = false;
		} else if job.readback_frames > 0 {
			job.readback_frames -= 1;
			if job.readback_frames == 0 {
				job.index += 1;
				if job.index >= job.views.len() {
					job.phase_switch_pending = true;
				} else {
					job.settle_frames = 15;
				}
			}
		} else if job.settle_frames > 0 {
			job.settle_frames -= 1;
		} else if job.index < job.views.len() {
			let view = job.views[job.index].clone();
			let prefix = shot_prefix(job.phase);
			let path = job.dir.join(format!("{}_{}.png", prefix, view.name));
			job.readback_frames = 10;
			shoot = Some((path, view));
		}
	}

	if switch_obj {
		let (character_root, obj_path) = {
			let job = world.resource::<GpuCaptureJob>();
			(job.character_root, job.obj_path.clone())
		};
		let obj_entity = spawn_obj_reimport_preview(world, character_root, &obj_path);
		let mut job = world.resource_mut::<GpuCaptureJob>();
		if let Some(entity) = obj_entity {
			job.obj_entity = Some(entity);
			job.phase = CapturePhase::ObjReimport;
			job.index = 0;
			job.settle_frames = 25;
		} else {
			job.done = true;
		}
	}

	if let Some((path, view)) = shoot {
		ensure_capture_rig(world, &view);
		let rig = world.resource::<CaptureRig>();
		world.entity_mut(rig.camera).insert(view_transform(&view));
		world
			.spawn(Screenshot::primary_window())
			.observe(save_to_disk(path.clone()));
		info!("GPU capture: {}", path.display());
	}
}

pub fn gpu_capture_finished(world: &World) -> bool {
	world.get_resource::<GpuCaptureJob>().is_some_and(|j| j.done)
}
