//! CLI-friendly bucket weight overrides for grove distributions.

use super::distribution::GroveDistribution;

/// Per-bucket weight overrides aligned with [`GroveDistribution::buckets`] order.
///
/// `None` at index *i* keeps the authored weight for bucket *i*; `Some(w)` replaces it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VariantWeightOverrides {
	pub slots: Vec<Option<f32>>,
}

impl VariantWeightOverrides {
	pub fn apply_to<V>(&self, distribution: &mut GroveDistribution<V>) -> Result<(), String> {
		if self.slots.len() != distribution.buckets.len() {
			return Err(format!(
				"variant weight count {} does not match bucket count {}",
				self.slots.len(),
				distribution.buckets.len()
			));
		}
		for (slot, bucket) in self.slots.iter().zip(&mut distribution.buckets) {
			if let Some(weight) = slot {
				bucket.weight = *weight;
			}
		}
		Ok(())
	}
}

/// Parse `--variant-weights 1.0,x,2.5,3.0,x` (one slot per bucket; `x` keeps the default).
pub fn parse_variant_weights(s: &str) -> Result<VariantWeightOverrides, String> {
	let slots = s
		.split(',')
		.map(str::trim)
		.filter(|part| !part.is_empty())
		.map(parse_variant_weight_slot)
		.collect::<Result<Vec<_>, _>>()?;
	if slots.is_empty() {
		return Err("expected at least one variant weight slot".into());
	}
	Ok(VariantWeightOverrides { slots })
}

fn parse_variant_weight_slot(part: &str) -> Result<Option<f32>, String> {
	if part.eq_ignore_ascii_case("x") {
		return Ok(None);
	}
	part.parse::<f32>()
		.map(Some)
		.map_err(|e| format!("invalid variant weight {part:?}: {e}"))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::BraidGrassCell;
	use anyhow::Result;

	#[test]
	fn parse_variant_weights_accepts_defaults_and_overrides() -> Result<()> {
		let overrides =
			parse_variant_weights("1.0,x,2.5,3.0,x").map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(overrides.slots, vec![Some(1.0), None, Some(2.5), Some(3.0), None]);
		Ok(())
	}

	#[test]
	fn apply_variant_weights_updates_only_set_slots() -> Result<()> {
		let mut distribution = BraidGrassCell::grove_distribution();
		let authored_deep_green = distribution.buckets[1].weight;
		let authored_jungle = distribution.buckets[3].weight;

		let overrides =
			parse_variant_weights("0.1,x,4.0,x,x").map_err(|e| anyhow::anyhow!("{e}"))?;
		overrides.apply_to(&mut distribution).map_err(|e| anyhow::anyhow!("{e}"))?;

		assert_eq!(distribution.buckets[0].weight, 0.1);
		assert_eq!(distribution.buckets[1].weight, authored_deep_green);
		assert_eq!(distribution.buckets[2].weight, 4.0);
		assert_eq!(distribution.buckets[3].weight, authored_jungle);
		Ok(())
	}

	#[test]
	fn apply_variant_weights_rejects_length_mismatch() -> Result<()> {
		let mut distribution = BraidGrassCell::grove_distribution();
		let overrides = VariantWeightOverrides { slots: vec![Some(1.0)] };
		assert!(overrides.apply_to(&mut distribution).is_err());
		Ok(())
	}
}
