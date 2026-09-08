//! Per-layer Bucket Throw ([RFC-183 §3.5.2.3]).

use bevy_math::Vec3;
use procedural_common::{BucketThrow, NoiseConfig, NoiseParams};

use crate::{
	ForestGroveKind, ForestLayering, GroundCoverGroveKind, SelectedLayers, WeightedCover,
	WeightedGrove,
};

const GROUND_COVER_LANE: Vec3 = Vec3::new(37.0, 0.0, 0.0);
const TUFTS_LANE: Vec3 = Vec3::new(41.0, 0.0, 0.0);
const UNDERSTORY_LANE: Vec3 = Vec3::new(43.0, 0.0, 0.0);
const LOWER_LANE: Vec3 = Vec3::new(47.0, 0.0, 0.0);
const UPPER_LANE: Vec3 = Vec3::new(53.0, 0.0, 0.0);

/// Throw one layer distribution at the forest-cell center.
pub fn throw_layer(
	buckets: &[WeightedGrove],
	noise: NoiseParams,
	position: Vec3,
	lane: Vec3,
) -> Option<ForestGroveKind> {
	throw_weighted(buckets.iter().map(|bucket| (bucket.kind, bucket.weight)), noise, position, lane)
}

/// Throw the flip-only ground-cover distribution at the forest-cell center.
pub fn throw_cover(
	buckets: &[WeightedCover],
	noise: NoiseParams,
	position: Vec3,
	lane: Vec3,
) -> Option<GroundCoverGroveKind> {
	throw_weighted(buckets.iter().map(|bucket| (bucket.kind, bucket.weight)), noise, position, lane)
}

fn throw_weighted<K: Copy>(
	buckets: impl IntoIterator<Item = (Option<K>, f32)>,
	noise: NoiseParams,
	position: Vec3,
	lane: Vec3,
) -> Option<K> {
	let buckets: Vec<(Option<K>, f32)> = buckets.into_iter().collect();
	if buckets.is_empty() {
		return None;
	}
	let throw = BucketThrow::from_weights(buckets.iter().map(|(_, weight)| *weight), 0.0);
	let n = NoiseConfig::new(noise);
	let sample =
		n.sample_3d(position + lane + crate::hopscotch::SAMPLE_ORIGIN) * throw.total_weight();
	let index = throw.select(sample)?;
	buckets.get(index).and_then(|(kind, _)| *kind)
}

/// Select one grove (or `None`) for each layer of `layering`.
pub fn select_layers(
	layering: &ForestLayering,
	noise: NoiseParams,
	position: Vec3,
) -> SelectedLayers {
	SelectedLayers {
		layering: layering.kind,
		ground_cover: throw_cover(&layering.ground_cover, noise, position, GROUND_COVER_LANE),
		tufts: throw_layer(&layering.tufts, noise, position, TUFTS_LANE),
		understory: throw_layer(&layering.understory, noise, position, UNDERSTORY_LANE),
		lower_canopy: throw_layer(&layering.lower_canopy, noise, position, LOWER_LANE),
		upper_canopy: throw_layer(&layering.upper_canopy, noise, position, UPPER_LANE),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::layerings;
	use anyhow::Result;

	#[test]
	fn ag_town_layer_throw_is_deterministic() -> Result<()> {
		let layering = layerings::ag_town();
		let noise = NoiseParams::from_scalar(2.0, 0.01, 1.0, 1);
		let at = Vec3::new(100.0, 0.0, 40.0);
		assert_eq!(select_layers(&layering, noise, at), select_layers(&layering, noise, at));
		Ok(())
	}

	#[test]
	fn mi_robles_cover_throw_is_allbed_or_none() -> Result<()> {
		let layering = layerings::mi_robles();
		let noise = NoiseParams::from_scalar(2.0, 0.01, 1.0, 1);
		let layers = select_layers(&layering, noise, Vec3::new(100.0, 0.0, 40.0));
		assert!(
			layers.ground_cover.is_none()
				|| layers.ground_cover == Some(crate::GroundCoverGroveKind::Allbed)
				|| layers.ground_cover == Some(crate::GroundCoverGroveKind::GrassyMounds)
		);
		Ok(())
	}
}
