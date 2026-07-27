//! Shared BSN helpers for composing child scenes.

use bevy::prelude::{Children, Transform, Visibility};
use bevy::scene::prelude::{bsn, Scene};

/// Group child scenes under a hierarchy-safe root.
///
/// Bevy B0004: ancestors of entities with [`Transform`] / [`Visibility`] must
/// also carry those components, or `GlobalTransform` / visibility inheritance
/// breaks (meshes pile at the origin / vanish).
pub fn scene_children(children: Vec<Box<dyn Scene>>) -> impl Scene + 'static {
	bsn! {
		Transform::default()
		Visibility::default()
		Children [ {children} ]
	}
}
