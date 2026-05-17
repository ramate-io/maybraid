//! Helpers for embedding [`MeshMaterial3d`] in CLI-driven configs without exposing handles as flags yet.

use bevy::prelude::*;

/// Wraps a [`MeshMaterial3d`] so it can participate in `clap` derives via [`clap::Args`] with all fields skipped.
#[derive(Clone, Debug, PartialEq, clap::Args)]
pub struct SkippedMeshMaterial<M: Material> {
	#[arg(skip)]
	pub mesh: MeshMaterial3d<M>,
}

impl<M: Material> Default for SkippedMeshMaterial<M> {
	fn default() -> Self {
		Self { mesh: MeshMaterial3d::default() }
	}
}

impl<M: Material> From<MeshMaterial3d<M>> for SkippedMeshMaterial<M> {
	fn from(mesh: MeshMaterial3d<M>) -> Self {
		Self { mesh }
	}
}

impl<M: Material> From<SkippedMeshMaterial<M>> for MeshMaterial3d<M> {
	fn from(wrapped: SkippedMeshMaterial<M>) -> Self {
		wrapped.mesh
	}
}
