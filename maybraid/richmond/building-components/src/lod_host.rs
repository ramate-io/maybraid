//! Crate-wide warm [`LodSceneHost`] scaffolding (structural LOD roots).
//!
//! This module owns **host graph** helpers: `LodSceneHost` + `LodLevelRoots` + per-level
//! visibility. Domain crates (e.g. partitions) plug in mesh sets / content scenes.
//!
//! Contrast with [`crate::partitions::host`], which maps **resolution GLB sets**
//! (high / mid / low — ultra-low when authored) onto these hosts.
//!
//! Sibling of [`lod::lod_host_scene`](lod::lod_scene_host::lod_host_scene) (lazy single root);
//! these helpers **warm** several level roots up front.

use bevy::prelude::{Children, Component, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::LodSceneLevel;
use lod::lod_scene_host::{LodLevelRoot, LodLevelRoots, LodSceneHost};

use crate::assets::AssetPath;

/// Optional GLB under a transform (e.g. omit content at a far band).
pub fn posed_asset_tier(
	asset: Option<AssetPath>,
	transform: Transform,
) -> impl Scene + 'static {
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

/// Warm host with optional per-level mesh assets.
///
/// Today callers typically pass High / Medium / Low. A fourth
/// [`LodSceneLevel::UltraLow`] root (dedicated ultra-low GLB) is expected once those
/// assets exist; until then UltraLow shares the Low band in domain banding policy.
pub fn warm_mesh_level_host<P: Component + Clone + Default + Unpin>(
	level: LodSceneLevel,
	probe: P,
	transform: Transform,
	roots: impl IntoIterator<Item = (LodSceneLevel, Option<AssetPath>)>,
) -> impl Scene + 'static {
	let root_scenes: Vec<Box<dyn Scene>> = roots
		.into_iter()
		.map(|(root_level, asset)| mesh_level_root(root_level, asset, level == root_level))
		.collect();
	host_with_roots(level, probe, transform, root_scenes)
}

/// Warm host whose level roots are arbitrary scene content (composite IR nodes).
///
/// Pass High / Medium / Low today; add UltraLow when domain content needs a distinct far root.
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
	host_with_roots(level, probe, Transform::IDENTITY, root_scenes)
}

/// Convenience: warm High / Medium / Low content roots (UltraLow not yet a separate root).
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
			(
				LodSceneLevel::High,
				Box::new(high) as Box<dyn Scene>,
			),
			(
				LodSceneLevel::Medium,
				Box::new(mid) as Box<dyn Scene>,
			),
			(LodSceneLevel::Low, Box::new(low) as Box<dyn Scene>),
		],
	)
}

fn host_with_roots<P: Component + Clone + Default + Unpin>(
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
) -> Box<dyn Scene> {
	let children: Vec<Box<dyn Scene>> = match asset {
		Some(a) => vec![Box::new(a.scene_ref().scene())],
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
	content: Box<dyn Scene>,
	visible: bool,
) -> Box<dyn Scene> {
	let children: Vec<Box<dyn Scene>> = vec![content];
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
