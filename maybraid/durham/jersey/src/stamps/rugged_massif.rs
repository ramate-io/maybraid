//! Jersey Rugged Massifs (unchained) — [RFC-105 §3.8.3](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#383-jersey-rugged-massifs-unchained).

use crate::config::{FractalAnchors, HysteresisSpine, SoftmaskAlongSpine};
use crate::region::RegionNoise;
use crate::stamp::{StampSemantics, StampSet};
use bevy_math::Vec2;
use procedural_common::{Bounds2, SeededHash};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassifStyle {
	Ridged,
	Serrated,
	CliffBanded,
}

#[derive(Debug, Clone, Copy)]
pub struct RuggedMassifParams {
	pub style: MassifStyle,
	pub width_frac: f32,
	pub lift: f32,
	pub crest_scale: f32,
}

impl Default for RuggedMassifParams {
	fn default() -> Self {
		Self { style: MassifStyle::Ridged, width_frac: 0.14, lift: 22.0, crest_scale: 1.15 }
	}
}

#[derive(Debug, Clone)]
pub struct RuggedMassif {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: RuggedMassifParams,
	pub path: Vec<Vec2>,
	pub stamp: StampSet,
}

impl RuggedMassif {
	pub fn from_bounds(bounds: Bounds2, seed: u32, params: RuggedMassifParams) -> Self {
		let hash = SeededHash::new(seed);
		let short = bounds.extent().min_element().max(1.0);
		let (start, end) = FractalAnchors::default().sample(bounds, seed, 100);
		let path = HysteresisSpine::default().build(bounds, seed.wrapping_add(19), start, end);
		let half_width = short * params.width_frac.clamp(0.06, 0.3);
		let (inner, outer, width_mul) = match params.style {
			MassifStyle::Ridged => (0.4, 0.65, 1.4),
			MassifStyle::Serrated => (0.3, 0.55, 1.4),
			MassifStyle::CliffBanded => (0.35, 0.7, 1.2),
		};
		let noise = RegionNoise::from_seed(
			seed.wrapping_add(8),
			0.04 + 0.02 * hash.unit(3),
			half_width * 0.15,
		);
		let mut modulations = SoftmaskAlongSpine::default().build(
			&path,
			half_width * width_mul,
			params.crest_scale,
			params.lift,
			inner,
			outer,
			&noise,
			Vec2::ZERO,
		);
		// Secondary parallel crest for serrated / cliff-banded looks.
		if matches!(params.style, MassifStyle::Serrated | MassifStyle::CliffBanded) {
			let axis = (*path.last().unwrap_or(&end) - *path.first().unwrap_or(&start))
				.normalize_or_zero();
			let perp = Vec2::new(-axis.y, axis.x);
			let side = if hash.unit(12) > 0.5 { 1.0 } else { -1.0 };
			modulations.extend(SoftmaskAlongSpine::default().build(
				&path,
				half_width * 0.55,
				params.crest_scale * 0.9,
				params.lift * 0.7,
				inner,
				outer,
				&noise,
				perp * half_width * 0.6 * side,
			));
		}

		let mut semantics = StampSemantics::default()
			.with_tag("massif")
			.with_tag("exposure")
			.with_tag("rockiness");
		semantics = match params.style {
			MassifStyle::Ridged => semantics.with_tag("ridged"),
			MassifStyle::Serrated => semantics.with_tag("serrated"),
			MassifStyle::CliffBanded => semantics.with_tag("cliff_banded"),
		};

		Self {
			bounds,
			seed,
			params,
			path: path.clone(),
			stamp: StampSet { modulations, spine: path, semantics },
		}
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, RuggedMassifParams::default())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn massif_raises_crest() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let m = RuggedMassif::from_bounds_default(bounds, 3);
		let p = m.path[m.path.len() / 2];
		let h_crest = m.stamp.apply_elevation(40.0, p.x, p.y);
		let h_out = m.stamp.apply_elevation(40.0, bounds.min.x + 2.0, bounds.min.y + 2.0);
		assert!(h_crest > h_out);
		Ok(())
	}
}
