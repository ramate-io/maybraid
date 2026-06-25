//! **Palm Bush** — trunkless ground-anchored frond cluster ([#231](https://github.com/ramate-io/maybraid/issues/231), [RFC §3.1.7.10](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/10-palm-bush/README.md)).

mod crown;
pub mod render_item_plugin;
mod tuft;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::PalmBushSbs;
use clap::Args;
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::SkippedLeafMeshMaterial;
use crown::spawn_crown_rings;
use tuft::spawn_crown_tuft;

/// Typical [`StandardMaterial`] Palm Bush using CLI-skipped leaf handles.
pub type PalmBushStd = PalmBush<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;

/// Foliage noise (seed, surface frequency / amplitude) lives on
/// [`PalmBushSbs::foliage_noise`]; hoist per-instance noise in with
/// [`PalmBushSbs::with_noise_params`].
#[derive(Component, Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PalmBush<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: PalmBushSbs,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	#[arg(skip)]
	__marker: PhantomData<fn() -> LeafM>,
}

impl<LeafM, LeafS> Default for PalmBush<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			geometry: PalmBushSbs::default(),
			leaf_material: LeafS::default(),
			__marker: PhantomData,
		}
	}
}

impl<LeafM, LeafS> PalmBush<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	pub fn new(geometry: PalmBushSbs, leaf_material: LeafS) -> Self {
		Self { geometry, leaf_material, __marker: PhantomData }
	}
}

impl<LeafM, LeafS> RenderItem for PalmBush<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let root = commands
			.spawn((self.clone(), cascade_chunk.clone(), transform, Visibility::default()))
			.id();

		spawn_crown_rings::<LeafM, LeafS>(
			&self.geometry,
			commands,
			cascade_chunk,
			root,
			self.leaf_material.clone(),
		);

		spawn_crown_tuft::<LeafM, LeafS>(
			&self.geometry,
			commands,
			cascade_chunk,
			root,
			self.leaf_material.clone(),
		);

		vec![root]
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_geometry_ring_count_matches_crown_params() {
		let bush =
			PalmBush::<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>::default();
		assert_eq!(bush.geometry.crown.ring_count, 8);
		assert_eq!(bush.geometry.crown.fronds_per_ring, 12);
	}
}
