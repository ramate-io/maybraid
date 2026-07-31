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
pub mod paneling;
pub mod portals;
pub mod stacked_rings;
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

pub use arc_spire::{
	best_fit_y_bindings, uniform_storey_bindings, ArcSpire, ArcSpireParams, FitTolerance,
};
pub use arcs::{
	portal_ring_wall, ArcSweep, ClippedArcSweep, PortalRingParams, PortalRingWall,
};
pub use bedroom::{Bed, Bedroom, BedroomFillParams, Closet, EnsuiteBathroom, Nightstand, ShellWall};
pub use constraints::{
	BoundaryOwnershipEntry, BoundaryOwnershipStatus, BoundaryRegionList, BoundaryThicknessEntry,
	CellBoundaryTable, CellConstraints, CirculationEntry, CirculationRequestStatus, FaceKind,
	JointCoordinate, JointEntry, PreJointSweep, SubsetError,
};
pub use paneling::{
	fit_rectangle, fit_rectangle_corners, shared_edges, ClippedQuadPanel, ClippedRectangle,
	ClippedRectangularStrip, ClippedRectangularStripPiece, ClippedRuledStrip, ClippedStripPiece,
	ClippedTessellatedTriangle, FittedRect, PanelComplex, PanelComplexJointPolicy,
	PanelComplexValidation, PanelMesh, PanelPoint, PanelPointId, PanelQuadMesh, PanelTriangle,
	ParsePanelComplexError, QuadPanel, QuadPanelComplex, RectInset, Rectangle, RectangularStrip,
	RuledPitch, RuledStrip, SharedEdge, TessellatedTrianglePanel, DEFAULT_PANEL_THICKNESS,
};
pub use portals::{
	ArcRegion, AssignedPortal, MustAssignPortal, Portal, PortalFootprint, WallRegion, SLICE_Y_FRAC,
};
pub use stacked_rings::{StackedRing, StackedRings};
pub use wall_demo::{NoisyRectangularWall, NoisyRectangularWallParams};
