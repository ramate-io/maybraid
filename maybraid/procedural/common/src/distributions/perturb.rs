//! Deterministic bucket **weight perturbation** ([RFC-183 3.4.2.1.2]).

use super::bucket_throw::BucketThrow;

/// Default floor for perturbed bucket weights.
pub const MIN_BUCKET_WEIGHT: f32 = 1e-4;

/// Multiply each bucket weight by `(1 + noise * strength)`, clamp, then renormalize to preserve
/// the original total ordering span.
pub fn perturb_weights(
	base: &BucketThrow,
	perturbation_strength: f32,
	bucket_noises: &[f32],
	min_weight: f32,
) -> BucketThrow {
	let len = base.len();
	if len == 0 {
		return BucketThrow::new();
	}

	let total = base.total_weight();
	let mut weights = Vec::with_capacity(len);
	for index in 0..len {
		let weight = base.weight_at(index).unwrap_or(0.0);
		let noise = bucket_noises.get(index).copied().unwrap_or(0.0);
		let perturbed = (weight * (1.0 + noise * perturbation_strength)).max(min_weight);
		weights.push(perturbed);
	}

	let sum: f32 = weights.iter().sum();
	if sum <= 0.0 || !sum.is_finite() {
		return base.clone();
	}

	let scale = total / sum;
	BucketThrow::from_weights(
		weights.into_iter().map(|w| w * scale),
		base.mean_anchor(),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn preserves_bucket_count() -> Result<()> {
		let mut base = BucketThrow::new();
		base.add(2.0);
		base.add(1.0);
		base.add(3.0);
		let perturbed = perturb_weights(&base, 0.5, &[0.5, -0.5, 0.0], MIN_BUCKET_WEIGHT);
		assert_eq!(perturbed.len(), 3);
		Ok(())
	}

	#[test]
	fn renormalizes_to_original_total() -> Result<()> {
		let mut base = BucketThrow::new();
		base.add(1.0);
		base.add(2.0);
		let perturbed = perturb_weights(&base, 1.0, &[1.0, -1.0], MIN_BUCKET_WEIGHT);
		assert!((perturbed.total_weight() - base.total_weight()).abs() < 1e-5);
		Ok(())
	}
}
