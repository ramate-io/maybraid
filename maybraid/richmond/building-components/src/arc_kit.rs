//! Shared angular kit sizes and continuous-arc decomposition.

/// Normalized angular kit sizes from the partition README.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArcKit {
	D180,
	D90,
	D15,
}

impl ArcKit {
	pub fn degrees(self) -> f32 {
		match self {
			Self::D180 => 180.0,
			Self::D90 => 90.0,
			Self::D15 => 15.0,
		}
	}
}

/// Greedy decomposition of a continuous arc sweep into kit pieces.
///
/// Returns `(kit, yaw_offset_radians)`. Example: 45° → three [`ArcKit::D15`].
pub fn decompose_arc_sweep(sweep_degrees: f32) -> Vec<(ArcKit, f32)> {
	let sign = if sweep_degrees < 0.0 { -1.0 } else { 1.0 };
	let mut remaining = sweep_degrees.abs();
	let mut cursor_deg = 0.0_f32;
	let mut out = Vec::new();
	const EPS: f32 = 1e-3;

	while remaining > EPS {
		let kit = if remaining + EPS >= 180.0 {
			ArcKit::D180
		} else if remaining + EPS >= 90.0 {
			ArcKit::D90
		} else {
			ArcKit::D15
		};
		let piece = kit.degrees();
		out.push((kit, f32::to_radians(sign * cursor_deg)));
		cursor_deg += piece;
		remaining -= piece;
	}

	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn forty_five_degrees_is_three_fifteens() -> anyhow::Result<()> {
		let pieces = decompose_arc_sweep(45.0);
		assert_eq!(pieces.len(), 3);
		assert!(pieces.iter().all(|(k, _)| *k == ArcKit::D15));
		Ok(())
	}

	#[test]
	fn one_eighty_is_single_piece() -> anyhow::Result<()> {
		let pieces = decompose_arc_sweep(180.0);
		assert_eq!(pieces, vec![(ArcKit::D180, 0.0)]);
		Ok(())
	}
}
