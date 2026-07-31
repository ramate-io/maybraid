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
pub mod bedroom;
pub mod constraints;
pub mod panel_complex;
pub mod panel_plane;
pub mod quad_panel;
pub mod quad_panel_complex;
pub mod ruled_pitch;
pub mod ruled_strip;
pub mod stacked_rings;
pub mod clipped_tessellated_triangle;
pub mod tessellated_triangle_panel;
pub mod walling;
pub mod wizards_tower;

pub use arc_spire::{
	best_fit_y_bindings, uniform_storey_bindings, ArcSpire, ArcSpireParams, FitTolerance,
};
pub use bedroom::{Bed, Bedroom, BedroomFillParams, Closet, EnsuiteBathroom, Nightstand};
pub use constraints::{
	BoundaryOwnershipEntry, BoundaryOwnershipStatus, BoundaryRegionList, BoundaryThicknessEntry,
	CellBoundaryTable, CellConstraints, CirculationEntry, CirculationRequestStatus, FaceKind,
	JointCoordinate, JointEntry, PreJointSweep, SubsetError,
};
pub use panel_complex::{
	shared_edges, PanelComplex, PanelComplexJointPolicy, PanelComplexValidation, PanelMesh,
	PanelPoint, PanelPointId, PanelQuadMesh, PanelTriangle, ParsePanelComplexError, SharedEdge,
	DEFAULT_PANEL_THICKNESS,
};
pub use quad_panel::QuadPanel;
pub use quad_panel_complex::QuadPanelComplex;
pub use ruled_pitch::RuledPitch;
pub use ruled_strip::RuledStrip;
pub use stacked_rings::{StackedRing, StackedRings};
pub use clipped_tessellated_triangle::ClippedTessellatedTriangle;
pub use tessellated_triangle_panel::TessellatedTrianglePanel;
pub use walling::{
	ArcRegion, ArcWall, ArcWallParams, AssignedPortal, LinearWall, LinearWallParams,
	MustAssignPortal, NoisyPolylineWall, NoisyPolylineWallParams, PolylineWall, PolylineWallParams,
	Portal, WallRegion, Walling,
};
