//! Shared BSN helpers for composing child scenes.

use bevy::prelude::{Children, Transform, Visibility};
use bevy::scene::prelude::{bsn, Scene};

/// Group child scenes under a hierarchy-safe root.
pub fn scene_children(children: Vec<Box<dyn Scene>>) -> impl Scene + 'static {
	bsn! {
		Transform::default()
		Visibility::default()
		Children [ {children} ]
	}
}
