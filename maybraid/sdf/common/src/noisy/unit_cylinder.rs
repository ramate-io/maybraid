//! Suggested noise preset for unit-scale / playground noisy cylinders.

use procedural_common::{NoiseParams, NoiseType};

/// Zero-sized marker: [`From`] / [`Into`] yields a Perlin preset suited to typical unit cylinders
/// (matches historical `NoisySurface::new_perlin`-style defaults: amplitude `0.05`, frequency `5.0`, single octave).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct UnitCylinderNoiseParams;

impl From<UnitCylinderNoiseParams> for NoiseParams {
	fn from(_: UnitCylinderNoiseParams) -> Self {
		Self {
			amplitude: 0.05,
			frequency: 5.0,
			octaves: 1,
			noise_type: NoiseType::Perlin,
			..Default::default()
		}
	}
}
