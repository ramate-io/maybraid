//! Ruled pitch strip: equal-length eave / ridge polylines with implicit index rafters.
//!
//! Station \(i\) connects `eave[i]` ↔ `ridge[i]`. Bay \(i\) is the quad
//! `{eave[i], eave[i+1], ridge[i], ridge[i+1]}` (diagonal \(a_0\)–\(b_1\)).
//!
//! Geometry lives in an owned [`PanelComplex`]; [`Self::stations`] keeps eave/ridge
//! point ids so the polylines remain queryable. Shared-ridge multi-pitch roofs and
//! triangular hips are separate constructions.

use richmond_building_components::panels::PanelStyle;

use crate::panel_complex::{
	PanelComplex, PanelComplexJointPolicy, PanelPoint, PanelPointId,
};

/// Equal-station eave/ridge pitch backed by a [`PanelComplex`].
#[derive(Debug, Clone, PartialEq)]
pub struct RuledPitch {
	complex: PanelComplex,
	/// Parallel stations `(eave_id, ridge_id)`.
	stations: Vec<(PanelPointId, PanelPointId)>,
}

impl RuledPitch {
	/// Empty pitch (no stations). Style + default joint policy on the complex.
	pub fn new(style: PanelStyle) -> Self {
		Self {
			complex: PanelComplex::new(style),
			stations: Vec::new(),
		}
	}

	pub fn rough_stone() -> Self {
		Self::new(PanelStyle::RoughStonework)
	}

	pub fn shepherds_thatch() -> Self {
		Self::new(PanelStyle::ShepherdsThatch)
	}

	/// Bulk construct: require `eave.len() == ridge.len() >= 2`.
	///
	/// On mismatch or too-short inputs: `debug_assert` and return an empty pitch
	/// with the given style (infallible presentation path).
	pub fn from_lines(
		style: PanelStyle,
		eave: impl IntoIterator<Item = impl Into<PanelPoint>>,
		ridge: impl IntoIterator<Item = impl Into<PanelPoint>>,
	) -> Self {
		let eave: Vec<PanelPoint> = eave.into_iter().map(Into::into).collect();
		let ridge: Vec<PanelPoint> = ridge.into_iter().map(Into::into).collect();
		if eave.len() != ridge.len() || eave.len() < 2 {
			debug_assert!(
				false,
				"RuledPitch::from_lines requires equal lengths >= 2 (got eave={}, ridge={})",
				eave.len(),
				ridge.len()
			);
			return Self::new(style);
		}
		let mut pitch = Self::new(style);
		for (e, r) in eave.into_iter().zip(ridge) {
			pitch.add_pair(e, r);
		}
		pitch
	}

	/// Sugar: rebuild from lines keeping current style.
	pub fn with_lines(
		self,
		eave: impl IntoIterator<Item = impl Into<PanelPoint>>,
		ridge: impl IntoIterator<Item = impl Into<PanelPoint>>,
	) -> Self {
		let style = self.complex.style;
		let policy = self.complex.joint_policy;
		Self::from_lines(style, eave, ridge).with_joint_policy(policy)
	}

	/// Append one eave/ridge station. When a previous station exists, adds the bay quad.
	pub fn add_pair(
		&mut self,
		eave: impl Into<PanelPoint>,
		ridge: impl Into<PanelPoint>,
	) -> &mut Self {
		let eave = eave.into();
		let ridge = ridge.into();
		let e_id = self.complex.insert_point_thick(eave.position, eave.thickness);
		let r_id = self.complex.insert_point_thick(ridge.position, ridge.thickness);
		if let Some(&(prev_e, prev_r)) = self.stations.last() {
			// Bay: {eave[i], eave[i+1], ridge[i], ridge[i+1]}
			self.complex.add_quad(prev_e, e_id, prev_r, r_id);
		}
		self.stations.push((e_id, r_id));
		self
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.complex = self.complex.with_joint_policy(joint_policy);
		self
	}

	pub fn set_joint_policy(&mut self, joint_policy: PanelComplexJointPolicy) -> &mut Self {
		self.complex.set_joint_policy(joint_policy);
		self
	}

	pub fn stations(&self) -> &[(PanelPointId, PanelPointId)] {
		&self.stations
	}

	pub fn eave_ids(&self) -> impl Iterator<Item = PanelPointId> + '_ {
		self.stations.iter().map(|(e, _)| *e)
	}

	pub fn ridge_ids(&self) -> impl Iterator<Item = PanelPointId> + '_ {
		self.stations.iter().map(|(_, r)| *r)
	}

	pub fn eave_polyline(&self) -> Vec<PanelPoint> {
		self.eave_ids()
			.filter_map(|id| self.complex.point(id).copied())
			.collect()
	}

	pub fn ridge_polyline(&self) -> Vec<PanelPoint> {
		self.ridge_ids()
			.filter_map(|id| self.complex.point(id).copied())
			.collect()
	}

	pub fn as_complex(&self) -> &PanelComplex {
		&self.complex
	}

	pub fn into_complex(self) -> PanelComplex {
		self.complex
	}

	pub fn into_parts(self) -> (PanelComplex, Vec<(PanelPointId, PanelPointId)>) {
		(self.complex, self.stations)
	}
}

impl AsRef<PanelComplex> for RuledPitch {
	fn as_ref(&self) -> &PanelComplex {
		&self.complex
	}
}

impl From<RuledPitch> for PanelComplex {
	fn from(value: RuledPitch) -> Self {
		value.into_complex()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use lod::gen::LodSceneLevel;
	use richmond_building_components::BuildingComponents;

	fn example_lines() -> (Vec<Vec3>, Vec<Vec3>) {
		let eave = vec![
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(0.0, 0.0, 2.0),
			Vec3::new(0.0, 0.0, 4.0),
		];
		let ridge = vec![
			Vec3::new(1.0, 1.0, 1.0),
			Vec3::new(1.0, 1.0, 2.0),
			Vec3::new(1.0, 1.0, 4.0),
		];
		(eave, ridge)
	}

	#[test]
	fn from_lines_builds_two_quads_and_retrieves_polylines() {
		let (eave, ridge) = example_lines();
		let pitch = RuledPitch::from_lines(PanelStyle::ShepherdsThatch, eave.clone(), ridge.clone());
		assert_eq!(pitch.stations().len(), 3);
		assert_eq!(pitch.as_complex().triangles().len(), 4); // 2 quads × 2 tris
		let got_e = pitch.eave_polyline();
		let got_r = pitch.ridge_polyline();
		assert_eq!(got_e.len(), 3);
		assert_eq!(got_r.len(), 3);
		for (a, b) in got_e.iter().zip(eave) {
			assert!((a.position - b).length() < 1e-5);
		}
		for (a, b) in got_r.iter().zip(ridge) {
			assert!((a.position - b).length() < 1e-5);
		}
	}

	#[test]
	fn add_pair_matches_from_lines_topology() {
		let (eave, ridge) = example_lines();
		let mut via_pairs = RuledPitch::shepherds_thatch();
		for (e, r) in eave.iter().copied().zip(ridge.iter().copied()) {
			via_pairs.add_pair(e, r);
		}
		let via_lines = RuledPitch::from_lines(PanelStyle::ShepherdsThatch, eave, ridge);
		assert_eq!(via_pairs.stations().len(), via_lines.stations().len());
		assert_eq!(
			via_pairs.as_complex().triangles().len(),
			via_lines.as_complex().triangles().len()
		);
		assert_eq!(
			via_pairs.as_complex().shared_edges().len(),
			via_lines.as_complex().shared_edges().len()
		);
	}

	#[test]
	fn interior_rafters_are_shared_edges() {
		let (eave, ridge) = example_lines();
		let pitch = RuledPitch::from_lines(PanelStyle::RoughStonework, eave, ridge);
		// Stations 1 and 2 (middle + last? interior rafters between bays):
		// bay0 shares with bay1 the rafter eave[1]-ridge[1]
		// Also each bay has a diagonal shared by its two tris.
		// 2 bays → 2 diagonals + 1 interior rafter = 3 shared edges.
		assert_eq!(pitch.as_complex().shared_edges().len(), 3);
		let mid = pitch.stations()[1];
		let shared = pitch.as_complex().shared_edges();
		assert!(
			shared.iter().any(|e| {
				let (a, b) = e.endpoints();
				(a, b) == mid || (b, a) == mid
			}),
			"expected shared rafter at station 1 {mid:?}, got {shared:?}"
		);
	}

	#[test]
	fn never_policy_suppresses_joints() {
		let (eave, ridge) = example_lines();
		let c = RuledPitch::from_lines(PanelStyle::ShepherdsThatch, eave, ridge)
			.with_joint_policy(PanelComplexJointPolicy::never())
			.into_complex();
		assert!(c.joint_nodes().is_empty());
	}

	#[test]
	#[cfg(not(debug_assertions))]
	fn mismatched_from_lines_yields_empty() {
		let pitch = RuledPitch::from_lines(
			PanelStyle::RoughStonework,
			[Vec3::ZERO, Vec3::X],
			[Vec3::Y],
		);
		assert!(pitch.stations().is_empty());
		assert!(pitch.as_complex().triangles().is_empty());
	}

	#[test]
	#[cfg(debug_assertions)]
	#[should_panic(expected = "RuledPitch::from_lines requires equal lengths >= 2")]
	fn mismatched_from_lines_debug_asserts() {
		let _ = RuledPitch::from_lines(
			PanelStyle::RoughStonework,
			[Vec3::ZERO, Vec3::X],
			[Vec3::Y],
		);
	}

	#[test]
	fn into_complex_escape_hatch() {
		let (eave, ridge) = example_lines();
		let mut c = RuledPitch::from_lines(PanelStyle::RoughStonework, eave, ridge).into_complex();
		let a = c.insert_point(Vec3::new(2.0, 0.0, 0.0));
		let b = c.insert_point(Vec3::new(2.0, 1.0, 0.0));
		let d = c.insert_point(Vec3::new(2.0, 0.0, 1.0));
		c.add_triangle(a, b, d);
		assert_eq!(
			c.panel_nodes_for_level(LodSceneLevel::High).flatten().len(),
			5 // 4 from pitch + 1 extra
		);
	}

	#[test]
	fn into_parts_keeps_stations() {
		let (eave, ridge) = example_lines();
		let pitch = RuledPitch::from_lines(PanelStyle::RoughStonework, eave, ridge);
		let (complex, stations) = pitch.into_parts();
		assert_eq!(stations.len(), 3);
		assert_eq!(complex.triangles().len(), 4);
	}
}
