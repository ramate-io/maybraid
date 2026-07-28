//! Normalized partition geometry components (linear / 180° / 90° / 15° / header kit).

use bevy_math::Vec3;

use crate::partitions::geometry::{ArcWall, LinearWall, PolylineWall, Wall};
use crate::placed::{IntoGeometryComponents, Placed};

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

/// Normalized wall kit piece (matches partition README spaces).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WallComponent {
	Linear,
	LinearSubsegment,
	LinearHeaderSubsegment,
	Arc180,
	Arc90,
	Arc15,
	HeaderArc180,
	HeaderArc90,
	HeaderArc15,
}

impl From<ArcKit> for WallComponent {
	fn from(kit: ArcKit) -> Self {
		match kit {
			ArcKit::D180 => Self::Arc180,
			ArcKit::D90 => Self::Arc90,
			ArcKit::D15 => Self::Arc15,
		}
	}
}

fn header_component(kit: ArcKit) -> WallComponent {
	match kit {
		ArcKit::D180 => WallComponent::HeaderArc180,
		ArcKit::D90 => WallComponent::HeaderArc90,
		ArcKit::D15 => WallComponent::HeaderArc15,
	}
}

impl IntoGeometryComponents for Wall {
	type Component = WallComponent;

	fn into_geometry_components(&self) -> Vec<Placed<WallComponent>> {
		match self {
			Self::Linear(g) => g.into_geometry_components(),
			Self::Polyline(g) => g.into_geometry_components(),
			Self::Arc(g) => g.into_geometry_components(),
			Self::HeaderArc(g) => decompose_arc_sweep(g.sweep_degrees)
				.into_iter()
				.map(|(kit, yaw)| Placed::new(header_component(kit), Vec3::ZERO, yaw))
				.collect(),
		}
	}
}

impl IntoGeometryComponents for LinearWall {
	type Component = WallComponent;

	fn into_geometry_components(&self) -> Vec<Placed<WallComponent>> {
		vec![Placed::at_origin(WallComponent::Linear)]
	}
}

impl IntoGeometryComponents for PolylineWall {
	type Component = WallComponent;

	fn into_geometry_components(&self) -> Vec<Placed<WallComponent>> {
		let n = self.points.len().saturating_sub(1).max(1);
		(0..n)
			.map(|_| Placed::at_origin(WallComponent::Linear))
			.collect()
	}
}

impl IntoGeometryComponents for ArcWall {
	type Component = WallComponent;

	fn into_geometry_components(&self) -> Vec<Placed<WallComponent>> {
		decompose_arc_sweep(self.sweep_degrees)
			.into_iter()
			.map(|(kit, yaw)| Placed::new(WallComponent::from(kit), Vec3::ZERO, yaw))
			.collect()
	}
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
