//! Warm [`LodSceneHost`] scaffolding for vegetation content.

use bevy::prelude::{Children, Component, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::LodSceneLevel;
use lod::lod_scene_host::{LodLevelRoot, LodLevelRoots, LodSceneHost};

use crate::assets::AssetPath;

/// Marks a foliage GLB [`scene_ref::SceneRefRoot`] subtree for playground material patching.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct VegetationFoliageAssetRoot;

/// Optional GLB under a transform.
pub fn posed_asset_tier(asset: Option<AssetPath>, transform: Transform) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = match asset {
		Some(a) => vec![Box::new(a.scene_ref().scene())],
		None => vec![],
	};
	bsn! {
		template_value(transform)
		Visibility::Inherited
		Children [ {children} ]
	}
}

fn foliage_asset_scene(asset: AssetPath) -> impl Scene + 'static {
	let scene = asset.scene_ref().scene();
	(
		bsn! { VegetationFoliageAssetRoot },
		scene,
	)
}

/// Foliage GLB under a transform, tagged with [`VegetationFoliageAssetRoot`].
pub fn posed_foliage_asset_tier(
	asset: Option<AssetPath>,
	transform: Transform,
) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = match asset {
		Some(a) => vec![Box::new(foliage_asset_scene(a))],
		None => vec![],
	};
	bsn! {
		template_value(transform)
		Visibility::Inherited
		Children [ {children} ]
	}
}

/// Warm host whose level roots are arbitrary scene content.
pub fn warm_content_host<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	roots: impl IntoIterator<Item = (LodSceneLevel, Box<dyn Scene>)>,
) -> impl Scene + 'static {
	let root_scenes: Vec<Box<dyn Scene>> = roots
		.into_iter()
		.map(|(root_level, content)| {
			let visible = level == root_level;
			content_level_root(root_level, content, visible)
		})
		.collect();
	host_with_probe_only(level, probe, Transform::IDENTITY, root_scenes)
}

pub fn warm_content_host_hsl<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	high: impl Scene + 'static,
	mid: impl Scene + 'static,
	low: impl Scene + 'static,
) -> impl Scene + 'static {
	warm_content_host(
		level,
		probe,
		[
			(LodSceneLevel::High, Box::new(high) as Box<dyn Scene>),
			(LodSceneLevel::Medium, Box::new(mid) as Box<dyn Scene>),
			(LodSceneLevel::Low, Box::new(low) as Box<dyn Scene>),
		],
	)
}

/// Warm host with optional per-level mesh assets.
pub fn warm_mesh_level_host<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	transform: Transform,
	roots: impl IntoIterator<Item = (LodSceneLevel, Option<AssetPath>)>,
) -> impl Scene + 'static {
	let root_list: Vec<(LodSceneLevel, Option<AssetPath>)> = roots.into_iter().collect();
	let root_scenes: Vec<Box<dyn Scene>> = root_list
		.iter()
		.map(|(root_level, asset)| mesh_level_root(*root_level, *asset, level == *root_level, false))
		.collect();
	host_with_probe_only(level, probe, transform, root_scenes)
}

/// Warm host for foliage GLB LOD triads (roots tagged [`VegetationFoliageAssetRoot`]).
pub fn warm_foliage_mesh_level_host<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	transform: Transform,
	roots: impl IntoIterator<Item = (LodSceneLevel, Option<AssetPath>)>,
) -> impl Scene + 'static {
	let root_list: Vec<(LodSceneLevel, Option<AssetPath>)> = roots.into_iter().collect();
	let root_scenes: Vec<Box<dyn Scene>> = root_list
		.iter()
		.map(|(root_level, asset)| mesh_level_root(*root_level, *asset, level == *root_level, true))
		.collect();
	host_with_probe_only(level, probe, transform, root_scenes)
}

fn host_with_probe_only<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	transform: Transform,
	roots: Vec<Box<dyn Scene>>,
) -> impl Scene + 'static {
	let level_roots: Box<dyn Scene> = Box::new(bsn! {
		LodLevelRoots
		Transform::default()
		Visibility::Inherited
		Children [ {roots} ]
	});
	let host_children = vec![level_roots];
	bsn! {
		LodSceneHost
		template_value(level)
		template_value(probe)
		template_value(transform)
		Visibility::Inherited
		Children [ {host_children} ]
	}
}

fn mesh_level_root(
	level: LodSceneLevel,
	asset: Option<AssetPath>,
	visible: bool,
	foliage: bool,
) -> Box<dyn Scene> {
	let children: Vec<Box<dyn Scene>> = match asset {
		Some(a) => {
			if foliage {
				vec![Box::new(foliage_asset_scene(a))]
			} else {
				vec![Box::new(a.scene_ref().scene())]
			}
		}
		None => vec![],
	};
	let visibility = if visible { Visibility::Inherited } else { Visibility::Hidden };
	Box::new(bsn! {
		template_value(LodLevelRoot(level))
		Transform::default()
		template_value(visibility)
		Children [ {children} ]
	})
}

fn content_level_root(
	level: LodSceneLevel,
	content: Box<dyn Scene>,
	visible: bool,
) -> Box<dyn Scene> {
	let children: Vec<Box<dyn Scene>> = vec![content];
	let visibility = if visible { Visibility::Inherited } else { Visibility::Hidden };
	Box::new(bsn! {
		template_value(LodLevelRoot(level))
		Transform::default()
		template_value(visibility)
		Children [ {children} ]
	})
}
