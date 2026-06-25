//! Suggested noise preset for unit-scale / playground noisy balls ([RFC-183 §3.1.2.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/02-noisy-ball/README.md)).

use procedural_common::{NoiseParams, NoiseType};

/// Zero-sized marker: [`From`] / [`Into`] yields a Perlin preset suited to typical unit spheres
/// (same numeric recipe as [`super::unit_cylinder::UnitCylinderNoiseParams`]: amplitude `0.05`, frequency `5.0`, single octave).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct UnitBallNoiseParams;

impl From<UnitBallNoiseParams> for NoiseParams {
	fn from(_: UnitBallNoiseParams) -> Self {
		Self {
			amplitude: 0.05,
			frequency: 5.0,
			octaves: 1,
			noise_type: NoiseType::Perlin,
			..Default::default()
		}
	}
}
