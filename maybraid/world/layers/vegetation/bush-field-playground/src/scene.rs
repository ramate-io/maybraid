//! Isolated bush, dense patch, slope, and leafless wood.

use bevy::prelude::*;
use vegetation_bush_field::{BushPatch, BushRecipe};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchKind {
	Close,
	Dense,
	Slope,
	Leafless,
	Sparse,
}

pub fn spawn_default_scene(mut commands: Commands) {
	for kind in [PatchKind::Dense, PatchKind::Close, PatchKind::Slope, PatchKind::Leafless] {
		commands.spawn((Name::new(kind.label()), kind, patch_for(kind)));
	}
}

pub fn spawn_default_scene_in_world(world: &mut World) {
	for kind in [PatchKind::Dense, PatchKind::Close, PatchKind::Slope, PatchKind::Leafless] {
		spawn_kind(world, kind);
	}
}

pub fn clear_patches(world: &mut World) {
	let ids: Vec<_> = world.query_filtered::<Entity, With<BushPatch>>().iter(world).collect();
	for id in ids {
		world.entity_mut(id).despawn();
	}
}

pub fn has_patches(world: &mut World) -> bool {
	world.query_filtered::<Entity, With<BushPatch>>().iter(world).next().is_some()
}

pub fn spawn_kind(world: &mut World, kind: PatchKind) {
	world.spawn((Name::new(kind.label()), kind, patch_for(kind)));
}

pub fn spawn_close_only(world: &mut World) {
	clear_patches(world);
	spawn_kind(world, PatchKind::Close);
}

pub fn spawn_leafless_only(world: &mut World) {
	clear_patches(world);
	spawn_kind(world, PatchKind::Leafless);
}

fn patch_for(kind: PatchKind) -> BushPatch {
	match kind {
		PatchKind::Close => BushPatch::isolated(11, Vec3::new(3.0, 0.0, 3.0), BushRecipe::small_scrub()),
		PatchKind::Dense => BushPatch::dense_scrub(7, Vec3::new(1.0, 0.0, 1.0), Vec2::splat(10.0), 1.15),
		PatchKind::Slope => BushPatch::sloped_scrub(
			19,
			Vec3::new(14.0, 0.3, 0.5),
			Vec2::splat(8.0),
			1.4,
			Vec3::new(-0.28, 1.0, 0.0),
		),
		PatchKind::Leafless => BushPatch::leafless_wood(23, Vec3::new(6.5, 0.0, 2.2)),
		PatchKind::Sparse => BushPatch::sparse_wood(13, Vec3::new(1.0, 0.0, 1.0), Vec2::splat(8.0), 1.6),
	}
}

impl PatchKind {
	pub fn label(self) -> &'static str {
		match self {
			Self::Close => "close",
			Self::Dense => "dense",
			Self::Slope => "slope",
			Self::Leafless => "leafless",
			Self::Sparse => "sparse",
		}
	}
}
