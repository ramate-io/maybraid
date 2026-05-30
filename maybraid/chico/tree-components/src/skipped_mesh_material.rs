//! Skipped [`MeshMaterial3d`] wrappers for CLI flattening (distinct clap group ids per slot).

use bevy::prelude::*;

macro_rules! define_skipped_mesh_material {
	($name:ident, $group_id:literal) => {
		#[derive(Clone, Debug, PartialEq)]
		#[cfg_attr(feature = "clap", derive(clap::Args))]
		#[cfg_attr(feature = "clap", group(id = $group_id))]
		pub struct $name<M: Material> {
			#[cfg_attr(feature = "clap", arg(skip))]
			pub mesh: MeshMaterial3d<M>,
		}

		impl<M: Material> Default for $name<M> {
			fn default() -> Self {
				Self { mesh: MeshMaterial3d::default() }
			}
		}

		impl<M: Material> From<MeshMaterial3d<M>> for $name<M> {
			fn from(mesh: MeshMaterial3d<M>) -> Self {
				Self { mesh }
			}
		}

		impl<M: Material> From<$name<M>> for MeshMaterial3d<M> {
			fn from(wrapped: $name<M>) -> Self {
				wrapped.mesh
			}
		}
	};
}

define_skipped_mesh_material!(SkippedBodyMeshMaterial, "jungle-growth-body-material");
define_skipped_mesh_material!(SkippedFoliageMeshMaterial, "jungle-growth-foliage-material");
