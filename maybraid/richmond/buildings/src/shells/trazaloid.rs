//! A trapezoid-like shell: two stacked frustum bands with an optional waist reveal.
//!
//! Good for modern and sci-fi buildings, outposts, and the like.
//!
//! Lower band: four walls (no floor / no ceiling). Upper band: four walls + ridge
//! ceiling. Optional centered door clips on lower sides. High LOD adds vertical
//! [`JointNode`] posts at corners and densified face generators.

use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::geometry::JointPost;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::paneling::clipped_ruled_strip::ClippedRuledStrip;
use crate::paneling::panel_complex::{PanelComplexJointPolicy, DEFAULT_PANEL_THICKNESS};

const EXTENT_EPS: f32 = 1e-3;
const GAP_EPS: f32 = 1e-4;

/// Which lower-band sides get a centered inset door clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrazaloidDoors {
	pub north: bool,
	pub east: bool,
	pub south: bool,
	pub west: bool,
}

impl TrazaloidDoors {
	pub const NONE: Self = Self {
		north: false,
		east: false,
		south: false,
		west: false,
	};

	pub const ALL: Self = Self {
		north: true,
		east: true,
		south: true,
		west: true,
	};
}

impl Default for TrazaloidDoors {
	fn default() -> Self {
		Self::NONE
	}
}

/// Authored parameters for a [`Trazaloid`] shell.
#[derive(Debug, Clone, PartialEq)]
pub struct TrazaloidParams {
	/// Full width (X) / depth (Z) at the footprint (`y = 0`).
	pub footprint: Vec2,
	/// Full width (X) / depth (Z) at the ridge.
	pub ridge: Vec2,
	pub lower_height: f32,
	pub upper_height: f32,
	/// Vertical gap between lower top and upper bottom.
	pub band_vertical_offset: f32,
	/// Inward meters from the linear footprint→ridge silhouette at lower-top height.
	pub waist_horizontal_offset: f32,
	pub doors: TrazaloidDoors,
	/// Door width as a fraction of the lower face width (centered).
	pub door_width_frac: f32,
	/// Door height as a fraction of the lower face height (centered).
	pub door_height_frac: f32,
	pub style: PanelStyle,
	pub joint_thickness: f32,
	/// Extra vertical posts per face (between corners) at [`LodSceneLevel::High`].
	pub face_post_count: u32,
}

impl Default for TrazaloidParams {
	fn default() -> Self {
		Self {
			footprint: Vec2::new(8.0, 6.0),
			ridge: Vec2::new(4.0, 3.0),
			lower_height: 3.0,
			upper_height: 2.5,
			band_vertical_offset: 0.35,
			waist_horizontal_offset: 0.25,
			doors: TrazaloidDoors {
				south: true,
				..TrazaloidDoors::NONE
			},
			door_width_frac: 0.28,
			door_height_frac: 0.55,
			style: PanelStyle::RoughStonework,
			joint_thickness: DEFAULT_PANEL_THICKNESS,
			face_post_count: 2,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlanRect {
	y: f32,
	half_x: f32,
	half_z: f32,
}

impl PlanRect {
	fn sw(self) -> Vec3 {
		Vec3::new(-self.half_x, self.y, -self.half_z)
	}
	fn se(self) -> Vec3 {
		Vec3::new(self.half_x, self.y, -self.half_z)
	}
	fn ne(self) -> Vec3 {
		Vec3::new(self.half_x, self.y, self.half_z)
	}
	fn nw(self) -> Vec3 {
		Vec3::new(-self.half_x, self.y, self.half_z)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
	North,
	East,
	South,
	West,
}

impl Side {
	fn all() -> [Self; 4] {
		[Self::North, Self::East, Self::South, Self::West]
	}

	fn door_enabled(self, doors: &TrazaloidDoors) -> bool {
		match self {
			Self::North => doors.north,
			Self::East => doors.east,
			Self::South => doors.south,
			Self::West => doors.west,
		}
	}

	/// Outward horizontal unit normal in XZ.
	fn outward(self) -> Vec3 {
		match self {
			Self::North => Vec3::Z,
			Self::East => Vec3::X,
			Self::South => -Vec3::Z,
			Self::West => -Vec3::X,
		}
	}
}

/// One vertical post segment for high-LOD joint emission.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PostSegment {
	start: Vec3,
	end: Vec3,
	radial_hint: Vec3,
}

/// Two-band trapezoidal-pyramid shell.
#[derive(Debug, Clone, PartialEq)]
pub struct Trazaloid {
	params: TrazaloidParams,
	joint_policy: PanelComplexJointPolicy,
	lower_walls: [ClippedRuledStrip; 4],
	upper_walls: [ClippedRuledStrip; 4],
	ridge_ceiling: ClippedRuledStrip,
	high_posts: Vec<PostSegment>,
	/// Resolved plan rectangles (foot, waist/lower-top, upper-bottom, ridge).
	rects: [PlanRect; 4],
}

impl Trazaloid {
	pub fn new(params: TrazaloidParams) -> Self {
		let rects = resolve_rects(&params);
		let [foot, waist, upper_bot, ridge] = rects;
		let style = params.style;
		let policy = PanelComplexJointPolicy::default();

		let lower_walls = Side::all().map(|side| {
			let (a0, b0) = face_bottom_pair(side, foot);
			let (a1, b1) = face_bottom_pair(side, waist);
			let clip = if side.door_enabled(&params.doors) {
				Some(centered_door_clip(a0, b0, a1, b1, params.door_width_frac, params.door_height_frac))
			} else {
				None
			};
			ClippedRuledStrip::from_lines(style, [a0, a1], [b0, b1], [clip]).with_joint_policy(policy)
		});

		let upper_walls = Side::all().map(|side| {
			let (a0, b0) = face_bottom_pair(side, upper_bot);
			let (a1, b1) = face_bottom_pair(side, ridge);
			ClippedRuledStrip::from_lines(style, [a0, a1], [b0, b1], [None]).with_joint_policy(policy)
		});

		// Ridge ceiling: rail_a = south edge (SW→SE), rail_b = north (NW→NE), one bay along X.
		// Stations along X so the strip covers the ridge rectangle.
		let ridge_ceiling = ClippedRuledStrip::from_lines(
			style,
			[ridge.sw(), ridge.se()],
			[ridge.nw(), ridge.ne()],
			[None],
		)
		.with_joint_policy(policy);

		let high_posts = build_high_posts(&params, &rects);

		Self {
			params,
			joint_policy: policy,
			lower_walls,
			upper_walls,
			ridge_ceiling,
			high_posts,
			rects,
		}
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		self.lower_walls = self.lower_walls.map(|w| w.with_joint_policy(joint_policy));
		self.upper_walls = self.upper_walls.map(|w| w.with_joint_policy(joint_policy));
		self.ridge_ceiling = std::mem::replace(
			&mut self.ridge_ceiling,
			ClippedRuledStrip::new(self.params.style),
		)
		.with_joint_policy(joint_policy);
		self
	}

	pub fn params(&self) -> &TrazaloidParams {
		&self.params
	}

	pub fn lower_walls(&self) -> &[ClippedRuledStrip; 4] {
		&self.lower_walls
	}

	pub fn upper_walls(&self) -> &[ClippedRuledStrip; 4] {
		&self.upper_walls
	}

	pub fn ridge_ceiling(&self) -> &ClippedRuledStrip {
		&self.ridge_ceiling
	}

	/// Foot, waist, upper-bottom, ridge half-extents and heights.
	pub fn plan_levels(&self) -> [(f32, Vec2); 4] {
		self.rects.map(|r| (r.y, Vec2::new(r.half_x * 2.0, r.half_z * 2.0)))
	}
}

impl BuildingComponents for Trazaloid {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for w in &self.lower_walls {
			out.extend(w.panel_nodes_for_level(level));
		}
		for w in &self.upper_walls {
			out.extend(w.panel_nodes_for_level(level));
		}
		out.extend(self.ridge_ceiling.panel_nodes_for_level(level));
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for w in &self.lower_walls {
			out.extend(w.joint_nodes_for_level(level));
		}
		for w in &self.upper_walls {
			out.extend(w.joint_nodes_for_level(level));
		}
		out.extend(self.ridge_ceiling.joint_nodes_for_level(level));

		if matches!(level, LodSceneLevel::High) {
			let thickness = self.params.joint_thickness.max(1e-4);
			for seg in &self.high_posts {
				if let Some(placement) =
					JointPost::placed_along_crease(seg.start, seg.end, thickness, seg.radial_hint)
				{
					out.push_free(JointNode::rough_stone_post(placement));
				}
			}
		}
		out
	}
}

fn resolve_rects(params: &TrazaloidParams) -> [PlanRect; 4] {
	let foot_x = params.footprint.x.max(EXTENT_EPS) * 0.5;
	let foot_z = params.footprint.y.max(EXTENT_EPS) * 0.5;
	let ridge_x = params.ridge.x.max(EXTENT_EPS) * 0.5;
	let ridge_z = params.ridge.y.max(EXTENT_EPS) * 0.5;
	let lower_h = params.lower_height.max(EXTENT_EPS);
	let upper_h = params.upper_height.max(EXTENT_EPS);
	let gap = params.band_vertical_offset.max(0.0);
	let total = lower_h + gap + upper_h;
	let t = (lower_h / total).clamp(0.0, 1.0);

	let silhouette_x = foot_x + (ridge_x - foot_x) * t;
	let silhouette_z = foot_z + (ridge_z - foot_z) * t;
	let inset = params.waist_horizontal_offset.max(0.0);
	let waist_x = (silhouette_x - inset).max(EXTENT_EPS);
	let waist_z = (silhouette_z - inset).max(EXTENT_EPS);

	[
		PlanRect {
			y: 0.0,
			half_x: foot_x,
			half_z: foot_z,
		},
		PlanRect {
			y: lower_h,
			half_x: waist_x,
			half_z: waist_z,
		},
		PlanRect {
			y: lower_h + gap,
			half_x: waist_x,
			half_z: waist_z,
		},
		PlanRect {
			y: lower_h + gap + upper_h,
			half_x: ridge_x,
			half_z: ridge_z,
		},
	]
}

/// Bottom-left / bottom-right of a face when viewed from outside (left = rail_a).
fn face_bottom_pair(side: Side, rect: PlanRect) -> (Vec3, Vec3) {
	match side {
		// Outside looking −Z: left = West (NW), right = East (NE).
		Side::North => (rect.nw(), rect.ne()),
		// Outside looking −X: left = North (NE), right = South (SE).
		Side::East => (rect.ne(), rect.se()),
		// Outside looking +Z: left = East (SE), right = West (SW).
		Side::South => (rect.se(), rect.sw()),
		// Outside looking +X: left = South (SW), right = North (NW).
		Side::West => (rect.sw(), rect.nw()),
	}
}

fn centered_door_clip(
	a0: Vec3,
	b0: Vec3,
	a1: Vec3,
	b1: Vec3,
	width_frac: f32,
	height_frac: f32,
) -> Vec<Vec3> {
	let w = width_frac.clamp(0.05, 0.95);
	let h = height_frac.clamp(0.05, 0.95);
	let u0 = (1.0 - w) * 0.5;
	let u1 = u0 + w;
	let v0 = (1.0 - h) * 0.5;
	let v1 = v0 + h;
	// Bilinear on the face quad {a0,a1,b0,b1} with u along bottom a0→b0, v up a0→a1.
	let p = |u: f32, v: f32| {
		let bottom = a0.lerp(b0, u);
		let top = a1.lerp(b1, u);
		bottom.lerp(top, v)
	};
	vec![p(u0, v0), p(u1, v0), p(u1, v1), p(u0, v1)]
}

fn build_high_posts(params: &TrazaloidParams, rects: &[PlanRect; 4]) -> Vec<PostSegment> {
	let [foot, waist, upper_bot, ridge] = *rects;
	let gap = upper_bot.y - waist.y;
	let mut posts = Vec::new();

	let corners = |r: PlanRect| [r.sw(), r.se(), r.ne(), r.nw()];
	let foot_c = corners(foot);
	let waist_c = corners(waist);
	let upper_c = corners(upper_bot);
	let ridge_c = corners(ridge);

	for i in 0..4 {
		let radial = (foot_c[i] - Vec3::new(0.0, foot_c[i].y, 0.0)).normalize_or_zero();
		let radial = if radial.length_squared() > 0.0 {
			radial
		} else {
			Vec3::X
		};
		posts.push(PostSegment {
			start: foot_c[i],
			end: waist_c[i],
			radial_hint: radial,
		});
		if gap > GAP_EPS {
			posts.push(PostSegment {
				start: waist_c[i],
				end: upper_c[i],
				radial_hint: radial,
			});
		}
		posts.push(PostSegment {
			start: upper_c[i],
			end: ridge_c[i],
			radial_hint: radial,
		});
	}

	let n = params.face_post_count;
	if n > 0 {
		for side in Side::all() {
			let outward = side.outward();
			// Lower band face posts.
			let (la0, lb0) = face_bottom_pair(side, foot);
			let (la1, lb1) = face_bottom_pair(side, waist);
			push_face_posts(&mut posts, la0, lb0, la1, lb1, n, outward);
			// Upper band face posts.
			let (ua0, ub0) = face_bottom_pair(side, upper_bot);
			let (ua1, ub1) = face_bottom_pair(side, ridge);
			push_face_posts(&mut posts, ua0, ub0, ua1, ub1, n, outward);
		}
	}

	posts
}

fn push_face_posts(
	posts: &mut Vec<PostSegment>,
	a0: Vec3,
	b0: Vec3,
	a1: Vec3,
	b1: Vec3,
	count: u32,
	radial_hint: Vec3,
) {
	let denom = (count + 1) as f32;
	for i in 1..=count {
		let u = i as f32 / denom;
		let start = a0.lerp(b0, u);
		let end = a1.lerp(b1, u);
		posts.push(PostSegment {
			start,
			end,
			radial_hint,
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::paneling::clipped_ruled_strip::ClippedStripPiece;

	fn demo_params() -> TrazaloidParams {
		TrazaloidParams::default()
	}

	#[test]
	fn resolves_waist_inset_and_gap() {
		let t = Trazaloid::new(demo_params());
		let levels = t.plan_levels();
		assert!((levels[0].0 - 0.0).abs() < 1e-5);
		assert!((levels[1].0 - 3.0).abs() < 1e-5);
		assert!((levels[2].0 - 3.35).abs() < 1e-5);
		assert!((levels[3].0 - 5.85).abs() < 1e-5);
		// Waist smaller than linear silhouette by 2*inset on full width.
		assert!(levels[1].1.x < levels[0].1.x);
		assert!(levels[1].1.x > levels[3].1.x - 1.0);
		assert_eq!(levels[1].1, levels[2].1);
	}

	#[test]
	fn lower_has_walls_upper_has_ceiling() {
		let t = Trazaloid::new(demo_params());
		for w in t.lower_walls() {
			assert!(!w.pieces().is_empty());
		}
		for w in t.upper_walls() {
			assert!(!w.pieces().is_empty());
		}
		assert!(!t.ridge_ceiling().pieces().is_empty());
		// Four lower walls + four upper + ceiling = panels; no separate lower ceiling strip.
		let high = t.panel_nodes_for_level(LodSceneLevel::High).len();
		let without_ceiling = {
			let mut n = 0;
			for w in t.lower_walls() {
				n += w.panel_nodes_for_level(LodSceneLevel::High).len();
			}
			for w in t.upper_walls() {
				n += w.panel_nodes_for_level(LodSceneLevel::High).len();
			}
			n
		};
		assert!(high > without_ceiling);
		assert_eq!(
			high,
			without_ceiling + t.ridge_ceiling().panel_nodes_for_level(LodSceneLevel::High).len()
		);
	}

	#[test]
	fn south_door_makes_clipped_lower_piece() {
		let mut params = demo_params();
		params.doors = TrazaloidDoors {
			south: true,
			..TrazaloidDoors::NONE
		};
		let t = Trazaloid::new(params);
		// Side order: N, E, S, W → index 2 is south.
		assert!(matches!(
			t.lower_walls()[2].pieces()[0],
			ClippedStripPiece::Clipped(_)
		));
		assert!(matches!(
			t.lower_walls()[0].pieces()[0],
			ClippedStripPiece::Solid(_)
		));
	}

	#[test]
	fn high_emits_more_joints_than_medium() {
		let t = Trazaloid::new(demo_params());
		let high = t.joint_nodes_for_level(LodSceneLevel::High).len();
		let mid = t.joint_nodes_for_level(LodSceneLevel::Medium).len();
		assert!(high > mid);
		assert!(high >= t.high_posts.len());
	}
}
