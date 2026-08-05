//! Shared BSN helpers for composing child scenes and poses.

use bevy::prelude::{
	Children, Handle, Mesh, Mesh3d, MeshMaterial3d, StandardMaterial, Transform, Visibility,
};
use bevy::scene::prelude::{bsn, template_value, Scene};

use crate::placed::Placement;

pub fn scene_children(children: Vec<Box<dyn Scene>>) -> impl Scene + 'static {
	bsn! {
		Transform::default()
		Visibility::default()
		Children [ {children} ]
	}
}

pub fn pose(placement: Placement) -> Transform {
	Transform::from_translation(placement.translation)
		.with_rotation(placement.rotation())
		.with_scale(placement.scale)
}

pub fn with_pose(transform: Transform, child: impl Scene + 'static) -> impl Scene + 'static {
	(
		child,
		bsn! {
			template_value(transform)
		},
	)
}

pub fn posed_mesh(
	mesh: Handle<Mesh>,
	material: Handle<StandardMaterial>,
	transform: Transform,
) -> impl Scene + 'static {
	bsn! {
		Mesh3d({mesh})
		MeshMaterial3d::<StandardMaterial>({material})
		template_value(transform)
		Visibility::default()
	}
}
