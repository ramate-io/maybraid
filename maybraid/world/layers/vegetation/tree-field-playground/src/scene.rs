//! Isolated canopy, grove edge, slope, LOD row, and leafless skeleton.

use bevy::prelude::*;
use vegetation_tree_field::{TreePatch, TreeRecipe};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchKind {
	Close,
	Grove,
	Slope,
	Leafless,
	Lod,
	Sparse,
}

pub fn spawn_default_scene(mut commands: Commands) {
	for kind in [PatchKind::Grove, PatchKind::Close, PatchKind::Slope, PatchKind::Leafless] {
		commands.spawn((Name::new(kind.label()), kind, patch_for(kind)));
	}
}

pub fn spawn_default_scene_in_world(world: &mut World) {
	for kind in [PatchKind::Grove, PatchKind::Close, PatchKind::Slope, PatchKind::Leafless] {
		spawn_kind(world, kind);
	}
}

pub fn clear_patches(world: &mut World) {
	let ids: Vec<_> = world.query_filtered::<Entity, With<TreePatch>>().iter(world).collect();
	for id in ids {
		world.entity_mut(id).despawn();
	}
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

fn patch_for(kind: PatchKind) -> TreePatch {
	match kind {
		PatchKind::Close => TreePatch::isolated(11, Vec3::ZERO, TreeRecipe::young_storybook()),
		PatchKind::Grove => TreePatch::grove(7, Vec3::new(8.0, 0.0, 8.0), Vec2::splat(36.0), 14.0),
		PatchKind::Slope => TreePatch::sloped_grove(
			19,
			Vec3::new(24.0, 0.6, -2.0),
			Vec2::splat(22.0),
			14.0,
			Vec3::new(-0.28, 1.0, 0.0),
		),
		PatchKind::Leafless => TreePatch::leafless_wood(23, Vec3::new(16.0, 0.0, -8.0)),
		PatchKind::Lod => TreePatch::from_grove_placements(
			13,
			&[
				(Vec3::ZERO, 1.0),
				(Vec3::new(42.0, 0.0, 8.0), 1.05),
				(Vec3::new(88.0, 0.0, 14.0), 0.95),
			],
			TreeRecipe::storybook(),
		),
		PatchKind::Sparse => TreePatch::sparse_wood(17, Vec3::new(8.0, 0.0, 8.0), Vec2::splat(28.0), 16.0),
	}
}

impl PatchKind {
	pub fn label(self) -> &'static str {
		match self {
			Self::Close => "close",
			Self::Grove => "grove",
			Self::Slope => "slope",
			Self::Leafless => "leafless",
			Self::Lod => "lod",
			Self::Sparse => "sparse",
		}
	}
}
