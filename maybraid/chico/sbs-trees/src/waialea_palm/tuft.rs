//! Optional concealment tuft at the crown tip (RFC §3.1.7.8).

use bevy::prelude::*;
use chico_ball_components::tuft::{SucculentTuft, SucculentTuftShape};
use chico_sbs_geometry::{BallStickChain, WaialeaPalmChain, WaialeaPalmSbs};
use procedural_common::NoiseParams;
use render_item::{CascadeChunk, RenderItem};

pub fn spawn_crown_tuft<LeafM, LeafS>(
	geometry: &WaialeaPalmSbs,
	chain: &BallStickChain<WaialeaPalmChain>,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	root_transform: Transform,
	foliage_noise: &NoiseParams,
	leaf_material: LeafS,
) -> Vec<Entity>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	let tip = WaialeaPalmSbs::trunk_tip_from_chain(chain);
	let local = root_transform
		.rotation
		.inverse()
		.mul_vec3(tip - root_transform.translation);
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

	tuft.spawn_render_items(
		commands,
		cascade_chunk,
		Transform {
			translation: local,
			scale: Vec3::splat(scale),
			..default()
		},
	)
}
