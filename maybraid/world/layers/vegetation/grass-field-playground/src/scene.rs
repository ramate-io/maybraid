//! Isolated clump, dense meadow, slope, hill-crest, and parting-validation patches.

use bevy::prelude::*;
use vegetation_grass_field::{GrassPartingOccluder, GrassPatch, GrassSpecies};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchKind {
	Clump,
	Meadow,
	Slope,
	Crest,
	Parting,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartingProp;

/// Ground-level camera inside the grass, looking through blades at a character capsule.
pub const PARTING_GROUND: (Vec3, Vec3) =
	(Vec3::new(8.05, 0.38, 8.85), Vec3::new(10.4, 0.55, 11.35));
/// Over-the-shoulder, same subject.
pub const PARTING_OTS: (Vec3, Vec3) = (Vec3::new(7.15, 1.28, 8.05), Vec3::new(10.35, 0.85, 11.4));
/// Top-down over the mid-distance capsule (~5.9 m up, ~85° pitch).
/// Small −Z offset keeps `looking_at` well-defined; 55° FOV frames ~6 m of ground.
pub const PARTING_TOPDOWN: (Vec3, Vec3) =
	(Vec3::new(9.62, 5.9, 10.00), Vec3::new(9.62, 0.05, 10.54));

pub fn parting_capsule_at(distance_behind_m: f32) -> Vec3 {
	let grass = Vec3::new(8.7, 0.0, 9.55);
	let dir = Vec3::new(0.68, 0.0, 0.73).normalize();
	grass + dir * distance_behind_m + Vec3::Y * 0.9
}

pub fn parting_rock_at(distance_behind_m: f32) -> Vec3 {
	let grass = Vec3::new(8.15, 0.0, 9.85);
	let dir = Vec3::new(0.55, 0.0, 0.84).normalize();
	grass + dir * distance_behind_m + Vec3::Y * 0.28
}

pub fn spawn_default_scene(mut commands: Commands) {
	for kind in [PatchKind::Meadow, PatchKind::Clump, PatchKind::Slope, PatchKind::Crest] {
		commands.spawn((Name::new(kind.label()), kind, patch_for(kind)));
	}
}

pub fn spawn_default_scene_in_world(world: &mut World) {
	for kind in [PatchKind::Meadow, PatchKind::Clump, PatchKind::Slope, PatchKind::Crest] {
		spawn_kind(world, kind);
	}
}

pub fn clear_patches(world: &mut World) {
	let ids: Vec<_> = world.query_filtered::<Entity, With<GrassPatch>>().iter(world).collect();
	for id in ids {
		world.entity_mut(id).despawn();
	}
}

pub fn has_patches(world: &mut World) -> bool {
	world.query_filtered::<Entity, With<GrassPatch>>().iter(world).next().is_some()
}

pub fn spawn_kind(world: &mut World, kind: PatchKind) {
	world.spawn((Name::new(kind.label()), kind, patch_for(kind)));
}

pub fn clear_parting_props(world: &mut World) {
	let ids: Vec<_> = world.query_filtered::<Entity, With<PartingProp>>().iter(world).collect();
	for id in ids {
		world.entity_mut(id).despawn();
	}
}

pub fn spawn_parting_scene_in_world(
	world: &mut World,
	capsule_distance_m: f32,
	rock_distance_m: f32,
) {
	spawn_parting_scene_with_trail(world, capsule_distance_m, rock_distance_m, 0);
}

/// Extra character capsules along the approach, for raster trail stills.
pub fn spawn_parting_scene_with_trail(
	world: &mut World,
	capsule_distance_m: f32,
	rock_distance_m: f32,
	trail_steps: u32,
) {
	clear_patches(world);
	clear_parting_props(world);
	spawn_kind(world, PatchKind::Parting);
	let capsule = parting_capsule_at(capsule_distance_m);
	let rock = parting_rock_at(rock_distance_m);
	let capsule_mesh = world.resource_mut::<Assets<Mesh>>().add(Capsule3d::new(0.4, 1.0));
	let rock_mesh = world.resource_mut::<Assets<Mesh>>().add(Sphere::new(0.32));
	let skin = world.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
		base_color: Color::srgb(0.86, 0.54, 0.34),
		perceptual_roughness: 0.72,
		..default()
	});
	let stone = world.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
		base_color: Color::srgb(0.40, 0.36, 0.32),
		perceptual_roughness: 0.9,
		..default()
	});
	spawn_parting_capsule(world, capsule, capsule_mesh.clone(), skin.clone());
	for step in 1..=trail_steps {
		let along = capsule_distance_m + trail_step_offset_m(step);
		if along < 0.15 {
			continue;
		}
		spawn_parting_capsule(world, parting_capsule_at(along), capsule_mesh.clone(), skin.clone());
	}
	world.spawn((
		Name::new("parting-rock"),
		PartingProp,
		GrassPartingOccluder::rock(0.32),
		Mesh3d(rock_mesh),
		MeshMaterial3d(stone),
		Transform::from_translation(rock),
	));
}

fn trail_step_offset_m(step: u32) -> f32 {
	let pair = ((step + 1) / 2) as f32;
	let sign = if step % 2 == 1 { -1.0 } else { 1.0 };
	sign * pair * 0.55
}

fn spawn_parting_capsule(
	world: &mut World,
	translation: Vec3,
	mesh: Handle<Mesh>,
	material: Handle<StandardMaterial>,
) {
	world.spawn((
		Name::new("parting-capsule"),
		PartingProp,
		GrassPartingOccluder::character(),
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_translation(translation),
	));
}

fn patch_for(kind: PatchKind) -> GrassPatch {
	match kind {
		PatchKind::Clump => GrassPatch::isolated_clump(11, Vec3::new(3.0, 0.0, 3.0), 96),
		PatchKind::Meadow => {
			GrassPatch::dense_field(7, Vec3::new(4.0, 0.0, 4.0), Vec2::splat(16.0), 0.10)
		}
		PatchKind::Slope => GrassPatch::sloped_field(
			19,
			Vec3::new(22.0, 0.4, 2.0),
			Vec2::splat(10.0),
			0.13,
			Vec3::new(-0.28, 1.0, 0.0),
		),
		PatchKind::Crest => GrassPatch::hill_crest(23, Vec3::new(12.0, 0.0, 22.0), 14.0, 2.4, 140),
		PatchKind::Parting => {
			GrassPatch::dense_field(41, Vec3::new(7.15, 0.0, 8.05), Vec2::new(5.6, 5.6), 0.085)
				.with_species(GrassSpecies::shade().with_coverage_height(1.12))
		}
	}
}

impl PatchKind {
	pub fn label(self) -> &'static str {
		match self {
			Self::Clump => "clump",
			Self::Meadow => "meadow",
			Self::Slope => "slope",
			Self::Crest => "crest",
			Self::Parting => "parting",
		}
	}
}
