//! External vs internal geometry gating relative to a parent volume.
//!
//! Used as an IR field on building-component nodes (`FloorNode`, `WallNode`, …).
//! Internal balls should be scaled to envelop a large open interior so detail
//! only appears once the viewer is well inside that space.

use bevy::math::Vec3;
use bevy::prelude::{Children, Component, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};

/// Whether a node is part of the external silhouette or internal detail.
#[derive(Debug, Clone, Copy, PartialEq, Component, Default)]
pub enum ParentConfines {
	/// Liberal footprint / extent banding — typically always a LOD candidate.
	#[default]
	External,
	/// Hidden until the viewer is well inside `center`+`radius`.
	Internal {
		center: Vec3,
		radius: f32,
	},
}

impl ParentConfines {
	pub fn internal(center: Vec3, radius: f32) -> Self {
		Self::Internal {
			center,
			radius: radius.max(1e-4),
		}
	}

	/// Whether the viewer may activate this node's detail for this confine.
	pub fn viewer_allowed(&self, viewer: &Transform) -> bool {
		match self {
			Self::External => true,
			Self::Internal { center, radius } => {
				viewer.translation.distance(*center) <= *radius
			}
		}
	}
}

/// Wrap `content` with a [`ParentConfines`] root so fine-phase visibility can gate it.
pub fn confined_scene(
	confines: ParentConfines,
	content: impl Scene + 'static,
) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = vec![Box::new(content)];
	bsn! {
		template_value(confines)
		Transform::default()
		Visibility::Inherited
		Children [ {children} ]
	}
}

/// Fine-phase: hide entities whose [`ParentConfines`] reject the camera.
pub fn apply_parent_confines(
	viewer: bevy::prelude::Query<&Transform, bevy::prelude::With<bevy::prelude::Camera3d>>,
	mut hosts: bevy::prelude::Query<(&ParentConfines, &mut Visibility)>,
) {
	let Ok(viewer_tf) = viewer.single() else {
		return;
	};
	for (confines, mut visibility) in &mut hosts {
		*visibility = if confines.viewer_allowed(viewer_tf) {
			Visibility::Inherited
		} else {
			Visibility::Hidden
		};
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Transform;

	#[test]
	fn internal_confine_radius() -> anyhow::Result<()> {
		let c = ParentConfines::internal(Vec3::ZERO, 5.0);
		assert!(c.viewer_allowed(&Transform::from_xyz(3.0, 0.0, 0.0)));
		assert!(!c.viewer_allowed(&Transform::from_xyz(6.0, 0.0, 0.0)));
		assert!(ParentConfines::External.viewer_allowed(&Transform::from_xyz(100.0, 0.0, 0.0)));
		Ok(())
	}
}
