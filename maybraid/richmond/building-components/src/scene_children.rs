//! Shared BSN helpers for composing child scenes and poses.

use bevy::prelude::{
	Children, Handle, Mesh, Mesh3d, MeshMaterial3d, StandardMaterial, Transform, Visibility,
};
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::Quat;

use crate::assets::AssetPath;
use crate::placed::Placement;

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

/// Build a [`Transform`] from a cell-space [`Placement`].
pub fn pose(placement: Placement) -> Transform {
	Transform::from_translation(placement.translation)
		.with_rotation(Quat::from_rotation_y(placement.yaw))
		.with_scale(placement.scale)
}

/// GLB asset with an applied pose transform.
pub fn posed_glb(asset: AssetPath, transform: Transform) -> impl Scene + 'static {
	(
		asset.mesh_ref().scene(),
		bsn! {
			template_value(transform)
		},
	)
}

/// Wrap a child scene with an applied pose transform.
pub fn with_pose(transform: Transform, child: impl Scene + 'static) -> impl Scene + 'static {
	(
		child,
		bsn! {
			template_value(transform)
		},
	)
}

/// Posed line-list cube using pre-registered mesh/material handles.
///
/// Unit mesh spans \([-0.5, 0.5]^3\); use transform scale as full edge lengths.
pub fn wireframe_box_with_handles(
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
