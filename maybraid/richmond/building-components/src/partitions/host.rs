//! Shared warm LOD host / single-tier mesh scene builders for partition kits.

use bevy::prelude::{Children, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::LodSceneLevel;
use lod::lod_scene_host::{LodLevelRoot, LodLevelRoots, LodSceneHost};

use crate::assets::AssetPath;
use crate::partitions::mesh_set::{mesh_child, PartitionMeshSet, PartitionMeshTier};
use crate::partitions::probe::PartitionLodProbe;

/// One MeshRef under a transform (for `scene_with_level`).
pub fn posed_mesh_tier(
	meshes: PartitionMeshSet,
	transform: Transform,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	let tier = match level {
		LodSceneLevel::High => PartitionMeshTier::High,
		LodSceneLevel::Medium => PartitionMeshTier::Mid,
		LodSceneLevel::Low | LodSceneLevel::UltraLow => PartitionMeshTier::Low,
		LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => PartitionMeshTier::Mid,
	};
	posed_asset_tier(Some(meshes.for_tier(tier)), transform)
}

/// Optional asset under a transform (joints omit content at Low).
pub fn posed_asset_tier(
	asset: Option<AssetPath>,
	transform: Transform,
) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = match asset {
		Some(a) => vec![mesh_child(a)],
		None => vec![],
	};
	bsn! {
		template_value(transform)
		Visibility::Inherited
		Children [ {children} ]
	}
}

/// Warm high/mid/low mesh roots under one [`LodSceneHost`].
pub fn warm_mesh_host(
	meshes: PartitionMeshSet,
	transform: Transform,
	level: LodSceneLevel,
	probe: PartitionLodProbe,
) -> impl Scene + 'static {
	warm_host(
		level,
		probe,
		transform,
		[
			(LodSceneLevel::High, Some(meshes.high)),
			(LodSceneLevel::Medium, Some(meshes.mid)),
			(LodSceneLevel::Low, Some(meshes.low)),
		],
	)
}

/// Warm host with optional per-level mesh (e.g. joint high/mid, empty low).
pub fn warm_host(
	level: LodSceneLevel,
	probe: PartitionLodProbe,
	transform: Transform,
	roots: [(LodSceneLevel, Option<AssetPath>); 3],
) -> impl Scene + 'static {
	let root_scenes: Vec<Box<dyn Scene>> = roots
		.into_iter()
		.map(|(root_level, asset)| {
			mesh_level_root(root_level, asset, level == root_level)
		})
		.collect();
	host_with_roots(level, probe, transform, root_scenes)
}

/// Warm host whose level roots are arbitrary scene content (partition parent).
pub fn warm_content_host(
	level: LodSceneLevel,
	probe: PartitionLodProbe,
	high: impl Scene + 'static,
	mid: impl Scene + 'static,
	low: impl Scene + 'static,
) -> impl Scene + 'static {
	let roots = vec![
		content_level_root(LodSceneLevel::High, high, level == LodSceneLevel::High),
		content_level_root(LodSceneLevel::Medium, mid, level == LodSceneLevel::Medium),
		content_level_root(LodSceneLevel::Low, low, level == LodSceneLevel::Low),
	];
	host_with_roots(level, probe, Transform::IDENTITY, roots)
}

fn host_with_roots(
	level: LodSceneLevel,
	probe: PartitionLodProbe,
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
) -> Box<dyn Scene> {
	let children: Vec<Box<dyn Scene>> = match asset {
		Some(a) => vec![mesh_child(a)],
		None => vec![],
	};
	let visibility = if visible {
		Visibility::Inherited
	} else {
		Visibility::Hidden
	};
	Box::new(bsn! {
		template_value(LodLevelRoot(level))
		Transform::default()
		template_value(visibility)
		Children [ {children} ]
	})
}

fn content_level_root(
	level: LodSceneLevel,
	content: impl Scene + 'static,
	visible: bool,
) -> Box<dyn Scene> {
	let children: Vec<Box<dyn Scene>> = vec![Box::new(content)];
	let visibility = if visible {
		Visibility::Inherited
	} else {
		Visibility::Hidden
	};
	Box::new(bsn! {
		template_value(LodLevelRoot(level))
		Transform::default()
		template_value(visibility)
		Children [ {children} ]
	})
}
