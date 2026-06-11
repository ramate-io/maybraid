//! Optional concealment tuft at the crown origin (RFC §3.1.7.10).

use bevy::prelude::*;
use chico_ball_components::tuft::{SucculentTuft, SucculentTuftShape};
use chico_sbs_geometry::PalmBushSbs;
use render_item::{CascadeChunk, RenderItem};

pub fn spawn_crown_tuft<LeafM, LeafS>(
	geometry: &PalmBushSbs,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	parent: Entity,
	leaf_material: LeafS,
) -> Vec<Entity>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	let foliage_noise = &geometry.foliage_noise;
	let scale = geometry.crown_tuft_world_scale();
	let tuft = SucculentTuft::from_shape(
		SucculentTuftShape {
			seed: foliage_noise.seed.wrapping_add(91),
			element_count: 8,
			noise_frequency: foliage_noise.frequency,
			noise_amplitude: foliage_noise.amplitude,
			..SucculentTuftShape::default()
		},
		leaf_material,
	);

	tuft.spawn_render_items_under(
		commands,
		cascade_chunk,
		Transform {
			translation: geometry.crown_origin(),
			scale: Vec3::splat(scale),
			..default()
		},
		Some(parent),
	)
}
