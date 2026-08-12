//! [`LodHostHelper`]: posed kit content for one LOD level root.
//!
//! Domain nodes are the [`lod::LodScene`] hosts (see [`crate::PartitionNode`], …); this
//! helper only builds the **content** a level root holds. Host scaffolding itself comes
//! from [`lod::lod_host_scene_pending`] via [`lod::LodScene::host`], so a node nested
//! under a parent streams its level through chunk fulfill instead of warming every band.
//!
//! Contrast with [`crate::partitions::host`], which maps **resolution GLB sets**
//! (high / mid / low — ultra-low when authored) onto a [`lod::gen::LodSceneLevel`].

use bevy::prelude::{Children, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use scene_ref::SceneRef;

use crate::assets::AssetPath;

/// Posed kit / GLB content helpers for LOD level-root payloads (not host scaffolding).
pub struct LodHostHelper;

impl LodHostHelper {
	/// Optional GLB under a transform (e.g. omit content at a far band).
	pub fn posed_asset_tier(
		asset: Option<AssetPath>,
		transform: Transform,
	) -> impl Scene + 'static {
		Self::posed_scene_ref_tier(asset.map(AssetPath::scene_ref), transform)
	}

	/// Optional [`SceneRef`] under a transform (mirrored kits pass an explicit ref).
	pub fn posed_scene_ref_tier(
		scene_ref: Option<SceneRef>,
		transform: Transform,
	) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = match scene_ref {
			Some(r) => vec![Box::new(r.scene())],
			None => vec![],
		};
		bsn! {
			template_value(transform)
			Visibility::Inherited
			Children [ {children} ]
		}
	}
}
