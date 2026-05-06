//! Default swatch tables: **macro** (broad lithology) vs **micro** (surface / fracture detail).
//!
//! Each of the eight slots is **one regional type** only: narrow `left`→`right` ramps so a swatch
//! does not stand in for chalk *and* alluvium at once. **[`EVEN_SWATCH_FOLD_WEIGHT`]** matches the
//! shader’s sequential `mix` chain so every swatch uses the same fold-in weight for tuning.

use bevy::prelude::*;

use super::swatch::DurhamSwatchUniform;

fn c(r: f32, g: f32, b: f32) -> Vec3 {
	Vec3::new(r, g, b)
}

/// Same fold weight on every swatch so palette reads are comparable before custom selection logic.
pub const EVEN_SWATCH_FOLD_WEIGHT: f32 = 1.0 / 8.0;

/// Narrow ramp: `b` is a small tweak from `a` (noise still traverses the segment via `u`).
fn tight_pair(a: Vec3, b: Vec3) -> DurhamSwatchUniform {
	DurhamSwatchUniform::transition(a, b, EVEN_SWATCH_FOLD_WEIGHT)
}

/// Eight **macro** lithology buckets (linear RGB). Order is stable for shader selection experiments.
pub fn macro_region_palette() -> [DurhamSwatchUniform; 8] {
	[
		// 0 — Alluvial / floodplain fines (silty brown-grey)
		tight_pair(c(0.38, 0.30, 0.22), c(0.42, 0.33, 0.24)),
		// 1 — Granite (pink–grey feldspar)
		tight_pair(c(0.58, 0.48, 0.46), c(0.62, 0.52, 0.50)),
		// 2 — Sand (warm quartz)
		tight_pair(c(0.76, 0.66, 0.48), c(0.80, 0.69, 0.51)),
		// 3 — Chalk (soft white–cream)
		tight_pair(c(0.92, 0.91, 0.88), c(0.94, 0.93, 0.90)),
		// 4 — Limestone (grey-beige carbonate)
		tight_pair(c(0.78, 0.76, 0.72), c(0.82, 0.79, 0.74)),
		// 5 — Clay / lateritic soil (reddish brown)
		tight_pair(c(0.48, 0.30, 0.22), c(0.52, 0.33, 0.24)),
		// 6 — Shale / mudstone (blue-grey laminated)
		tight_pair(c(0.34, 0.36, 0.38), c(0.38, 0.39, 0.41)),
		// 7 — Basalt / greywacke (dark grey-green)
		tight_pair(c(0.22, 0.24, 0.23), c(0.26, 0.27, 0.26)),
	]
}

/// Eight **micro** buckets aligned with [`macro_region_palette`] indices: moisture, grain chip, rind.
pub fn micro_region_palette() -> [DurhamSwatchUniform; 8] {
	[
		// 0 — Wet alluvial
		tight_pair(c(0.32, 0.26, 0.19), c(0.36, 0.29, 0.21)),
		// 1 — Granite with slight olive weathering film
		tight_pair(c(0.52, 0.48, 0.44), c(0.55, 0.51, 0.47)),
		// 2 — Damp sand (cooler)
		tight_pair(c(0.68, 0.62, 0.46), c(0.72, 0.65, 0.49)),
		// 3 — Chalk dust / crush (still chalk-only)
		tight_pair(c(0.88, 0.87, 0.84), c(0.90, 0.89, 0.86)),
		// 4 — Limestone with faint sulphur tint
		tight_pair(c(0.74, 0.74, 0.68), c(0.78, 0.77, 0.71)),
		// 5 — Clay crust (slightly drier)
		tight_pair(c(0.44, 0.28, 0.20), c(0.47, 0.31, 0.22)),
		// 6 — Wet shale (darker)
		tight_pair(c(0.26, 0.30, 0.33), c(0.30, 0.33, 0.36)),
		// 7 — Fracture face on dark stone
		tight_pair(c(0.18, 0.19, 0.18), c(0.22, 0.23, 0.22)),
	]
}
