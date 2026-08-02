//! A pitched roof shell built from two [`RoofHalf`]s.
//!
//! Each half authors a ridge / eave / wall plate. The main pitch is a
//! [`RuledPitch`]; optional wall strip, half-gable walling, and half-hip
//! facets fill the ends. Guiding case: a rectangular hip where both halves
//! share one ridge and eaves run parallel at equal offset.

use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::paneling::panel_complex::{
	PanelComplex, PanelComplexJointPolicy, DEFAULT_PANEL_THICKNESS,
};
use crate::paneling::ruled_pitch::RuledPitch;
use crate::paneling::ruled_strip::RuledStrip;
use crate::paneling::tessellated_triangle_panel::TessellatedTrianglePanel;

/// One pitch of a [`PitchedRoof`]: ridge, eave, and wall-plate segment.
#[derive(Debug, Clone, PartialEq)]
pub struct RoofHalf {
	pub ridge_line: (Vec3, Vec3),
	pub eave_line: (Vec3, Vec3),
	pub wall_line: (Vec3, Vec3),
	/// Longitudinal strip between wall plate and eave.
	pub draw_in_wall_line: bool,
	/// End walling at line endpoints `.0` / `.1` (not exclusive with hip).
	pub draw_in_half_gable_end: (bool, bool),
	/// End hip facets at line endpoints `.0` / `.1`.
	pub draw_in_half_hip: (bool, bool),
}

impl RoofHalf {
	pub fn new(
		ridge_line: (Vec3, Vec3),
		eave_line: (Vec3, Vec3),
		wall_line: (Vec3, Vec3),
	) -> Self {
		Self {
			ridge_line,
			eave_line,
			wall_line,
			draw_in_wall_line: false,
			draw_in_half_gable_end: (false, false),
			draw_in_half_hip: (false, false),
		}
	}

	pub fn draw_in_wall_line(mut self, draw: bool) -> Self {
		self.draw_in_wall_line = draw;
		self
	}

	pub fn draw_in_half_gable_end(mut self, ends: (bool, bool)) -> Self {
		self.draw_in_half_gable_end = ends;
		self
	}

	pub fn draw_in_half_hip(mut self, ends: (bool, bool)) -> Self {
		self.draw_in_half_hip = ends;
		self
	}

	fn line_end(line: (Vec3, Vec3), end: usize) -> Vec3 {
		if end == 0 {
			line.0
		} else {
			line.1
		}
	}

	/// Drop ridge end straight down in cardinal Y to the eave height.
	fn hip_drop(ridge_end: Vec3, eave_end: Vec3) -> Vec3 {
		Vec3::new(ridge_end.x, eave_end.y, ridge_end.z)
	}

	fn ridge_at_wall_height(ridge_end: Vec3, wall_end: Vec3) -> Vec3 {
		Vec3::new(ridge_end.x, wall_end.y, ridge_end.z)
	}

	fn resolve(&self, style: PanelStyle, joint_policy: PanelComplexJointPolicy) -> ResolvedRoofHalf {
		let (e0, e1) = self.eave_line;
		let (r0, r1) = self.ridge_line;
		let (w0, w1) = self.wall_line;

		let pitch = RuledPitch::from_lines(style, [e0, e1], [r0, r1])
			.with_joint_policy(joint_policy)
			.into_complex();

		let wall = if self.draw_in_wall_line {
			Some(
				RuledStrip::from_lines(style, [w0, w1], [e0, e1])
					.with_joint_policy(joint_policy)
					.into_complex(),
			)
		} else {
			None
		};

		let mut hips = Vec::new();
		for (end, draw) in [(0usize, self.draw_in_half_hip.0), (1, self.draw_in_half_hip.1)] {
			if !draw {
				continue;
			}
			let e = Self::line_end(self.eave_line, end);
			let r = Self::line_end(self.ridge_line, end);
			let p = Self::hip_drop(r, e);
			hips.push(TessellatedTrianglePanel::new(style, e, r, p));
		}

		let mut gables = Vec::new();
		for (end, draw) in [
			(0usize, self.draw_in_half_gable_end.0),
			(1, self.draw_in_half_gable_end.1),
		] {
			if !draw {
				continue;
			}
			let w = Self::line_end(self.wall_line, end);
			let e = Self::line_end(self.eave_line, end);
			let r = Self::line_end(self.ridge_line, end);
			let r_wall = Self::ridge_at_wall_height(r, w);
			// Overhang barge + vertical gable face.
			gables.push(TessellatedTrianglePanel::new(style, w, e, r));
			gables.push(TessellatedTrianglePanel::new(style, w, r, r_wall));
		}

		ResolvedRoofHalf {
			pitch,
			wall,
			hips,
			gables,
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
struct ResolvedRoofHalf {
	pitch: PanelComplex,
	wall: Option<PanelComplex>,
	hips: Vec<TessellatedTrianglePanel>,
	gables: Vec<TessellatedTrianglePanel>,
}

/// Authored parameters for a [`PitchedRoof`].
#[derive(Debug, Clone, PartialEq)]
pub struct PitchedRoofParams {
	pub halves: [RoofHalf; 2],
	pub style: PanelStyle,
	pub joint_thickness: f32,
}

impl Default for PitchedRoofParams {
	fn default() -> Self {
		Self::rectangular_hip(Vec2::new(10.0, 6.0), 4.0, 2.5, 1.5)
	}
}

impl PitchedRoofParams {
	pub fn new(halves: [RoofHalf; 2]) -> Self {
		Self {
			halves,
			style: PanelStyle::ShepherdsThatch,
			joint_thickness: DEFAULT_PANEL_THICKNESS,
		}
	}

	/// Axis-aligned rectangular hip about the origin.
	///
	/// `footprint.x` is length along the ridge/eaves (X); `footprint.y` is the
	/// full span between eaves (Z). Ridge is centered on X at `z = 0`, shortened
	/// by `ridge_inset` on each end. Wall plates sit `overhang` inward from each
	/// eave (default overhang 0.3). Both halves draw hips on both ends.
	pub fn rectangular_hip(
		footprint: Vec2,
		ridge_height: f32,
		eave_height: f32,
		ridge_inset: f32,
	) -> Self {
		const OVERHANG: f32 = 0.3;
		let half_x = footprint.x * 0.5;
		let half_z = footprint.y * 0.5;
		let ridge_half = (half_x - ridge_inset).max(0.0);
		let wall_z = (half_z - OVERHANG).max(0.0);

		let ridge = (
			Vec3::new(-ridge_half, ridge_height, 0.0),
			Vec3::new(ridge_half, ridge_height, 0.0),
		);
		let eave_pos = (
			Vec3::new(-half_x, eave_height, half_z),
			Vec3::new(half_x, eave_height, half_z),
		);
		let eave_neg = (
			Vec3::new(-half_x, eave_height, -half_z),
			Vec3::new(half_x, eave_height, -half_z),
		);
		let wall_pos = (
			Vec3::new(-half_x, eave_height, wall_z),
			Vec3::new(half_x, eave_height, wall_z),
		);
		let wall_neg = (
			Vec3::new(-half_x, eave_height, -wall_z),
			Vec3::new(half_x, eave_height, -wall_z),
		);

		let pos = RoofHalf::new(ridge, eave_pos, wall_pos)
			.draw_in_wall_line(true)
			.draw_in_half_hip((true, true));
		let neg = RoofHalf::new(ridge, eave_neg, wall_neg)
			.draw_in_wall_line(true)
			.draw_in_half_hip((true, true));

		Self::new([pos, neg])
	}

	pub fn style(mut self, style: PanelStyle) -> Self {
		self.style = style;
		self
	}

	pub fn joint_thickness(mut self, joint_thickness: f32) -> Self {
		self.joint_thickness = joint_thickness;
		self
	}

	pub fn build(self) -> PitchedRoof {
		PitchedRoof::new(self)
	}
}

/// Two-half pitched roof shell.
#[derive(Debug, Clone, PartialEq)]
pub struct PitchedRoof {
	params: PitchedRoofParams,
	joint_policy: PanelComplexJointPolicy,
	halves: [ResolvedRoofHalf; 2],
}

impl PitchedRoof {
	pub fn new(params: PitchedRoofParams) -> Self {
		let joint_policy = PanelComplexJointPolicy::default();
		let style = params.style;
		let halves = [
			params.halves[0].resolve(style, joint_policy),
			params.halves[1].resolve(style, joint_policy),
		];
		Self {
			params,
			joint_policy,
			halves,
		}
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		self.halves = [
			self.params.halves[0].resolve(self.params.style, joint_policy),
			self.params.halves[1].resolve(self.params.style, joint_policy),
		];
		self
	}

	pub fn params(&self) -> &PitchedRoofParams {
		&self.params
	}

	pub fn pitch_complexes(&self) -> [&PanelComplex; 2] {
		[&self.halves[0].pitch, &self.halves[1].pitch]
	}

	pub fn wall_complexes(&self) -> [Option<&PanelComplex>; 2] {
		[self.halves[0].wall.as_ref(), self.halves[1].wall.as_ref()]
	}

	pub fn hip_panels(&self) -> impl Iterator<Item = &TessellatedTrianglePanel> {
		self.halves.iter().flat_map(|h| h.hips.iter())
	}

	pub fn gable_panels(&self) -> impl Iterator<Item = &TessellatedTrianglePanel> {
		self.halves.iter().flat_map(|h| h.gables.iter())
	}

	pub fn hip_count(&self) -> usize {
		self.halves.iter().map(|h| h.hips.len()).sum()
	}

	pub fn gable_count(&self) -> usize {
		self.halves.iter().map(|h| h.gables.len()).sum()
	}
}

impl BuildingComponents for PitchedRoof {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for half in &self.halves {
			out.extend(half.pitch.panel_nodes_for_level(level));
			if let Some(wall) = &half.wall {
				out.extend(wall.panel_nodes_for_level(level));
			}
			for hip in &half.hips {
				out.extend(hip.panel_nodes_for_level(level));
			}
			for gable in &half.gables {
				out.extend(gable.panel_nodes_for_level(level));
			}
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for half in &self.halves {
			out.extend(half.pitch.joint_nodes_for_level(level));
			if let Some(wall) = &half.wall {
				out.extend(wall.joint_nodes_for_level(level));
			}
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use lod::gen::LodSceneLevel;
	use richmond_building_components::BuildingComponents;

	fn assert_vec3_close(got: Vec3, want: Vec3) {
		assert!(
			(got - want).length() < 1e-4,
			"got {got:?} want {want:?}"
		);
	}

	#[test]
	fn rectangular_hip_shared_ridge_and_four_hips() {
		let params = PitchedRoofParams::rectangular_hip(Vec2::new(10.0, 6.0), 4.0, 2.5, 1.5);
		let roof = PitchedRoof::new(params);

		assert_eq!(roof.params().halves[0].ridge_line, roof.params().halves[1].ridge_line);
		assert_eq!(roof.hip_count(), 4);
		assert_eq!(roof.gable_count(), 0);
		assert!(roof.wall_complexes()[0].is_some());
		assert!(roof.wall_complexes()[1].is_some());

		for pitch in roof.pitch_complexes() {
			// One bay → two triangles.
			assert_eq!(pitch.triangles().len(), 2);
		}

		let eave_pos_z = roof.params().halves[0].eave_line.0.z;
		let eave_neg_z = roof.params().halves[1].eave_line.0.z;
		let mid_z = 0.5 * (eave_pos_z + eave_neg_z);
		assert!((mid_z).abs() < 1e-5);

		for hip in roof.hip_panels() {
			// Third corner is the vertical drop of the ridge end to eave height.
			let p = hip.c;
			assert!((p.y - 2.5).abs() < 1e-4);
			assert!((p.z - mid_z).abs() < 1e-4);
			assert_vec3_close(p, Vec3::new(hip.b.x, 2.5, mid_z));
		}

		let panels = roof
			.panel_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.len();
		// 2 pitches × 2 tris + 2 walls × 2 tris + 4 hips = 4 + 4 + 4 = 12
		assert_eq!(panels, 12);
	}

	#[test]
	fn gable_only_emits_end_walling() {
		let footprint = Vec2::new(8.0, 5.0);
		let half_x = footprint.x * 0.5;
		let half_z = footprint.y * 0.5;
		let ridge = (
			Vec3::new(-half_x, 4.0, 0.0),
			Vec3::new(half_x, 4.0, 0.0),
		);
		let eave = (
			Vec3::new(-half_x, 2.0, half_z),
			Vec3::new(half_x, 2.0, half_z),
		);
		let wall = (
			Vec3::new(-half_x, 2.0, half_z - 0.2),
			Vec3::new(half_x, 2.0, half_z - 0.2),
		);
		// Mirror half omitted for a single-pitch gable check: still build two
		// halves so the shell API is exercised; second half has no end fill.
		let pos = RoofHalf::new(ridge, eave, wall)
			.draw_in_wall_line(true)
			.draw_in_half_gable_end((true, true));
		let neg_eave = (
			Vec3::new(-half_x, 2.0, -half_z),
			Vec3::new(half_x, 2.0, -half_z),
		);
		let neg_wall = (
			Vec3::new(-half_x, 2.0, -(half_z - 0.2)),
			Vec3::new(half_x, 2.0, -(half_z - 0.2)),
		);
		let neg = RoofHalf::new(ridge, neg_eave, neg_wall);
		let roof = PitchedRoofParams::new([pos, neg]).build();

		assert_eq!(roof.hip_count(), 0);
		// Two ends × two tris each on the +Z half only.
		assert_eq!(roof.gable_count(), 4);
		assert!(roof.wall_complexes()[0].is_some());
		assert!(roof.wall_complexes()[1].is_none());
	}

	#[test]
	fn gable_and_hip_coexist_on_same_end() {
		let ridge = (Vec3::new(-2.0, 4.0, 0.0), Vec3::new(2.0, 4.0, 0.0));
		let eave = (Vec3::new(-4.0, 2.0, 3.0), Vec3::new(4.0, 2.0, 3.0));
		let wall = (Vec3::new(-4.0, 2.0, 2.7), Vec3::new(4.0, 2.0, 2.7));
		let half = RoofHalf::new(ridge, eave, wall)
			.draw_in_half_hip((true, false))
			.draw_in_half_gable_end((true, false));
		let other = RoofHalf::new(
			ridge,
			(Vec3::new(-4.0, 2.0, -3.0), Vec3::new(4.0, 2.0, -3.0)),
			(Vec3::new(-4.0, 2.0, -2.7), Vec3::new(4.0, 2.0, -2.7)),
		);
		let roof = PitchedRoofParams::new([half, other]).build();
		assert_eq!(roof.hip_count(), 1);
		assert_eq!(roof.gable_count(), 2);
	}
}
