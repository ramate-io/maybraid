//! Isolated clump, dense meadow, slope, and hill-crest patches.

use bevy::prelude::*;
use vegetation_grass_field::GrassPatch;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchKind {
	Clump,
	Meadow,
	Slope,
	Crest,
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
	}
}

impl PatchKind {
	pub fn label(self) -> &'static str {
		match self {
			Self::Clump => "clump",
			Self::Meadow => "meadow",
			Self::Slope => "slope",
			Self::Crest => "crest",
		}
	}
}
