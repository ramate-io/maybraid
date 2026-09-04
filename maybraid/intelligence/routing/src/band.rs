/// One planning resolution. Coarse bands come first; finer bands refine along them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoutingBand {
	/// Spacing of committed waypoints at this resolution.
	pub segment: f32,
	/// How often the chord is sampled for ground and drop.
	pub probe_step: f32,
	/// Lateral offsets on each side of the corridor tangent (straight is always included).
	pub laterals: u32,
	/// Maximum sideways offset, in meters, when sampling around the parent corridor.
	pub lateral_span: f32,
}

impl RoutingBand {
	/// Derive probe and lateral defaults from a segment length.
	pub fn new(segment: f32) -> Self {
		let segment = segment.max(1.0);
		Self {
			segment,
			probe_step: (segment / 4.0).max(2.0),
			laterals: 4,
			lateral_span: segment * 0.55,
		}
	}

	pub fn with_probe_step(mut self, probe_step: f32) -> Self {
		self.probe_step = probe_step.max(0.5);
		self
	}

	pub fn with_laterals(mut self, laterals: u32, lateral_span: f32) -> Self {
		self.laterals = laterals;
		self.lateral_span = lateral_span.max(0.0);
		self
	}
}

/// Policy for hierarchical long-range routing. Band lengths are not a crate constant;
/// they belong to a particular mover / application.
#[derive(Clone, Debug, PartialEq)]
pub struct RoutingSettings {
	/// Coarse-to-fine. Empty means a single straight hop to the destination.
	pub bands: Vec<RoutingBand>,
	pub max_fall: f32,
	pub hip_height: f32,
	pub feet_below_origin: f32,
	pub agent_radius: f32,
	/// Arrival disk for the hop handed to movement intelligence.
	pub arrival_radius: f32,
	pub blocked_cost: f32,
	pub cliff_cost: f32,
	pub failed_cost: f32,
	pub weight_drop: f32,
	/// Pull samples toward the previous plan (feedforward continuity).
	pub continuity: f32,
}

impl RoutingSettings {
	pub fn from_segments(segments: impl IntoIterator<Item = f32>) -> Self {
		Self {
			bands: segments
				.into_iter()
				.filter(|segment| *segment > 0.0)
				.map(RoutingBand::new)
				.collect(),
			..Self::policy_defaults()
		}
	}

	fn policy_defaults() -> Self {
		Self {
			bands: Vec::new(),
			max_fall: 1.2,
			hip_height: 0.55,
			feet_below_origin: 0.9,
			agent_radius: 0.4,
			arrival_radius: 2.4,
			blocked_cost: 400.0,
			cliff_cost: 400.0,
			failed_cost: 250.0,
			weight_drop: 8.0,
			continuity: 0.35,
		}
	}
}

impl Default for RoutingSettings {
	fn default() -> Self {
		Self::from_segments([1000.0, 500.0, 100.0])
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_bands_are_an_example_not_a_fixed_ladder() -> anyhow::Result<()> {
		let long = RoutingSettings::default();
		assert_eq!(
			long.bands.iter().map(|band| band.segment).collect::<Vec<_>>(),
			vec![1000.0, 500.0, 100.0]
		);
		let short = RoutingSettings::from_segments([80.0, 20.0]);
		assert_eq!(short.bands.len(), 2);
		assert!((short.bands[0].probe_step - 20.0).abs() < 1e-4);
		assert_ne!(short.bands[0].segment, long.bands[0].segment);
		Ok(())
	}
}
