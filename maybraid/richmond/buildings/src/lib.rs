//! A common constraint representation for building generation.
//!
//! Often, for complicated patterns, authors will use a combination of
//! this representation and direct access to parent types to build
//! the authored type.
//!
//! At the same time, common constructions built only requiring this representation
//! can be reused within authored types.
//!
//! The cells are rectangular prisms, describing authoring bounds.
//! The authored types do not need, however, to author strictly rectangular geometry.

pub mod arc_spire;
pub mod arcs;
pub mod bedroom;
pub mod constraints;
pub mod demos;
pub mod fit;
pub mod openings;
pub mod paneling;
pub mod portals;
pub mod shells;
pub mod stacked_rings;
pub mod storeys;
pub mod usage_areas;
pub mod wall_demo;
pub mod wizards_tower;

// Compatibility module paths (pre-paneling layout).
pub use paneling::clipped_quad_panel;
pub use paneling::clipped_ruled_strip;
pub use paneling::clipped_tessellated_triangle;
pub use paneling::panel_complex;
pub use paneling::panel_plane;
pub use paneling::quad_panel;
pub use paneling::quad_panel_complex;
pub use paneling::ruled_pitch;
pub use paneling::ruled_strip;
pub use paneling::tessellated_triangle_panel;
pub use paneling::tube;

pub use arc_spire::{
	best_fit_y_bindings, uniform_storey_bindings, ArcSpire, ArcSpireParams, FitTolerance,
};
pub use arcs::{portal_ring_wall, ArcSweep, ClippedArcSweep, PortalRingParams, PortalRingWall};
pub use bedroom::{
	Bed, Bedroom, BedroomFillParams, Closet, EnsuiteBathroom, Nightstand, ShellWall,
};
pub use constraints::{
	BoundaryOwnershipEntry, BoundaryOwnershipStatus, BoundaryRegionList, BoundaryThicknessEntry,
	CellBoundaryTable, CellConstraints, CirculationEntry, CirculationRequestStatus, FaceKind,
	JointCoordinate, JointEntry, PreJointSweep, SubsetError,
};
pub use demos::ConnectingShells;
pub use fit::{
	aabb_near_plane, aabb_xz_area, aabb_xz_center, aabb_xz_extent, aabb_xz_near_eq,
	aabb_xz_overlap_area, Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind,
	StackRegion,
};
pub use openings::{
	MappedOpening, MappedOpeningQuad, MappedOpenings, MapsOpenings, Opening, OpeningId,
	OpeningLabel, Openings,
};
pub use paneling::{
	fallback_oriented, fit_rectangle, fit_rectangle_corners, orient_rectangle,
	roll_to_align_height, shared_edges, zero_roll_height_axis, ApproximatedCircle,
	ClippedFittedRectangle, ClippedFittedRectangularStrip, ClippedFittedRectangularStripPiece,
	ClippedQuadPanel, ClippedRectangle, ClippedRectangularStrip, ClippedRectangularStripPiece,
	ClippedRuledStrip, ClippedStripPiece, ClippedTessellatedTriangle, FittedRect, FittedRectangle,
	FittedRectangularStrip, OrientedRect, PanelComplex, PanelComplexJointPolicy,
	PanelComplexValidation, PanelMesh, PanelPoint, PanelPointId, PanelQuadMesh, PanelTriangle,
	ParsePanelComplexError, QuadPanel, QuadPanelComplex, RectInset, Rectangle, RectangularNTube,
	RectangularNTubeCorner, RectangularNTubeStation, RectangularStrip, RectangularStripNode,
	RuledPitch, RuledStrip, SharedEdge, TessellatedTrianglePanel, Tube, TubeCorners,
	TubeCrossSectionNode, TubeFaces, TubeFrame, DEFAULT_PANEL_THICKNESS, DEFAULT_SEGMENTS,
	MIN_SEGMENTS,
};
pub use portals::{
	ArcRegion, AssignedPortal, MustAssignPortal, Portal, PortalFootprint, WallRegion, SLICE_Y_FRAC,
};
pub use shells::{
	ArcFloor, ArcFloorParams, ArcFloorSlab, ArcTower, ArcTowerParams, CircRingFloor,
	CircRingFloorParams, CircRingFloorSlab, ConnectingHall, EndCap, IFloor, IFloorParams,
	IFloorSlab, Overhang, PitchedRoof, PitchedRoofParams, RectFloor, RectFloorParams,
	RectFloorSide, RectFloorSlab, RectRingFloor, RectRingFloorParams, RectRingFloorSide,
	RectRingFloorSlab, RectangularPitchedRoofComplex, RectangularPitchedRoofComplexParams,
	RidgeJunction, RoofHalf, RoundedRectCorner, RoundedRectFloor, RoundedRectFloorParams,
	RoundedRectFloorSide, RoundedRectFloorSlab, Trazaloid, TrazaloidParams, TrazaloidSide,
	TrazaloidSlab, ValleySegment,
};
pub use stacked_rings::{StackedRing, StackedRings};
pub use storeys::les_halles::{
	LesHallesFloorPlan, LesHallesFullStorey, LesHallesParameterized, LesHallesPlacedDoor,
	LesHallesShaftPlacement, LesHallesStallDoor, SCOPE as LES_HALLES_SCOPE,
};
pub use usage_areas::{
	BitesSitdownStall, BitesStall, CommercialStall, CommercialStallInterior,
	CommercialStallParameterized, CommercialStallPlan, CommercialStallStrip,
	CommercialStallStripParameterized, CommercialStallStripPlan, KnickKnackStall, MiniMart,
	PartsStall, PublicRestroom,
};
pub use wall_demo::{NoisyRectangularWall, NoisyRectangularWallParams};
