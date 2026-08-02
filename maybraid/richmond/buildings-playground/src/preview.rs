//! Preview subject sync. Viewer tracking lives in [`lod::LodFinePassPlugin`].

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Isometry3d, Vec2};
use lod::gen::LodScene;
use lod::LodViewerState;
use procedural_common::{AllowedAngles, NoiseParams, StepLenRange};
use richmond_building_components::panels::{PanelGeometry, PanelNode, TessellatedTriangle};
use richmond_building_components::partitions::rough_stonework::{
	RoughStonework180, RoughStonework90, RoughStoneworkLinear, RoughStoneworkSlice90,
};
use richmond_building_components::partitions::{Partition, PartitionNode};
use richmond_building_components::roofs::{Pitch, RoofGeometry, RoofNode};
use richmond_building_components::Placement;
use richmond_building_components::ComponentsOnly;
use richmond_buildings::bedroom::Bedroom;
use richmond_buildings::panel_complex::{PanelComplex, PanelComplexJointPolicy, PanelPoint};
use richmond_buildings::quad_panel::QuadPanel;
use richmond_buildings::quad_panel_complex::QuadPanelComplex;
use crate::commands::show::opening::{openings_from_preview, PreviewOpening};
use richmond_buildings::{
	ApproximatedCircle, ArcFloor, ArcFloorParams, ArcFloorSlab, ArcSweep, ArcTower, ArcTowerParams,
	ClippedArcSweep, ClippedFittedRectangle, ClippedFittedRectangularStrip, ClippedQuadPanel,
	ClippedRectangle, ClippedRectangularStrip, ClippedRuledStrip, ClippedTessellatedTriangle,
	ConnectingHall, ConnectingShells, FittedRectangle, IFloor, IFloorParams, IFloorSlab,
	MappedOpening, MappedOpeningQuad, MapsOpenings, OpeningId, OpeningLabel, Openings, PitchedRoof,
	PitchedRoofParams, RectFloor, RectFloorParams, RectFloorSlab, RectInset, Rectangle,
	RectangularNTube, RectangularNTubeCorner, RectangularNTubeStation, RectangularStripNode,
	RoundedRectFloor, RoundedRectFloorParams, RoundedRectFloorSlab, RuledPitch, Tube,
	TubeCrossSectionNode, TubeFaces, Trazaloid, TrazaloidParams, TrazaloidSlab,
	DEFAULT_PANEL_THICKNESS,
};
use richmond_buildings::stacked_rings::StackedRings;
use richmond_buildings::tessellated_triangle_panel::TessellatedTrianglePanel;
use richmond_buildings::portals::{MustAssignPortal, Portal};
use richmond_buildings::wall_demo::{NoisyRectangularWall, NoisyRectangularWallParams};
use richmond_buildings::wizards_tower::WizardsTower;
use richmond_buildings::{
	BedroomFillParams, CellConstraints, CirculationEntry, CirculationRequestStatus,
};

#[derive(Component)]
pub struct PreviewRoot;

#[derive(Clone, Debug, PartialEq)]
pub enum PreviewSubject {
	None,
	Linear,
	Arc90,
	Arc180,
	Slice90,
	Pitch {
		rise: f32,
		run: f32,
		length: Option<f32>,
		tile_width: f32,
		left: Option<f32>,
		right: Option<f32>,
	},
	TessellatedTriangle {
		a: Vec2,
		b: Vec2,
		c: Vec2,
	},
	TessellatedTriangle3d {
		a: Vec3,
		b: Vec3,
		c: Vec3,
	},
	ClippedTessellatedTriangle {
		a: Vec3,
		b: Vec3,
		c: Vec3,
		clip: Vec<Vec3>,
		min_dihedral: f32,
		no_joint: bool,
	},
	ClippedQuadPanel {
		a0: Vec3,
		a1: Vec3,
		b0: Vec3,
		b1: Vec3,
		clip: Vec<Vec3>,
		min_dihedral: f32,
		no_joint: bool,
	},
	ClippedRuledStrip {
		min_dihedral: f32,
		no_joint: bool,
	},
	Tube {
		min_dihedral: f32,
		no_joint: bool,
		no_floor: bool,
		no_ceiling: bool,
		no_left: bool,
		no_right: bool,
	},
	ConnectingHall,
	ArcFloor {
		radius: f32,
		storey_height: f32,
		floor: bool,
		ceiling: bool,
		openings: Vec<PreviewOpening>,
	},
	ArcTower {
		radius: f32,
		floor_count: u32,
		storey_height: f32,
		floor_hole: f32,
		no_base_floor: bool,
		no_ceiling: bool,
	},
	ConnectingShells,
	Trazaloid {
		footprint_x: f32,
		footprint_z: f32,
		ridge_x: f32,
		ridge_z: f32,
		lower_height: f32,
		upper_height: f32,
		band_vertical_offset: f32,
		waist_horizontal_offset: f32,
		openings: Vec<PreviewOpening>,
		floor: bool,
		no_ceiling: bool,
		face_post_count: u32,
	},
	PitchedRectangularRoof {
		footprint_x: f32,
		footprint_z: f32,
		ridge_height: f32,
		eave_height: f32,
		ridge_inset: f32,
		gables: bool,
		no_walls: bool,
		no_hips: bool,
		openings: Vec<PreviewOpening>,
	},
	RectFloor {
		footprint_x: f32,
		footprint_z: f32,
		storey_height: f32,
		openings: Vec<PreviewOpening>,
		floor: bool,
		ceiling: bool,
	},
	RoundedRectFloor {
		footprint_x: f32,
		footprint_z: f32,
		storey_height: f32,
		corner_radius: f32,
		corner_segments: u32,
		openings: Vec<PreviewOpening>,
		floor: bool,
		ceiling: bool,
	},
	IFloor {
		central_x: f32,
		central_z: f32,
		storey_height: f32,
		top_left: Option<f32>,
		top_right: Option<f32>,
		bottom_left: Option<f32>,
		bottom_right: Option<f32>,
		openings: Vec<PreviewOpening>,
		floor: bool,
		ceiling: bool,
	},
	Rectangle {
		origin: Vec3,
		edge: Vec3,
		height: f32,
		thickness: f32,
		roll: f32,
	},
	ClippedRectangle {
		origin: Vec3,
		edge: Vec3,
		height: f32,
		thickness: f32,
		roll: f32,
		left: f32,
		right: f32,
		bottom: f32,
		top: f32,
	},
	ClippedRectangularStrip {
		inset: f32,
		min_dihedral: f32,
		no_joint: bool,
	},
	FittedRectangle {
		a0: Vec3,
		a1: Vec3,
		b0: Vec3,
		b1: Vec3,
	},
	ClippedFittedRectangle {
		a0: Vec3,
		a1: Vec3,
		b0: Vec3,
		b1: Vec3,
		left: f32,
		right: f32,
		bottom: f32,
		top: f32,
	},
	ClippedFittedRectangularStrip {
		inset: f32,
		min_dihedral: f32,
		no_joint: bool,
	},
	RectangularNTube {
		inset: f32,
		min_dihedral: f32,
		no_joint: bool,
		omit_faces: Vec<usize>,
	},
	ApproximatedCircle {
		center: Vec3,
		radius: f32,
		segments: u32,
		clip: Option<f32>,
	},
	ArcSweep {
		radius: f32,
		height: f32,
		sweep_degrees: f32,
		start_yaw_deg: f32,
	},
	ClippedArcSweep {
		radius: f32,
		height: f32,
		sweep_degrees: f32,
		start_yaw_deg: f32,
	},
	QuadPanel {
		a0: Vec3,
		a1: Vec3,
		b0: Vec3,
		b1: Vec3,
		t_a0: f32,
		t_a1: f32,
		t_b0: f32,
		t_b1: f32,
		min_dihedral: f32,
		no_joint: bool,
	},
	PanelComplex {
		mesh: String,
		min_dihedral: f32,
		no_joint: bool,
	},
	QuadPanelComplex {
		mesh: String,
		min_dihedral: f32,
		no_joint: bool,
	},
	RuledPitch {
		min_dihedral: f32,
		no_joint: bool,
	},
	Polyline,
	NoisyRectangularWall {
		distance: f32,
		step_len: StepLenRange,
		allowed_angles: AllowedAngles,
		path_noise: NoiseParams,
	},
	WizardsTower {
		noise: f32,
	},
	StackedRings {
		floor_count: u32,
		floor_height: f32,
		radius: f32,
	},
	Bedroom {
		/// Cell size along X / Y / Z (AABB from origin to `extent`).
		extent: Vec3,
		/// Unit noise for layout fitting.
		noise: f32,
		spaciousness: f32,
		occupancy: f32,
		/// When true, add a required −Z door circulation region.
		door: bool,
	},
}

impl Default for PreviewSubject {
	fn default() -> Self {
		Self::None
	}
}

#[derive(Resource, Clone, Debug)]
pub struct PreviewConfig {
	pub subject: PreviewSubject,
	pub transform: Transform,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self { subject: PreviewSubject::None, transform: Transform::IDENTITY }
	}
}

impl PreviewConfig {
	pub fn status_label(&self) -> String {
		match self.subject {
			PreviewSubject::None => "preview: (none — `/show …`)".into(),
			PreviewSubject::Linear => "preview: rough-stonework linear".into(),
			PreviewSubject::Arc90 => "preview: rough-stonework arc-90".into(),
			PreviewSubject::Arc180 => "preview: rough-stonework arc-180".into(),
			PreviewSubject::Slice90 => "preview: rough-stonework slice-90".into(),
			PreviewSubject::Pitch {
				rise,
				run,
				length,
				tile_width,
				left,
				right,
			} => {
				format!(
					"preview: pitch (rise={rise:.2} run={run:.2} len={length:?} tile={tile_width:.2} left={left:?} right={right:?})"
				)
			}
			PreviewSubject::TessellatedTriangle { a, b, c } => {
				format!("preview: tessellated-triangle (a={a:?} b={b:?} c={c:?})")
			}
			PreviewSubject::TessellatedTriangle3d { a, b, c } => {
				format!("preview: tessellated-triangle-3d (a={a:?} b={b:?} c={c:?})")
			}
			PreviewSubject::ClippedTessellatedTriangle {
				a,
				b,
				c,
				ref clip,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: clipped-tessellated-triangle (a={a:?} b={b:?} c={c:?} clip={clip:?} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::ClippedQuadPanel {
				a0,
				a1,
				b0,
				b1,
				ref clip,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: clipped-quad-panel (a0={a0:?} a1={a1:?} b0={b0:?} b1={b1:?} clip={clip:?} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::ClippedRuledStrip {
				min_dihedral,
				no_joint,
			} => format!(
				"preview: clipped-ruled-strip (min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::Tube {
				min_dihedral,
				no_joint,
				no_floor,
				no_ceiling,
				no_left,
				no_right,
			} => format!(
				"preview: tube (min_dihedral={min_dihedral:.3} no_joint={no_joint} no_floor={no_floor} no_ceiling={no_ceiling} no_left={no_left} no_right={no_right})"
			),
			PreviewSubject::ConnectingHall => "preview: connecting-hall (one kink)".into(),
			PreviewSubject::ArcFloor {
				radius,
				storey_height,
				floor,
				ceiling,
				ref openings,
			} => format!(
				"preview: arc-floor (r={radius:.1} h={storey_height:.1} floor={floor} ceil={ceiling} openings={})",
				openings.len()
			),
			PreviewSubject::ArcTower {
				radius,
				floor_count,
				storey_height,
				floor_hole,
				no_base_floor,
				no_ceiling,
			} => format!(
				"preview: arc-tower (r={radius:.1} floors={floor_count} h={storey_height:.1} hole={floor_hole:.2} no_base={no_base_floor} no_ceil={no_ceiling})"
			),
			PreviewSubject::ConnectingShells => {
				"preview: connecting-shells (arc-tower + hall + trazaloid)".into()
			}
			PreviewSubject::Trazaloid {
				footprint_x,
				footprint_z,
				ridge_x,
				ridge_z,
				lower_height,
				upper_height,
				band_vertical_offset,
				waist_horizontal_offset,
				ref openings,
				floor,
				no_ceiling,
				face_post_count,
			} => format!(
				"preview: trazaloid (foot={footprint_x:.1}x{footprint_z:.1} ridge={ridge_x:.1}x{ridge_z:.1} h={lower_height:.1}+{upper_height:.1} gap={band_vertical_offset:.2} inset={waist_horizontal_offset:.2} openings={} floor={floor} ceil=!{no_ceiling} posts={face_post_count})",
				openings.len()
			),
			PreviewSubject::PitchedRectangularRoof {
				footprint_x,
				footprint_z,
				ridge_height,
				eave_height,
				ridge_inset,
				gables,
				no_walls,
				no_hips,
				ref openings,
			} => format!(
				"preview: pitched-rectangular-roof (foot={footprint_x:.1}x{footprint_z:.1} ridge_h={ridge_height:.1} eave_h={eave_height:.1} inset={ridge_inset:.1} gables={gables} walls={} hips={} openings={})",
				!no_walls,
				!no_hips,
				openings.len()
			),
			PreviewSubject::RectFloor {
				footprint_x,
				footprint_z,
				storey_height,
				ref openings,
				floor,
				ceiling,
			} => format!(
				"preview: rect-floor (foot={footprint_x:.1}x{footprint_z:.1} h={storey_height:.1} openings={} floor={floor} ceil={ceiling})",
				openings.len()
			),
			PreviewSubject::RoundedRectFloor {
				footprint_x,
				footprint_z,
				storey_height,
				corner_radius,
				corner_segments,
				ref openings,
				floor,
				ceiling,
			} => format!(
				"preview: rounded-rect-floor (foot={footprint_x:.1}x{footprint_z:.1} h={storey_height:.1} r={corner_radius:.2} segs={corner_segments} openings={} floor={floor} ceil={ceiling})",
				openings.len()
			),
			PreviewSubject::IFloor {
				central_x,
				central_z,
				storey_height,
				ref openings,
				floor,
				ceiling,
				..
			} => format!(
				"preview: i-floor (central={central_x:.1}x{central_z:.1} h={storey_height:.1} openings={} floor={floor} ceil={ceiling})",
				openings.len()
			),
			PreviewSubject::Rectangle {
				origin,
				edge,
				height,
				thickness,
				roll,
			} => {
				format!(
					"preview: rectangle (origin={origin:?} edge={edge:?} height={height:.3} thickness={thickness:.3} roll={roll:.3})"
				)
			}
			PreviewSubject::ClippedRectangle {
				origin,
				edge,
				height,
				thickness,
				roll,
				left,
				right,
				bottom,
				top,
			} => format!(
				"preview: clipped-rectangle (origin={origin:?} edge={edge:?} height={height:.3} thickness={thickness:.3} roll={roll:.3} inset=[{left:.2},{right:.2},{bottom:.2},{top:.2}])"
			),
			PreviewSubject::ClippedRectangularStrip {
				inset,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: clipped-rectangular-strip (inset={inset:.2} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::FittedRectangle { a0, a1, b0, b1 } => {
				format!("preview: fitted-rectangle (a0={a0:?} a1={a1:?} b0={b0:?} b1={b1:?})")
			}
			PreviewSubject::ClippedFittedRectangle {
				a0,
				a1,
				b0,
				b1,
				left,
				right,
				bottom,
				top,
			} => format!(
				"preview: clipped-fitted-rectangle (a0={a0:?} a1={a1:?} b0={b0:?} b1={b1:?} inset=[{left:.2},{right:.2},{bottom:.2},{top:.2}])"
			),
			PreviewSubject::ClippedFittedRectangularStrip {
				inset,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: clipped-fitted-rectangular-strip (inset={inset:.2} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::RectangularNTube {
				inset,
				min_dihedral,
				no_joint,
				ref omit_faces,
			} => format!(
				"preview: rectangular-n-tube (inset={inset:.2} min_dihedral={min_dihedral:.3} no_joint={no_joint} omit_faces={omit_faces:?})"
			),
			PreviewSubject::ApproximatedCircle {
				center,
				radius,
				segments,
				clip,
			} => format!(
				"preview: approximated-circle (c={center:?} r={radius:.2} n={segments} clip={clip:?})"
			),
			PreviewSubject::ArcSweep {
				radius,
				height,
				sweep_degrees,
				start_yaw_deg,
			} => format!(
				"preview: arc-sweep (r={radius:.2} h={height:.2} sweep={sweep_degrees:.1} yaw0={start_yaw_deg:.1})"
			),
			PreviewSubject::ClippedArcSweep {
				radius,
				height,
				sweep_degrees,
				start_yaw_deg,
			} => format!(
				"preview: clipped-arc-sweep (r={radius:.2} h={height:.2} sweep={sweep_degrees:.1} yaw0={start_yaw_deg:.1})"
			),
			PreviewSubject::QuadPanel {
				a0,
				a1,
				b0,
				b1,
				t_a0,
				t_a1,
				t_b0,
				t_b1,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: quad-panel (a0={a0:?} a1={a1:?} b0={b0:?} b1={b1:?} t=[{t_a0:.2},{t_a1:.2},{t_b0:.2},{t_b1:.2}] min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::PanelComplex {
				ref mesh,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: panel-complex (mesh={mesh:?} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::QuadPanelComplex {
				ref mesh,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: quad-panel-complex (mesh={mesh:?} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::RuledPitch {
				min_dihedral,
				no_joint,
			} => format!(
				"preview: ruled-pitch (min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::Polyline => "preview: partition polyline (L)".into(),
			PreviewSubject::NoisyRectangularWall {
				distance,
				step_len,
				allowed_angles,
				path_noise,
			} => format!(
				"preview: noisy-rectangular-wall (d={distance:.1} step=[{:.2},{:.2}] ang=({:.2},{:.2},{:.2}) seed={})",
				step_len.min, step_len.max,
				allowed_angles.x, allowed_angles.y, allowed_angles.z, path_noise.seed
			),
			PreviewSubject::WizardsTower { noise } => {
				format!("preview: wizards-tower (noise={noise:.2})")
			}
			PreviewSubject::StackedRings {
				floor_count,
				floor_height,
				radius,
			} => format!(
				"preview: stacked-rings (n={floor_count} h={floor_height:.2} r={radius:.2})"
			),
			PreviewSubject::Bedroom {
				extent,
				noise,
				spaciousness,
				occupancy,
				door,
			} => {
				format!(
					"preview: bedroom (extent={:.2},{:.2},{:.2} noise={noise:.2} space={spaciousness:.2} occ={occupancy:.2} door={door})",
					extent.x, extent.y, extent.z
				)
			}
		}
	}

	fn subject_bounds(&self) -> Aabb3d {
		match &self.subject {
			PreviewSubject::StackedRings { radius, floor_count, floor_height } => {
				let r = (*radius).max(1e-4);
				let h = (*floor_count as f32) * (*floor_height).max(1e-4);
				Aabb3d::from_min_max(Vec3::new(-r, 0.0, -r), Vec3::new(r, h, r))
			}
			PreviewSubject::WizardsTower { .. } => {
				Aabb3d::from_min_max(Vec3::new(-4.0, 0.0, -4.0), Vec3::new(4.0, 3.0, 4.0))
			}
			PreviewSubject::Bedroom { extent, .. } => Aabb3d::from_min_max(Vec3::ZERO, *extent),
			PreviewSubject::Pitch { rise, run, length, left, right, .. } => {
				let left_w = left.map(|b| b.abs()).unwrap_or(0.0);
				let right_w = right.map(|b| b.abs()).unwrap_or(0.0);
				let len = length.unwrap_or(0.0);
				let x_max = (left_w + len + right_w).max(1e-4);
				let run = (*run).max(1e-4);
				let rise = (*rise).max(0.0);
				Aabb3d::from_min_max(Vec3::new(0.0, -0.2, -run), Vec3::new(x_max, rise + 0.2, 0.0))
			}
			PreviewSubject::TessellatedTriangle { a, b, c, .. } => {
				let min_x = a.x.min(b.x).min(c.x) - 0.2;
				let max_x = a.x.max(b.x).max(c.x) + 0.2;
				let min_z = a.y.min(b.y).min(c.y) - 0.2;
				let max_z = a.y.max(b.y).max(c.y) + 0.2;
				Aabb3d::from_min_max(Vec3::new(min_x, -0.2, min_z), Vec3::new(max_x, 0.2, max_z))
			}
			PreviewSubject::TessellatedTriangle3d { a, b, c } => {
				let min = a.min(*b).min(*c) - Vec3::splat(0.2);
				let max = a.max(*b).max(*c) + Vec3::splat(0.2);
				Aabb3d::from_min_max(min, max)
			}
			PreviewSubject::QuadPanel { a0, a1, b0, b1, .. }
			| PreviewSubject::FittedRectangle { a0, a1, b0, b1, .. }
			| PreviewSubject::ClippedFittedRectangle { a0, a1, b0, b1, .. } => {
				let min = a0.min(*a1).min(*b0).min(*b1) - Vec3::splat(0.2);
				let max = a0.max(*a1).max(*b0).max(*b1) + Vec3::splat(0.2);
				Aabb3d::from_min_max(min, max)
			}
			PreviewSubject::Rectangle {
				origin,
				edge,
				height,
				..
			}
			| PreviewSubject::ClippedRectangle {
				origin,
				edge,
				height,
				..
			} => {
				let end = *origin + *edge;
				let up = Vec3::Y * (*height);
				let min = origin.min(end).min(*origin + up).min(end + up) - Vec3::splat(0.2);
				let max = origin.max(end).max(*origin + up).max(end + up) + Vec3::splat(0.2);
				Aabb3d::from_min_max(min, max)
			}
			PreviewSubject::ClippedFittedRectangularStrip { .. } => Aabb3d::from_min_max(
				Vec3::new(-0.5, -0.5, -0.5),
				Vec3::new(3.5, 3.0, 7.0),
			),
			PreviewSubject::RectangularNTube { .. } => Aabb3d::from_min_max(
				Vec3::new(-2.0, -0.5, -0.5),
				Vec3::new(2.0, 2.5, 7.0),
			),
			PreviewSubject::Polyline => {
				Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(4.0, 3.0, 4.0))
			}
			PreviewSubject::NoisyRectangularWall { distance, .. } => {
				let r = (*distance).max(4.0);
				Aabb3d::from_min_max(Vec3::new(-r, -r * 0.5, -r), Vec3::new(r, r * 0.5 + 3.0, r))
			}
			PreviewSubject::Tube { .. } => {
				// Demo polyline bends +X/+Y along +Z with ~1.3 half-widths / ~2.4 height.
				Aabb3d::from_min_max(
					Vec3::new(-2.0, -0.5, -0.5),
					Vec3::new(7.0, 4.0, 9.0),
				)
			}
			PreviewSubject::ConnectingHall => {
				Aabb3d::from_min_max(Vec3::new(-5.0, -0.5, -5.0), Vec3::new(5.0, 4.0, 5.0))
			}
			PreviewSubject::ArcFloor {
				radius,
				storey_height,
				..
			} => {
				let r = radius.max(1e-4) + 0.5;
				let h = storey_height.max(1e-4) + 0.5;
				Aabb3d::from_min_max(Vec3::new(-r, -0.2, -r), Vec3::new(r, h, r))
			}
			PreviewSubject::ArcTower {
				radius,
				floor_count,
				storey_height,
				..
			} => {
				let r = radius.max(1e-4) + 0.5;
				let h = (*floor_count as f32) * storey_height.max(1e-4) + 0.5;
				Aabb3d::from_min_max(Vec3::new(-r, -0.2, -r), Vec3::new(r, h, r))
			}
			PreviewSubject::ConnectingShells => Aabb3d::from_min_max(
				Vec3::new(-19.0, -0.2, -5.0),
				Vec3::new(5.0, 10.0, 5.0),
			),
			PreviewSubject::Trazaloid {
				footprint_x,
				footprint_z,
				lower_height,
				upper_height,
				band_vertical_offset,
				..
			} => {
				let hx = footprint_x.max(1e-4) * 0.5 + 0.5;
				let hz = footprint_z.max(1e-4) * 0.5 + 0.5;
				let h = lower_height + band_vertical_offset + upper_height + 0.5;
				Aabb3d::from_min_max(Vec3::new(-hx, -0.2, -hz), Vec3::new(hx, h, hz))
			}
			PreviewSubject::PitchedRectangularRoof {
				footprint_x,
				footprint_z,
				ridge_height,
				..
			} => {
				let hx = footprint_x.max(1e-4) * 0.5 + 0.5;
				let hz = footprint_z.max(1e-4) * 0.5 + 0.5;
				let h = ridge_height.max(1e-4) + 0.5;
				Aabb3d::from_min_max(Vec3::new(-hx, -0.2, -hz), Vec3::new(hx, h, hz))
			}
			PreviewSubject::RectFloor {
				footprint_x,
				footprint_z,
				storey_height,
				..
			}
			| PreviewSubject::RoundedRectFloor {
				footprint_x,
				footprint_z,
				storey_height,
				..
			} => {
				let hx = footprint_x.max(1e-4) * 0.5 + 0.5;
				let hz = footprint_z.max(1e-4) * 0.5 + 0.5;
				let h = storey_height.max(1e-4) + 0.5;
				Aabb3d::from_min_max(Vec3::new(-hx, -0.2, -hz), Vec3::new(hx, h, hz))
			}
			PreviewSubject::IFloor {
				central_x,
				central_z,
				storey_height,
				top_left,
				top_right,
				bottom_left,
				bottom_right,
				..
			} => {
				let half_w = central_x.max(1e-4) * 0.5;
				let half_d = central_z.max(1e-4) * 0.5;
				let left = top_left.unwrap_or(0.0).max(bottom_left.unwrap_or(0.0));
				let right = top_right.unwrap_or(0.0).max(bottom_right.unwrap_or(0.0));
				let flange_t = central_x.max(1e-4);
				let hx = half_w + left.max(right) + 0.5;
				let hz = half_d
					+ if top_left.is_some() || top_right.is_some() {
						flange_t
					} else {
						0.0
					}
					+ if bottom_left.is_some() || bottom_right.is_some() {
						flange_t
					} else {
						0.0
					}
					+ 0.5;
				let h = storey_height.max(1e-4) + 0.5;
				Aabb3d::from_min_max(Vec3::new(-hx, -0.2, -hz), Vec3::new(hx, h, hz))
			}
			_ => Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE),
		}
	}
}

/// Authored preview payload kept across LOD flips (stable noise / geometry).
#[derive(Resource, Default)]
pub struct CachedPreview {
	key: Option<(PreviewSubject, Transform)>,
	wizards_tower: Option<WizardsTower>,
	stacked_rings: Option<StackedRings>,
	bedroom: Option<Bedroom>,
	noisy_wall: Option<NoisyRectangularWall>,
}

impl CachedPreview {
	fn rebuild_if_needed(&mut self, config: &PreviewConfig) {
		let key = (config.subject.clone(), config.transform);
		if self.key.as_ref() == Some(&key) {
			return;
		}
		self.key = Some(key);
		self.wizards_tower = None;
		self.stacked_rings = None;
		self.bedroom = None;
		self.noisy_wall = None;
		match &config.subject {
			PreviewSubject::WizardsTower { noise } => {
				let footprint = CellConstraints::cell_owned(Aabb3d::from_min_max(
					Vec3::new(-4.0, 0.0, -4.0),
					Vec3::new(4.0, 3.0, 4.0),
				));
				self.wizards_tower = Some(WizardsTower::new(&footprint, *noise));
			}
			PreviewSubject::StackedRings { floor_count, floor_height, radius } => {
				self.stacked_rings = Some(StackedRings::new(*floor_count, *floor_height, *radius));
			}
			PreviewSubject::Bedroom { extent, noise, spaciousness, occupancy, door } => {
				let mut room =
					CellConstraints::cell_owned(Aabb3d::from_min_max(Vec3::ZERO, *extent));
				if *door {
					room.circulation.front = Some(CirculationEntry(vec![(
						Aabb2d { min: Vec2::new(0.35, 0.0), max: Vec2::new(0.65, 0.9) },
						vec![CirculationRequestStatus::Required],
					)]));
				}
				self.bedroom = Some(Bedroom::with_fill(
					room,
					*noise,
					BedroomFillParams { spaciousness: *spaciousness, occupancy: *occupancy },
				));
			}
			PreviewSubject::NoisyRectangularWall {
				distance,
				step_len,
				allowed_angles,
				path_noise,
			} => {
				self.noisy_wall = Some(NoisyRectangularWall::new(NoisyRectangularWallParams {
					distance: *distance,
					step_len: *step_len,
					allowed_angles: *allowed_angles,
					path_noise: *path_noise,
					must_assign: vec![MustAssignPortal::at(0.5, Portal::Window)],
					optional_portals: (0, 0),
					..NoisyRectangularWallParams::default()
				}));
			}
			_ => {}
		}
	}
}

/// Spawn preview when the subject changes. LOD flips update host levels in-place
/// ([`lod::LodFinePassPlugin`] + domain fine-phase systems).
pub fn present_preview_lod(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	lod_state: Res<LodViewerState>,
	mut cache: ResMut<CachedPreview>,
	roots: Query<Entity, With<PreviewRoot>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut last_subject: Local<Option<(PreviewSubject, Transform)>>,
) {
	let subject_key = (config.subject.clone(), config.transform);
	let subject_changed = last_subject.as_ref() != Some(&subject_key);
	let has_root = roots.iter().next().is_some();

	if matches!(config.subject, PreviewSubject::None) {
		if subject_changed || has_root {
			for entity in &roots {
				commands.entity(entity).despawn();
			}
			*last_subject = Some(subject_key);
			cache.key = None;
			cache.wizards_tower = None;
			cache.stacked_rings = None;
			cache.bedroom = None;
			cache.noisy_wall = None;
		}
		return;
	}

	cache.rebuild_if_needed(&config);

	if !subject_changed && has_root {
		return;
	}

	for entity in &roots {
		commands.entity(entity).despawn();
	}
	*last_subject = Some(subject_key);

	let bounds = config.subject_bounds();
	let lod_ref = lod_state.lod_ref(&bounds);

	let transform = config.transform;
	match &config.subject {
		PreviewSubject::None => {}
		PreviewSubject::Linear => {
			spawn_preview(&mut commands, transform, RoughStoneworkLinear.scene_with_lod(&lod_ref));
		}
		PreviewSubject::Arc90 => {
			spawn_preview(&mut commands, transform, RoughStonework90.scene_with_lod(&lod_ref));
		}
		PreviewSubject::Arc180 => {
			spawn_preview(&mut commands, transform, RoughStonework180.scene_with_lod(&lod_ref));
		}
		PreviewSubject::Slice90 => {
			spawn_preview(&mut commands, transform, RoughStoneworkSlice90.scene_with_lod(&lod_ref));
		}
		PreviewSubject::Pitch { rise, run, length, tile_width, left, right } => {
			let mut pitch = Pitch::new(*rise, *run, *tile_width);
			if let Some(len) = length {
				pitch = pitch.with_length(*len);
			}
			if let Some(base) = left {
				pitch = pitch.with_left(*base);
			}
			if let Some(base) = right {
				pitch = pitch.with_right(*base);
			}
			let roof = RoofNode::shepherds_thatch(RoofGeometry::pitch(pitch), Placement::IDENTITY);
			spawn_preview(&mut commands, transform, roof.scene_with_lod(&lod_ref));
		}
		PreviewSubject::TessellatedTriangle { a, b, c } => {
			let panel = PanelNode::rough_stone(
				PanelGeometry::tessellated_triangle(TessellatedTriangle::new(*a, *b, *c)),
				Placement::IDENTITY,
			);
			spawn_preview(&mut commands, transform, panel.scene_with_lod(&lod_ref));
		}
		PreviewSubject::TessellatedTriangle3d { a, b, c } => {
			let panel = TessellatedTrianglePanel::rough_stone(*a, *b, *c);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(panel).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::ClippedTessellatedTriangle {
			a,
			b,
			c,
			clip,
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let complex = ClippedTessellatedTriangle::rough_stone(*a, *b, *c, clip.clone())
				.with_joint_policy(policy)
				.into_complex();
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(complex).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::ClippedQuadPanel {
			a0,
			a1,
			b0,
			b1,
			clip,
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let complex = ClippedQuadPanel::rough_stone(*a0, *a1, *b0, *b1, clip.clone())
				.with_joint_policy(policy)
				.into_complex();
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(complex).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::ClippedRuledStrip {
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let rail_a = [
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(0.0, 0.0, 2.0),
				Vec3::new(0.0, 0.0, 4.0),
				Vec3::new(0.0, 0.0, 6.0),
			];
			let rail_b = [
				Vec3::new(2.5, 0.8, 0.0),
				Vec3::new(2.2, 1.0, 2.0),
				Vec3::new(2.8, 0.7, 4.0),
				Vec3::new(2.4, 1.1, 6.0),
			];
			let mid_clip = vec![
				Vec3::new(0.5, 0.2, 2.4),
				Vec3::new(1.8, 0.5, 2.4),
				Vec3::new(1.8, 0.5, 3.4),
				Vec3::new(0.5, 0.2, 3.4),
			];
			let strip = ClippedRuledStrip::from_lines(
				richmond_building_components::panels::PanelStyle::ShepherdsThatch,
				rail_a,
				rail_b,
				[None, Some(mid_clip), None],
			)
			.with_joint_policy(policy);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(strip).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::Tube {
			min_dihedral,
			no_joint,
			no_floor,
			no_ceiling,
			no_left,
			no_right,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let mut faces = TubeFaces::ALL;
			if *no_floor {
				faces = faces.without_floor();
			}
			if *no_ceiling {
				faces = faces.without_ceiling();
			}
			if *no_left {
				faces = faces.without_left();
			}
			if *no_right {
				faces = faces.without_right();
			}
			// Level start, then plan bend + pitch; slight roll on the kink station.
			let nodes = [
				TubeCrossSectionNode::new(
					Vec3::new(0.0, 0.0, 0.0),
					1.2,
					1.2,
					2.2,
					1.0,
					1.0,
				),
				TubeCrossSectionNode::new(
					Vec3::new(0.0, 0.0, 3.0),
					1.2,
					1.2,
					2.2,
					1.0,
					1.0,
				),
				TubeCrossSectionNode::new(
					Vec3::new(2.0, 0.5, 6.0),
					1.3,
					1.1,
					2.4,
					1.1,
					0.9,
				)
				.with_roll(0.15),
				TubeCrossSectionNode::new(
					Vec3::new(5.0, 1.0, 8.0),
					1.2,
					1.2,
					2.2,
					1.0,
					1.0,
				),
			];
			// Opening on the left wall of the middle bay (between stations 1–2).
			let left_clip = vec![
				Vec3::new(-1.0, 0.4, 3.6),
				Vec3::new(-1.0, 1.6, 3.6),
				Vec3::new(-0.2, 1.6, 5.0),
				Vec3::new(-0.2, 0.4, 5.0),
			];
			let tube = Tube::from_nodes_with_clips(
				richmond_building_components::panels::PanelStyle::RoughStonework,
				nodes,
				std::iter::empty(),
				std::iter::empty(),
				[None, Some(left_clip), None],
				std::iter::empty(),
			)
			.with_joint_policy(policy)
			.with_faces(faces);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(tube).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::ConnectingHall => {
			let (end_a, end_b) = connecting_hall_demo_endpoints();
			let hall = ConnectingHall::rough_stone(end_a, end_b);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(hall).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::ArcFloor {
			radius,
			storey_height,
			floor,
			ceiling,
			openings,
		} => {
			let floor_shell = ArcFloor::new(ArcFloorParams {
				center_xz: Vec3::ZERO,
				radius: *radius,
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor {
					ArcFloorSlab::Solid
				} else {
					ArcFloorSlab::None
				},
				ceiling: if *ceiling {
					ArcFloorSlab::Solid
				} else {
					ArcFloorSlab::None
				},
				..ArcFloorParams::default()
			});
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(floor_shell).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::ArcTower {
			radius,
			floor_count,
			storey_height,
			floor_hole,
			no_base_floor,
			no_ceiling,
		} => {
			let mut openings = Openings::new();
			for (id, t, label) in [
				("door", 0.0, OpeningLabel::Passage),       // +X
				("window_w", 0.25, OpeningLabel::Aperture), // −Z
				("window_e", 0.5, OpeningLabel::Aperture),  // −X
				("window_n", 0.75, OpeningLabel::Aperture), // +Z
			] {
				let (id, opening) = ArcFloor::plan_opening_at_t(
					id,
					label,
					Vec3::ZERO,
					*radius,
					*storey_height,
					t,
				);
				openings.insert(id, opening);
			}
			let tower = ArcTower::new(ArcTowerParams {
				center_xz: Vec3::ZERO,
				radius: *radius,
				floor_count: *floor_count,
				storey_height: *storey_height,
				openings,
				base_floor: if *no_base_floor {
					ArcFloorSlab::None
				} else {
					ArcFloorSlab::Solid
				},
				intermediate_floors: ArcFloorSlab::Solid,
				top_ceiling: if *no_ceiling {
					ArcFloorSlab::None
				} else {
					ArcFloorSlab::Solid
				},
				intermediate_floor_hole: *floor_hole,
				..ArcTowerParams::default()
			});
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(tower).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::ConnectingShells => {
			let demo = ConnectingShells::new();
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(demo).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::Trazaloid {
			footprint_x,
			footprint_z,
			ridge_x,
			ridge_z,
			lower_height,
			upper_height,
			band_vertical_offset,
			waist_horizontal_offset,
			openings,
			floor,
			no_ceiling,
			face_post_count,
		} => {
			let shell = Trazaloid::new(TrazaloidParams {
				footprint: Vec2::new(*footprint_x, *footprint_z),
				ridge: Vec2::new(*ridge_x, *ridge_z),
				lower_height: *lower_height,
				upper_height: *upper_height,
				band_vertical_offset: *band_vertical_offset,
				waist_horizontal_offset: *waist_horizontal_offset,
				openings: openings_from_preview(openings),
				floor: if *floor {
					TrazaloidSlab::Solid
				} else {
					TrazaloidSlab::None
				},
				ceiling: if *no_ceiling {
					TrazaloidSlab::None
				} else {
					TrazaloidSlab::Solid
				},
				face_post_count: *face_post_count,
				..TrazaloidParams::default()
			});
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(shell).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::PitchedRectangularRoof {
			footprint_x,
			footprint_z,
			ridge_height,
			eave_height,
			ridge_inset,
			gables,
			no_walls,
			no_hips,
			openings,
		} => {
			let shell = pitched_roof_from_preview(
				*footprint_x,
				*footprint_z,
				*ridge_height,
				*eave_height,
				*ridge_inset,
				*gables,
				*no_walls,
				*no_hips,
				openings,
			);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(shell).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::RectFloor {
			footprint_x,
			footprint_z,
			storey_height,
			openings,
			floor,
			ceiling,
		} => {
			let shell = RectFloor::new(RectFloorParams {
				center_xz: Vec3::ZERO,
				footprint: Vec2::new(*footprint_x, *footprint_z),
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor {
					RectFloorSlab::Solid
				} else {
					RectFloorSlab::None
				},
				ceiling: if *ceiling {
					RectFloorSlab::Solid
				} else {
					RectFloorSlab::None
				},
				..RectFloorParams::default()
			});
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(shell).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::RoundedRectFloor {
			footprint_x,
			footprint_z,
			storey_height,
			corner_radius,
			corner_segments,
			openings,
			floor,
			ceiling,
		} => {
			let shell = RoundedRectFloor::new(RoundedRectFloorParams {
				center_xz: Vec3::ZERO,
				footprint: Vec2::new(*footprint_x, *footprint_z),
				storey_height: *storey_height,
				corner_radius: *corner_radius,
				corner_segments: *corner_segments,
				openings: openings_from_preview(openings),
				floor: if *floor {
					RoundedRectFloorSlab::Solid
				} else {
					RoundedRectFloorSlab::None
				},
				ceiling: if *ceiling {
					RoundedRectFloorSlab::Solid
				} else {
					RoundedRectFloorSlab::None
				},
				..RoundedRectFloorParams::default()
			});
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(shell).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::IFloor {
			central_x,
			central_z,
			storey_height,
			top_left,
			top_right,
			bottom_left,
			bottom_right,
			openings,
			floor,
			ceiling,
		} => {
			let shell = IFloor::new(IFloorParams {
				center_xz: Vec3::ZERO,
				top_left_length: *top_left,
				top_right_length: *top_right,
				central_rectangle: Vec2::new(*central_x, *central_z),
				bottom_left_length: *bottom_left,
				bottom_right_length: *bottom_right,
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor {
					IFloorSlab::Solid
				} else {
					IFloorSlab::None
				},
				ceiling: if *ceiling {
					IFloorSlab::Solid
				} else {
					IFloorSlab::None
				},
				..IFloorParams::default()
			});
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(shell).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::Rectangle {
			origin,
			edge,
			height,
			thickness,
			roll,
		} => {
			let rect = Rectangle::rough_stone(*origin, *edge, *height, *thickness, *roll);
			let corners = [
				rect.oriented.a0,
				rect.oriented.a1,
				rect.oriented.b0,
				rect.oriented.b1,
			];
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(rect).scene_with_lod(&lod_ref),
			);
			spawn_rectangle_debug_balls(
				&mut commands,
				&mut meshes,
				&mut materials,
				transform,
				corners,
			);
		}
		PreviewSubject::ClippedRectangle {
			origin,
			edge,
			height,
			thickness,
			roll,
			left,
			right,
			bottom,
			top,
		} => {
			let rect = ClippedRectangle::rough_stone(
				*origin,
				*edge,
				*height,
				*thickness,
				*roll,
				RectInset::new(*left, *right, *bottom, *top),
			);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(rect).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::ClippedRectangularStrip {
			inset,
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			// Plan-turn fold so bay creases exceed the default dihedral threshold.
			let nodes = [
				RectangularStripNode::new(Vec3::new(0.0, 0.0, 0.0), 2.5, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(0.0, 0.0, 2.0), 2.5, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(2.0, 0.0, 2.0), 2.5, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(2.0, 0.0, 4.0), 2.5, 0.75, 0.0),
			];
			let strip = ClippedRectangularStrip::from_nodes(
				richmond_building_components::panels::PanelStyle::RoughStonework,
				nodes,
				[None, Some(RectInset::uniform(*inset)), None],
			)
			.with_joint_policy(policy);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(strip).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::FittedRectangle { a0, a1, b0, b1 } => {
			let rect = FittedRectangle::rough_stone(*a0, *a1, *b0, *b1);
			let corners = [
				rect.fitted.a0,
				rect.fitted.a1,
				rect.fitted.b0,
				rect.fitted.b1,
			];
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(rect).scene_with_lod(&lod_ref),
			);
			spawn_rectangle_debug_balls(
				&mut commands,
				&mut meshes,
				&mut materials,
				transform,
				corners,
			);
		}
		PreviewSubject::ClippedFittedRectangle {
			a0,
			a1,
			b0,
			b1,
			left,
			right,
			bottom,
			top,
		} => {
			let rect = ClippedFittedRectangle::rough_stone(
				*a0,
				*a1,
				*b0,
				*b1,
				RectInset::new(*left, *right, *bottom, *top),
			);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(rect).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::ClippedFittedRectangularStrip {
			inset,
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			// Folded two-rail strip (a along path, b offset; mid bay tips up).
			let rail_a = [
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(0.0, 0.0, 2.0),
				Vec3::new(0.0, 0.0, 4.0),
				Vec3::new(0.0, 0.0, 6.0),
			];
			let rail_b = [
				Vec3::new(2.5, 0.0, 0.0),
				Vec3::new(2.5, 0.0, 2.0),
				Vec3::new(2.5, 1.2, 4.0),
				Vec3::new(2.5, 1.2, 6.0),
			];
			let strip = ClippedFittedRectangularStrip::from_lines(
				richmond_building_components::panels::PanelStyle::RoughStonework,
				rail_a,
				rail_b,
				[None, Some(RectInset::uniform(*inset)), None],
			)
			.with_joint_policy(policy);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(strip).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::RectangularNTube {
			inset,
			min_dihedral,
			no_joint,
			omit_faces,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let t = DEFAULT_PANEL_THICKNESS;
			// Floor at y=0, ceiling at y=2 (not centered on the ground plane).
			let station = |z: f32| {
				RectangularNTubeStation::new([
					RectangularNTubeCorner::new(Vec3::new(-1.0, 0.0, z), t),
					RectangularNTubeCorner::new(Vec3::new(1.0, 0.0, z), t),
					RectangularNTubeCorner::new(Vec3::new(1.0, 2.0, z), t),
					RectangularNTubeCorner::new(Vec3::new(-1.0, 2.0, z), t),
				])
			};
			let tube = RectangularNTube::from_stations_with_insets(
				richmond_building_components::panels::PanelStyle::RoughStonework,
				[station(0.0), station(2.0), station(4.0), station(6.0)],
				[
					vec![None, None, None],
					vec![None, Some(RectInset::uniform(*inset)), None],
					vec![None, None, None],
					vec![None, None, None],
				],
			)
			.without_face_edges(omit_faces.iter().copied())
			.with_joint_policy(policy);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(tube).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::ApproximatedCircle {
			center,
			radius,
			segments,
			clip,
		} => {
			let disk = ApproximatedCircle::rough_stone(*center, *radius, *segments, *clip);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(disk).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::ArcSweep {
			radius,
			height,
			sweep_degrees,
			start_yaw_deg,
		} => {
			let sweep = ArcSweep::rough_stone(
				Vec3::ZERO,
				*radius,
				*height,
				*sweep_degrees,
				start_yaw_deg.to_radians(),
			);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(sweep).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::ClippedArcSweep {
			radius,
			height,
			sweep_degrees,
			start_yaw_deg,
		} => {
			let sweep = ClippedArcSweep::rough_stone(
				Vec3::ZERO,
				*radius,
				*height,
				*sweep_degrees,
				start_yaw_deg.to_radians(),
				[(0.2, 0.35), (0.6, 0.72)],
			);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(sweep).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::QuadPanel {
			a0,
			a1,
			b0,
			b1,
			t_a0,
			t_a1,
			t_b0,
			t_b1,
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let complex = QuadPanel::rough_stone(
				PanelPoint::new(*a0, *t_a0),
				PanelPoint::new(*a1, *t_a1),
				PanelPoint::new(*b0, *t_b0),
				PanelPoint::new(*b1, *t_b1),
			)
			.with_joint_policy(policy)
			.into_complex();
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(complex).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::PanelComplex {
			mesh,
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			match mesh.parse::<PanelComplex>() {
				Ok(complex) => {
					let complex = complex.with_joint_policy(policy);
					spawn_preview(
						&mut commands,
						transform,
						ComponentsOnly(complex).scene_with_lod(&lod_ref),
					);
				}
				Err(e) => {
					warn!("panel-complex parse failed: {e}");
				}
			}
		}
		PreviewSubject::QuadPanelComplex {
			mesh,
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			match mesh.parse::<QuadPanelComplex>() {
				Ok(quads) => {
					let complex = quads.with_joint_policy(policy).into_complex();
					spawn_preview(
						&mut commands,
						transform,
						ComponentsOnly(complex).scene_with_lod(&lod_ref),
					);
				}
				Err(e) => {
					warn!("quad-panel-complex parse failed: {e}");
				}
			}
		}
		PreviewSubject::RuledPitch {
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			// Funky 5+5: eave snakes on the ground; ridge wanders higher with a lag,
			// so rafters twist and bays pick up visible crease dihedrals.
			let eave = [
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(0.6, 0.15, 1.4),
				Vec3::new(-0.3, 0.0, 2.8),
				Vec3::new(0.9, 0.25, 4.1),
				Vec3::new(0.2, 0.0, 5.6),
			];
			let ridge = [
				Vec3::new(1.8, 1.6, 0.4),
				Vec3::new(2.6, 2.1, 1.1),
				Vec3::new(1.4, 1.4, 2.5),
				Vec3::new(2.9, 2.4, 3.6),
				Vec3::new(2.1, 1.7, 5.2),
			];
			let complex = RuledPitch::shepherds_thatch()
				.with_lines(eave, ridge)
				.with_joint_policy(policy)
				.into_complex();
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(complex).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::Polyline => {
			let node = PartitionNode::rough_stone(
				Partition::Polyline(
					richmond_building_components::partitions::PolylinePartition::new([
						Vec3::new(0.0, 0.0, 0.0),
						Vec3::new(4.0, 0.0, 0.0),
						Vec3::new(4.0, 0.0, 4.0),
					])
					.with_wall_scale(3.0, 1.0),
				),
				Placement::IDENTITY,
			);
			spawn_preview(&mut commands, transform, node.scene_with_lod(&lod_ref));
		}
		PreviewSubject::NoisyRectangularWall { .. } => {
			if let Some(wall) = cache.noisy_wall.as_ref() {
				spawn_preview(
					&mut commands,
					transform,
					ComponentsOnly(wall).scene_with_lod(&lod_ref),
				);
			}
		}
		PreviewSubject::WizardsTower { .. } => {
			if let Some(tower) = cache.wizards_tower.clone() {
				commands
					.spawn_scene((
						tower.scene_with_lod(&lod_ref),
						bsn! {
							template_value(transform)
							Visibility::default()
						},
					))
					.insert(PreviewRoot)
					.insert(tower);
			}
		}
		PreviewSubject::StackedRings { .. } => {
			if let Some(rings) = cache.stacked_rings.as_ref() {
				spawn_preview(
					&mut commands,
					transform,
					ComponentsOnly(rings).scene_with_lod(&lod_ref),
				);
			}
		}
		PreviewSubject::Bedroom { .. } => {
			if let Some(bedroom) = cache.bedroom.as_ref() {
				spawn_preview(
					&mut commands,
					transform,
					ComponentsOnly(bedroom).scene_with_lod(&lod_ref),
				);
			}
		}
	}
}

fn spawn_preview(commands: &mut Commands, transform: Transform, scene: impl bevy::scene::Scene) {
	commands
		.spawn_scene((
			scene,
			bsn! {
				template_value(transform)
				Visibility::default()
			},
		))
		.insert(PreviewRoot);
}

/// Demo openings: south facing +Z, east facing −X (mild kink near the origin).
pub fn connecting_hall_demo_endpoints() -> (MappedOpening, MappedOpening) {
	let end_a = MappedOpening::new(
		MappedOpeningQuad::new(
			Vec3::new(-1.2, 0.0, -4.0),
			Vec3::new(1.2, 0.0, -4.0),
			Vec3::new(-1.0, 2.4, -4.0),
			Vec3::new(1.0, 2.4, -4.0),
		),
		Vec2::Y,
	);
	// Looking along −X: left = −Z, right = +Z.
	let end_b = MappedOpening::new(
		MappedOpeningQuad::new(
			Vec3::new(4.0, 0.5, -1.2),
			Vec3::new(4.0, 0.5, 1.2),
			Vec3::new(4.0, 2.6, -1.0),
			Vec3::new(4.0, 2.6, 1.0),
		),
		-Vec2::X,
	);
	(end_a, end_b)
}

/// Wireframe plan AABBs (+ mapped contact quads) for `--opening` previews.
///
/// Color key:
/// - cyan / amber: authored plan [`Aabb3d`] voids
/// - lime: mapped outward opening quads (what connectors consume)
/// - orange arrows: mapped XZ orientation
pub fn draw_opening_plan_gizmos(mut gizmos: Gizmos, config: Res<PreviewConfig>) {
	let tf = config.transform;
	let map = |p: Vec3| tf.transform_point(p);
	let cyan = Color::srgb(0.25, 0.95, 1.0);
	let amber = Color::srgb(1.0, 0.75, 0.2);
	let lime = Color::srgb(0.35, 0.95, 0.35);
	let orange = Color::srgb(1.0, 0.55, 0.15);

	match &config.subject {
		PreviewSubject::ArcFloor {
			radius,
			storey_height,
			floor,
			ceiling,
			openings,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let floor_shell = ArcFloor::new(ArcFloorParams {
				center_xz: Vec3::ZERO,
				radius: *radius,
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor {
					ArcFloorSlab::Solid
				} else {
					ArcFloorSlab::None
				},
				ceiling: if *ceiling {
					ArcFloorSlab::Solid
				} else {
					ArcFloorSlab::None
				},
				..ArcFloorParams::default()
			});
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &floor_shell, lime, orange);
		}
		PreviewSubject::Trazaloid {
			footprint_x,
			footprint_z,
			ridge_x,
			ridge_z,
			lower_height,
			upper_height,
			band_vertical_offset,
			waist_horizontal_offset,
			openings,
			floor,
			no_ceiling,
			face_post_count,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let shell = Trazaloid::new(TrazaloidParams {
				footprint: Vec2::new(*footprint_x, *footprint_z),
				ridge: Vec2::new(*ridge_x, *ridge_z),
				lower_height: *lower_height,
				upper_height: *upper_height,
				band_vertical_offset: *band_vertical_offset,
				waist_horizontal_offset: *waist_horizontal_offset,
				openings: openings_from_preview(openings),
				floor: if *floor {
					TrazaloidSlab::Solid
				} else {
					TrazaloidSlab::None
				},
				ceiling: if *no_ceiling {
					TrazaloidSlab::None
				} else {
					TrazaloidSlab::Solid
				},
				face_post_count: *face_post_count,
				..TrazaloidParams::default()
			});
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &shell, lime, orange);
		}
		PreviewSubject::RectFloor {
			footprint_x,
			footprint_z,
			storey_height,
			openings,
			floor,
			ceiling,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let shell = RectFloor::new(RectFloorParams {
				center_xz: Vec3::ZERO,
				footprint: Vec2::new(*footprint_x, *footprint_z),
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor {
					RectFloorSlab::Solid
				} else {
					RectFloorSlab::None
				},
				ceiling: if *ceiling {
					RectFloorSlab::Solid
				} else {
					RectFloorSlab::None
				},
				..RectFloorParams::default()
			});
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &shell, lime, orange);
		}
		PreviewSubject::RoundedRectFloor {
			footprint_x,
			footprint_z,
			storey_height,
			corner_radius,
			corner_segments,
			openings,
			floor,
			ceiling,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let shell = RoundedRectFloor::new(RoundedRectFloorParams {
				center_xz: Vec3::ZERO,
				footprint: Vec2::new(*footprint_x, *footprint_z),
				storey_height: *storey_height,
				corner_radius: *corner_radius,
				corner_segments: *corner_segments,
				openings: openings_from_preview(openings),
				floor: if *floor {
					RoundedRectFloorSlab::Solid
				} else {
					RoundedRectFloorSlab::None
				},
				ceiling: if *ceiling {
					RoundedRectFloorSlab::Solid
				} else {
					RoundedRectFloorSlab::None
				},
				..RoundedRectFloorParams::default()
			});
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &shell, lime, orange);
		}
		PreviewSubject::IFloor {
			central_x,
			central_z,
			storey_height,
			top_left,
			top_right,
			bottom_left,
			bottom_right,
			openings,
			floor,
			ceiling,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let shell = IFloor::new(IFloorParams {
				center_xz: Vec3::ZERO,
				top_left_length: *top_left,
				top_right_length: *top_right,
				central_rectangle: Vec2::new(*central_x, *central_z),
				bottom_left_length: *bottom_left,
				bottom_right_length: *bottom_right,
				storey_height: *storey_height,
				openings: openings_from_preview(openings),
				floor: if *floor {
					IFloorSlab::Solid
				} else {
					IFloorSlab::None
				},
				ceiling: if *ceiling {
					IFloorSlab::Solid
				} else {
					IFloorSlab::None
				},
				..IFloorParams::default()
			});
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &shell, lime, orange);
		}
		PreviewSubject::PitchedRectangularRoof {
			footprint_x,
			footprint_z,
			ridge_height,
			eave_height,
			ridge_inset,
			gables,
			no_walls,
			no_hips,
			openings,
		} => {
			if openings.is_empty() {
				return;
			}
			for (i, opening) in openings.iter().enumerate() {
				let color = if i % 2 == 0 { cyan } else { amber };
				gizmos.aabb_3d(opening.bounds(), tf, color);
			}
			let shell = pitched_roof_from_preview(
				*footprint_x,
				*footprint_z,
				*ridge_height,
				*eave_height,
				*ridge_inset,
				*gables,
				*no_walls,
				*no_hips,
				openings,
			);
			draw_mapped_opening_overlays(&mut gizmos, map, openings, &shell, lime, orange);
		}
		_ => {}
	}
}

/// Rebuild a pitched roof from preview knobs (matches `/show pitched-rectangular-roof`).
fn pitched_roof_from_preview(
	footprint_x: f32,
	footprint_z: f32,
	ridge_height: f32,
	eave_height: f32,
	ridge_inset: f32,
	gables: bool,
	no_walls: bool,
	no_hips: bool,
	openings: &[PreviewOpening],
) -> PitchedRoof {
	let footprint = Vec2::new(footprint_x, footprint_z);
	let mut params = if no_hips && gables && ridge_inset <= 1e-4 {
		PitchedRoofParams::rectangular_gable(footprint, ridge_height, eave_height)
	} else {
		PitchedRoofParams::rectangular_hip(footprint, ridge_height, eave_height, ridge_inset)
	};
	for half in &mut params.halves {
		half.draw_in_wall_line = !no_walls;
		half.draw_in_half_hip = if no_hips {
			(false, false)
		} else {
			(true, true)
		};
		half.draw_in_half_gable_end = if gables {
			(true, true)
		} else {
			(false, false)
		};
	}
	params.openings = openings_from_preview(openings);
	PitchedRoof::new(params)
}

fn draw_mapped_opening_overlays<M: MapsOpenings>(
	gizmos: &mut Gizmos,
	map: impl Fn(Vec3) -> Vec3 + Copy,
	openings: &[PreviewOpening],
	shell: &M,
	lime: Color,
	orange: Color,
) {
	for opening in openings {
		let id = OpeningId::new(opening.id.clone());
		let Some(mapped) = shell.mapped_opening(&id) else {
			continue;
		};
		draw_opening_gizmos(gizmos, map, *mapped, lime);
		let (bl, br, ..) = mapped.endpoint_corners();
		let mid = (bl + br) * 0.5;
		let dir = Vec3::new(mapped.orientation.x, 0.0, mapped.orientation.y).normalize_or_zero();
		gizmos
			.arrow(map(mid), map(mid + dir * 1.25), orange)
			.with_tip_length(0.2);
	}
}

/// Debug overlay for [`PreviewSubject::ConnectingHall`]: opening corners, orientation
/// arrows, path A→mid→B, and station dots.
pub fn draw_connecting_hall_gizmos(mut gizmos: Gizmos, config: Res<PreviewConfig>) {
	if !matches!(config.subject, PreviewSubject::ConnectingHall) {
		return;
	}
	let tf = config.transform;
	let map = |p: Vec3| tf.transform_point(p);

	let (end_a, end_b) = connecting_hall_demo_endpoints();
	let hall = ConnectingHall::rough_stone(end_a, end_b);
	let stations = hall.stations();
	let mid = hall.midpoint();

	let cyan = Color::srgb(0.2, 0.9, 0.95);
	let magenta = Color::srgb(0.95, 0.25, 0.85);
	let lime = Color::srgb(0.35, 0.95, 0.35);
	let orange = Color::srgb(1.0, 0.55, 0.15);
	let yellow = Color::srgb(1.0, 0.9, 0.2);
	let white = Color::srgb(0.95, 0.95, 0.95);

	draw_opening_gizmos(&mut gizmos, map, end_a, cyan);
	draw_opening_gizmos(&mut gizmos, map, end_b, magenta);

	// Path A → mid → B
	let a_c = stations[0].bottom_middle;
	let b_c = stations[2].bottom_middle;
	gizmos.line(map(a_c), map(mid), white);
	gizmos.line(map(mid), map(b_c), white);

	// Station / junction dots
	let r = 0.12;
	gizmos.sphere(Isometry3d::from_translation(map(a_c)), r, lime);
	gizmos.sphere(Isometry3d::from_translation(map(mid)), r * 1.25, yellow);
	gizmos.sphere(Isometry3d::from_translation(map(b_c)), r, orange);

	// Orientation arrows from opening centers (plan facing).
	let arrow_len = 1.5;
	let a_dir = Vec3::new(end_a.orientation.x, 0.0, end_a.orientation.y).normalize_or_zero();
	let b_dir = Vec3::new(end_b.orientation.x, 0.0, end_b.orientation.y).normalize_or_zero();
	gizmos
		.arrow(map(a_c), map(a_c + a_dir * arrow_len), lime)
		.with_tip_length(0.25);
	gizmos
		.arrow(map(b_c), map(b_c + b_dir * arrow_len), orange)
		.with_tip_length(0.25);
}

fn draw_opening_gizmos(
	gizmos: &mut Gizmos,
	map: impl Fn(Vec3) -> Vec3,
	end: MappedOpening,
	color: Color,
) {
	let (bl, br, tl, tr) = end.endpoint_corners();
	let r = 0.08;
	for p in [bl, br, tl, tr] {
		gizmos.sphere(Isometry3d::from_translation(map(p)), r, color);
	}
	// Opening quad wireframe
	gizmos.line(map(bl), map(br), color);
	gizmos.line(map(br), map(tr), color);
	gizmos.line(map(tr), map(tl), color);
	gizmos.line(map(tl), map(bl), color);
}

/// Debug overlay for [`PreviewSubject::ConnectingShells`].
///
/// Color key (tower side):
/// - white: 15° grid after +7.5° clockwise bias (should sit on jambs / edges)
/// - lime: visible door = one segment (`t−30°` → `t−15°`), on-arc pre-widen
/// - orange: opening mid (`t−22.5°`)
/// - cyan: widened tower hall opening
/// - magenta: trazaloid hall opening
pub fn draw_connecting_shells_gizmos(mut gizmos: Gizmos, config: Res<PreviewConfig>) {
	if !matches!(config.subject, PreviewSubject::ConnectingShells) {
		return;
	}
	let tf = config.transform;
	let map = |p: Vec3| tf.transform_point(p);

	let demo = ConnectingShells::new();
	let floor = demo.tower().storey(0).expect("ground storey");
	let connect = OpeningId::new("connect");
	let end_tower_raw = floor
		.mapped_opening(&connect)
		.expect("ground door")
		.clone();
	let (end_tower_wide, end_traz) = demo.hall().endpoints();
	let door_t = 0.5;
	let seg = floor.segment_t();
	let c = floor.params().center_xz;
	let y = c.y + 0.15;

	let white = Color::srgb(0.95, 0.95, 0.95);
	let lime = Color::srgb(0.35, 0.95, 0.35);
	let cyan = Color::srgb(0.2, 0.9, 0.95);
	let magenta = Color::srgb(0.95, 0.25, 0.85);
	let orange = Color::srgb(1.0, 0.55, 0.15);

	for i in 0..24 {
		let t = i as f32 / 24.0;
		let p = floor.ring_point_at(t);
		gizmos.sphere(
			Isometry3d::from_translation(map(Vec3::new(p.x, y, p.z))),
			0.06,
			white,
		);
	}

	// One 15° segment clockwise of portal t (decreasing t).
	let j_lo = floor.ring_point_at(door_t - 2.0 * seg);
	let j_hi = floor.ring_point_at(door_t - seg);
	let j_mid = floor.ring_point_at(door_t - 1.5 * seg);
	for p in [j_lo, j_hi] {
		gizmos.sphere(
			Isometry3d::from_translation(map(Vec3::new(p.x, y, p.z))),
			0.14,
			lime,
		);
	}
	gizmos.line(
		map(Vec3::new(j_lo.x, y, j_lo.z)),
		map(Vec3::new(j_hi.x, y, j_hi.z)),
		lime,
	);
	gizmos.sphere(
		Isometry3d::from_translation(map(Vec3::new(j_mid.x, y, j_mid.z))),
		0.1,
		orange,
	);

	draw_opening_gizmos(&mut gizmos, map, end_tower_raw, lime);
	draw_opening_gizmos(&mut gizmos, map, end_tower_wide, cyan);
	draw_opening_gizmos(&mut gizmos, map, end_traz, magenta);

	let (bl, br, ..) = end_tower_wide.endpoint_corners();
	let mid = (bl + br) * 0.5;
	let dir =
		Vec3::new(end_tower_wide.orientation.x, 0.0, end_tower_wide.orientation.y).normalize_or_zero();
	gizmos
		.arrow(map(mid), map(mid + dir * 2.0), cyan)
		.with_tip_length(0.3);
}

/// Authored bay corners as colored spheres (a0 red, a1 orange, b0 green, b1 cyan).
fn spawn_rectangle_debug_balls(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	parent: Transform,
	corners: [Vec3; 4],
) {
	let mesh = meshes.add(Sphere::new(0.12));
	let colors = [
		Color::srgb(0.95, 0.2, 0.15),
		Color::srgb(0.95, 0.55, 0.1),
		Color::srgb(0.2, 0.85, 0.25),
		Color::srgb(0.15, 0.75, 0.95),
	];
	for (p, color) in corners.into_iter().zip(colors) {
		let material = materials.add(StandardMaterial {
			base_color: color,
			unlit: true,
			..default()
		});
		commands.spawn((
			Mesh3d(mesh.clone()),
			MeshMaterial3d(material),
			parent * Transform::from_translation(p),
			Visibility::default(),
			PreviewRoot,
		));
	}
}
