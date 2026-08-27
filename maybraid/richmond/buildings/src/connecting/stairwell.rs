//! Exclusive [`WellAabb`] stairwell: run-in + circular or rectangular flight +
//! exit landing.
//!
//! Two horizontal shaft faces allocate one orthogonal box. Offset / skew / size
//! mismatch is another well (or a hall), not a polyline. Walk-off is a landing.
//! The last tread arrives at that landing's interior edge (the back-point).
//! Extra laps exist only to keep going above a floor when headroom still holds.

mod opening;
mod rect;
mod spiral;
mod tread;
mod well;

pub use opening::StairwellOpening;
pub use tread::TreadEnd;
pub use well::{WellAabb, WellSide};

use lod::gen::LodSceneLevel;
use richmond_building_components::floors::FloorNode;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::stairs::StairNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::quad_panel::QuadPanel;

/// Aesthetic run-in depth along the walk-on (meters).
pub const RUN_IN_M: f32 = 0.75;

/// Kit thickness for owned floor slabs (meters).
pub const SLAB_THICKNESS_M: f32 = 0.05;

/// Default tread span as a fraction of the well's tighter half-extent.
pub const TREAD_FILL_DEFAULT: f32 = well::TREAD_FILL_DEFAULT;

/// Circular helix or wall-hugging rectangular flights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StairwellKind {
	#[default]
	Circular,
	Rectangular,
}

/// Exclusive box → run-in + flight + optional walk-off landing.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectingStairwell {
	style: PanelStyle,
	well: WellAabb,
	kind: StairwellKind,
	run_in: QuadPanel,
	want_landing: bool,
	slab_thickness: f32,
	upper_landing: Option<QuadPanel>,
	mid_landings: Vec<QuadPanel>,
	stairs: Vec<StairNode>,
}

impl ConnectingStairwell {
	/// Allocate an exclusive box from two shaft faces. `lower` is the floor-space
	/// anchor even when both faces share a \(Y\).
	pub fn new(
		style: PanelStyle,
		lower: impl Into<StairwellOpening>,
		upper: impl Into<StairwellOpening>,
	) -> Self {
		Self::from_well(style, WellAabb::allocate(lower.into(), upper.into(), TREAD_FILL_DEFAULT))
	}

	/// Fit a circular spiral into an already-allocated exclusive box.
	pub fn from_well(style: PanelStyle, well: WellAabb) -> Self {
		Self::from_well_kind(style, well, StairwellKind::Circular)
	}

	/// Fit [`StairwellKind`] into an already-allocated exclusive box.
	pub fn from_well_kind(style: PanelStyle, well: WellAabb, kind: StairwellKind) -> Self {
		let slab_thickness = SLAB_THICKNESS_M;
		let (stairs, landing, mids) = fit_kind(&well, style, slab_thickness, kind);
		Self {
			style,
			kind,
			run_in: well.run_in_slab(style, slab_thickness),
			want_landing: true,
			slab_thickness,
			upper_landing: landing,
			mid_landings: mids,
			stairs,
			well,
		}
	}

	pub fn rough_stone(
		lower: impl Into<StairwellOpening>,
		upper: impl Into<StairwellOpening>,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, lower, upper)
	}

	/// Walk-off landing. Default `true`. Set `false` when a follow-on well owns
	/// that floor (its run-in).
	pub fn with_upper_landing(mut self, enabled: bool) -> Self {
		self.want_landing = enabled;
		self.rebuild();
		self
	}

	/// Kit thickness of owned slabs (meters). Default [`SLAB_THICKNESS_M`].
	pub fn with_slab_thickness(mut self, thickness: f32) -> Self {
		self.slab_thickness = thickness.max(1e-4);
		self.rebuild();
		self
	}

	/// Tread span as a fraction of the tighter well half-extent.
	///
	/// Default [`TREAD_FILL_DEFAULT`]. Clamped to \(0.2\ldots 0.95\).
	pub fn with_tread_fill(mut self, fill: f32) -> Self {
		self.well.tread_fill = well::clamp_tread_fill(fill);
		self.rebuild();
		self
	}

	/// Circular helix or rectangular wall-hug. Default [`StairwellKind::Circular`].
	pub fn with_kind(mut self, kind: StairwellKind) -> Self {
		self.kind = kind;
		self.rebuild();
		self
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.run_in = self.run_in.clone().with_joint_policy(joint_policy);
		self.upper_landing =
			self.upper_landing.take().map(|slab| slab.with_joint_policy(joint_policy));
		self.mid_landings = self
			.mid_landings
			.drain(..)
			.map(|slab| slab.with_joint_policy(joint_policy))
			.collect();
		self
	}

	fn rebuild(&mut self) {
		let (stairs, landing, mids) =
			fit_kind(&self.well, self.style, self.slab_thickness, self.kind);
		self.stairs = stairs;
		self.run_in = self.well.run_in_slab(self.style, self.slab_thickness);
		self.upper_landing = self.want_landing.then_some(landing).flatten();
		self.mid_landings = mids;
	}

	pub fn well(&self) -> WellAabb {
		self.well
	}

	pub fn kind(&self) -> StairwellKind {
		self.kind
	}

	pub fn slab_thickness(&self) -> f32 {
		self.slab_thickness
	}

	pub fn tread_fill(&self) -> f32 {
		self.well.tread_fill
	}

	pub fn run_in(&self) -> &QuadPanel {
		&self.run_in
	}

	pub fn upper_landing(&self) -> Option<&QuadPanel> {
		self.upper_landing.as_ref()
	}

	pub fn mid_landings(&self) -> &[QuadPanel] {
		&self.mid_landings
	}

	pub fn stairs(&self) -> &[StairNode] {
		&self.stairs
	}

	pub fn last_tread_end(&self) -> Option<TreadEnd> {
		TreadEnd::from_last_straight(&self.stairs)
	}
}

impl BuildingComponents for ConnectingStairwell {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.run_in.panel_nodes_for_level(level);
		for pad in &self.mid_landings {
			out.extend(pad.panel_nodes_for_level(level));
		}
		if let Some(landing) = &self.upper_landing {
			out.extend(landing.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.run_in.joint_nodes_for_level(level);
		for pad in &self.mid_landings {
			out.extend(pad.joint_nodes_for_level(level));
		}
		if let Some(landing) = &self.upper_landing {
			out.extend(landing.joint_nodes_for_level(level));
		}
		out
	}

	fn floor_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FloorNode> {
		Layers::new()
	}

	fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
		Layers::from_free(self.stairs.clone())
	}
}

fn fit_kind(
	well: &WellAabb,
	style: PanelStyle,
	thickness: f32,
	kind: StairwellKind,
) -> (Vec<StairNode>, Option<QuadPanel>, Vec<QuadPanel>) {
	match kind {
		StairwellKind::Circular => {
			let (stairs, landing) = spiral::fit(well, style, thickness);
			(stairs, landing, Vec::new())
		}
		StairwellKind::Rectangular => rect::fit(well, style, thickness),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::MappedOpening;
	use bevy_math::{Vec2, Vec3};
	use richmond_building_components::partitions::PANEL_Y_HALF;
	use richmond_building_components::stairs::Stair;

	fn normalize_xz(v: Vec2) -> Option<Vec2> {
		let n = v.length();
		(n > 1e-5).then(|| v / n)
	}

	/// Horizontal shaft face: `center` in the hole, walk-on on the −orientation side.
	fn shaft_opening(
		center: Vec3,
		half_w: f32,
		half_d: f32,
		orient: Vec2,
	) -> anyhow::Result<MappedOpening> {
		let d = normalize_xz(orient)
			.ok_or_else(|| anyhow::anyhow!("orientation too short: {orient:?}"))?;
		let right = Vec3::new(-d.y, 0.0, d.x);
		let out = Vec3::new(d.x, 0.0, d.y);
		let walk = center - out * half_d;
		let far = center + out * half_d;
		let bl = walk - right * half_w;
		let br = walk + right * half_w;
		let tl = far - right * half_w;
		let tr = far + right * half_w;
		Ok(MappedOpening::from_corners(bl, br, tl, tr, orient))
	}

	#[test]
	fn stacked_shafts_allocate_an_exclusive_box() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		let aabb = well.well();
		assert!((aabb.rise() - 3.0).abs() < 1e-3);
		assert_eq!(aabb.walk_on, WellSide::NegZ);
		assert_eq!(aabb.walk_off, WellSide::NegZ);
		assert!(aabb.half_x() > 1.1 && aabb.half_z() > 1.1);
		Ok(())
	}

	#[test]
	fn run_in_follows_walk_on_into_the_box() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::X)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::X)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		assert!((well.run_in().thickness() - SLAB_THICKNESS_M).abs() < 1e-3);
		let [a0, a1, b0, b1] = well.run_in().corners();
		assert!(
			(a0.y - (-PANEL_Y_HALF)).abs() < 1e-3,
			"run-in kit center should sit {PANEL_Y_HALF} below the walk-on, got y={}",
			a0.y
		);
		let inward = (b0 + b1) * 0.5 - (a0 + a1) * 0.5;
		assert!(inward.x > 0.5, "run-in should follow +X into the shaft, got {inward:?}");
		Ok(())
	}

	#[test]
	fn spiral_stays_in_the_box_and_lands_to_walk_off() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let well = ConnectingStairwell::rough_stone(lower, upper);
		let stairs = well.stairs();
		assert!(!stairs.is_empty());
		assert!(stairs.iter().all(|s| matches!(s.geometry, Stair::Straight(_))));
		let aabb = well.well();
		for s in stairs {
			let p = s.placement.translation;
			assert!(aabb.contains_xz(p.x, p.z), "spiral treads should sit in the well, got {p:?}");
		}
		let landing = well.upper_landing().expect("walk-off landing");
		let walk_off = aabb.side_mid(aabb.walk_off, aabb.top_y());
		let door = aabb.walk_off.into_xz();
		let on_door = landing.corners().into_iter().any(|p| {
			let along_out = (p.x - walk_off.x) * door.x + (p.z - walk_off.z) * door.y;
			along_out.abs() < 0.04
		});
		assert!(on_door, "landing should sit on the walk-off edge");
		let last = well.last_tread_end().expect("last tread");
		let last_mid = last.leading_mid();
		let last_to_door = (last_mid - Vec2::new(walk_off.x, walk_off.z)).length();
		assert!(
			last_to_door > 0.15,
			"last tread must not sit on the walk-off, dist={last_to_door}"
		);
		let [c0, c1, c2, c3] = landing.corners();
		let pad_min =
			Vec2::new(c0.x.min(c1.x).min(c2.x).min(c3.x), c0.z.min(c1.z).min(c2.z).min(c3.z));
		let pad_max =
			Vec2::new(c0.x.max(c1.x).max(c2.x).max(c3.x), c0.z.max(c1.z).max(c2.z).max(c3.z));
		for (label, p) in [("outer", last.leading_outer), ("inner", last.leading_inner)] {
			assert!(
				p.x >= pad_min.x - 0.04
					&& p.x <= pad_max.x + 0.04
					&& p.y >= pad_min.y - 0.04
					&& p.y <= pad_max.y + 0.04,
				"landing must cover last leading {label} {p:?}, pad {pad_min:?}..{pad_max:?}"
			);
		}
		let along = match aabb.walk_off {
			WellSide::NegZ | WellSide::PosZ => pad_max.x - pad_min.x,
			WellSide::NegX | WellSide::PosX => pad_max.y - pad_min.y,
		};
		let well_along = match aabb.walk_off {
			WellSide::NegZ | WellSide::PosZ => aabb.max().x - aabb.min().x,
			WellSide::NegX | WellSide::PosX => aabb.max().z - aabb.min().z,
		};
		assert!(
			(along - well_along).abs() < 0.04,
			"landing along-wall {along} should span the well face {well_along}"
		);
		Ok(())
	}

	#[test]
	fn quarter_turn_landing_is_a_strip_not_a_lid() {
		let well = ConnectingStairwell::from_well(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::NegX,
				TREAD_FILL_DEFAULT,
			),
		);
		let aabb = well.well();
		let landing = well.upper_landing().expect("walk-off landing");
		let [c0, c1, c2, c3] = landing.corners();
		let pad_min =
			Vec2::new(c0.x.min(c1.x).min(c2.x).min(c3.x), c0.z.min(c1.z).min(c2.z).min(c3.z));
		let pad_max =
			Vec2::new(c0.x.max(c1.x).max(c2.x).max(c3.x), c0.z.max(c1.z).max(c2.z).max(c3.z));
		let inward = pad_max.x - pad_min.x;
		assert!(
			inward < aabb.half_x() + 0.04,
			"quarter-turn landing must not lid the well, inward={inward}"
		);
		let center = aabb.center_xz();
		assert!(
			center.x < pad_min.x - 0.04 || center.x > pad_max.x + 0.04,
			"well center {center:?} should stay off the landing {pad_min:?}..{pad_max:?}"
		);
		let last = well.last_tread_end().expect("last tread");
		let door = aabb.side_mid(aabb.walk_off, aabb.top_y());
		let last_to_door = (last.leading_mid() - Vec2::new(door.x, door.z)).length();
		assert!(
			last_to_door < aabb.half_min(),
			"last tread should arrive at the walk-off strip, dist={last_to_door}"
		);
	}

	#[test]
	fn omitting_upper_landing_leaves_only_run_in() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let with = ConnectingStairwell::rough_stone(lower, upper);
		let without = ConnectingStairwell::rough_stone(lower, upper).with_upper_landing(false);
		assert!(with.upper_landing().is_some());
		assert!(without.upper_landing().is_none());
		Ok(())
	}

	#[test]
	fn tread_fill_widens_treads() -> anyhow::Result<()> {
		let lower = shaft_opening(Vec3::new(0.0, 0.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let upper = shaft_opening(Vec3::new(0.0, 3.0, 0.0), 1.2, 1.2, Vec2::Y)?;
		let default = ConnectingStairwell::rough_stone(lower, upper);
		let wide = ConnectingStairwell::rough_stone(lower, upper).with_tread_fill(0.8);
		assert!((default.tread_fill() - TREAD_FILL_DEFAULT).abs() < 1e-4);
		assert!((wide.tread_fill() - 0.8).abs() < 1e-4);
		let w0 = first_tread_width(&default);
		let w1 = first_tread_width(&wide);
		assert!(w1 > w0 + 0.2, "fill 0.8 should be wider than default, {w0} vs {w1}");
		Ok(())
	}

	#[test]
	fn tall_well_adds_turns_to_protect_going() {
		let short = WellAabb::from_plan(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
			TREAD_FILL_DEFAULT,
		);
		let tall = WellAabb::from_plan(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 6.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
			TREAD_FILL_DEFAULT,
		);
		let a = ConnectingStairwell::from_well(PanelStyle::RoughStonework, short);
		let b = ConnectingStairwell::from_well(PanelStyle::RoughStonework, tall);
		assert!(b.stairs().len() > a.stairs().len());
		for s in b.stairs() {
			let Stair::Straight(g) = &s.geometry else {
				panic!("spiral well should emit Straight treads");
			};
			assert!(
				g.going_per_tread() + 1e-3 >= spiral::MIN_GOING,
				"going {} below floor",
				g.going_per_tread()
			);
		}
	}

	#[test]
	fn short_well_keeps_one_lap_instead_of_stacking() {
		let tiny = WellAabb::from_plan(
			Vec3::new(-0.6, 0.0, -0.6),
			Vec3::new(0.6, 1.5, 0.6),
			WellSide::NegZ,
			WellSide::NegZ,
			TREAD_FILL_DEFAULT,
		);
		let well = ConnectingStairwell::from_well(PanelStyle::RoughStonework, tiny);
		assert!(!well.stairs().is_empty());
		let going = match &well.stairs()[0].geometry {
			Stair::Straight(g) => g.going_per_tread(),
			Stair::Spiral(_) => panic!("spiral well should emit Straight treads"),
		};
		let center = tiny.center_xz();
		let p = well.stairs()[0].placement.translation;
		let radius = (Vec2::new(p.x, p.z) - center).length().max(1e-4);
		let intervals = well.stairs().len().saturating_sub(1).max(1) as f32;
		let turns = going * intervals / (std::f32::consts::TAU * radius);
		assert!(turns < 1.5, "1.5 m well must not add a second lap, turns={turns} going={going}");
		assert!(
			tiny.rise() / turns.max(1e-4) + 1e-3 >= tiny.rise() - 0.05,
			"one lap should keep the full rise as headroom, turns={turns}"
		);
	}

	#[test]
	fn stacked_wells_share_the_walk_off_face() {
		let lower = WellAabb::from_plan(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
			TREAD_FILL_DEFAULT,
		);
		let upper = WellAabb::from_plan(
			Vec3::new(-1.2, 3.0, -1.2),
			Vec3::new(1.2, 6.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
			TREAD_FILL_DEFAULT,
		);
		assert_eq!(lower.walk_off, upper.walk_on);
		assert!((lower.top_y() - upper.bottom_y()).abs() < 1e-4);
		let a = ConnectingStairwell::from_well(PanelStyle::RoughStonework, lower)
			.with_upper_landing(false);
		let b = ConnectingStairwell::from_well(PanelStyle::RoughStonework, upper);
		assert!(a.upper_landing().is_none());
		assert!(b.upper_landing().is_some());
		assert!(!a.stairs().is_empty());
		assert!(!b.stairs().is_empty());
	}

	#[test]
	fn rectangular_same_side_hugs_four_walls() {
		let well = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::NegZ,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		);
		assert_eq!(well.kind(), StairwellKind::Rectangular);
		assert_eq!(well.stairs().len(), 4);
		assert_eq!(well.mid_landings().len(), 3);
		assert!(well.upper_landing().is_some());
		let aabb = well.well();
		for s in well.stairs() {
			let p = s.placement.translation;
			assert!(
				aabb.contains_xz(p.x, p.z),
				"rectangular flight should stay in the well, {p:?}"
			);
		}
	}

	#[test]
	fn rectangular_opposite_is_two_flights() {
		let well = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::PosZ,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		);
		assert_eq!(well.stairs().len(), 2);
		assert_eq!(well.mid_landings().len(), 1);
	}

	#[test]
	fn rectangular_quarter_landing_is_a_strip() {
		let well = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::NegX,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		);
		let aabb = well.well();
		let landing = well.upper_landing().expect("walk-off");
		let [c0, c1, c2, c3] = landing.corners();
		let pad_min =
			Vec2::new(c0.x.min(c1.x).min(c2.x).min(c3.x), c0.z.min(c1.z).min(c2.z).min(c3.z));
		let pad_max =
			Vec2::new(c0.x.max(c1.x).max(c2.x).max(c3.x), c0.z.max(c1.z).max(c2.z).max(c3.z));
		let center = aabb.center_xz();
		assert!(
			center.x < pad_min.x - 0.04 || center.x > pad_max.x + 0.04,
			"rectangular quarter-turn must not lid the well"
		);
		assert!(!well.stairs().is_empty());
	}

	#[test]
	fn rectangular_last_leading_meets_the_end_pad() {
		let well = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-1.2, 0.0, -1.2),
				Vec3::new(1.2, 3.0, 1.2),
				WellSide::NegZ,
				WellSide::NegZ,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		);
		let aabb = well.well();
		for (i, node) in well.stairs().iter().enumerate() {
			let end = TreadEnd::from_straight(node);
			let side = [WellSide::NegZ, WellSide::PosX, WellSide::PosZ, WellSide::NegX]
				.into_iter()
				.max_by(|a, b| {
					end.travel
						.dot(a.travel_xz())
						.partial_cmp(&end.travel.dot(b.travel_xz()))
						.unwrap_or(std::cmp::Ordering::Equal)
				})
				.expect("side");
			let want = rect::flight_end_leading(&aabb, side);
			let got = end.leading_mid();
			assert!(
				(got - want).length() < 0.05,
				"flight {i} last leading {got:?} should meet end pad {want:?} on {side:?}"
			);
		}
	}

	#[test]
	fn rectangular_skinny_collapses_the_hole() {
		let well = ConnectingStairwell::from_well_kind(
			PanelStyle::RoughStonework,
			WellAabb::from_plan(
				Vec3::new(-0.35, 0.0, -1.4),
				Vec3::new(0.35, 3.0, 1.4),
				WellSide::NegZ,
				WellSide::NegZ,
				TREAD_FILL_DEFAULT,
			),
			StairwellKind::Rectangular,
		);
		assert!(!well.stairs().is_empty());
		let aabb = well.well();
		for s in well.stairs() {
			let p = s.placement.translation;
			assert!(
				aabb.contains_xz(p.x, p.z),
				"skinny rectangular should still hug the box, {p:?}"
			);
		}
	}

	fn first_tread_width(well: &ConnectingStairwell) -> f32 {
		match &well.stairs()[0].geometry {
			Stair::Straight(g) => g.width,
			Stair::Spiral(_) => panic!("spiral well should emit Straight treads"),
		}
	}
}
