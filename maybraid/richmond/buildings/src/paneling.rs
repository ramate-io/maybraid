//! Irregular and ruled panel constructions over [`PanelComplex`](panel_complex::PanelComplex).
//!
//! Rectangle strips force each bay to a best-fit ordinary rectangle in the bay’s
//! average plane. Free skew quads live in the ruled / clipped-quad types.

pub mod approximated_circle;
pub mod clipped_quad_panel;
pub mod clipped_rectangular_strip;
pub mod clipped_ruled_strip;
pub mod clipped_tessellated_triangle;
pub mod panel_complex;
pub mod panel_plane;
pub mod quad_panel;
pub mod quad_panel_complex;
pub mod rect_crease;
pub mod rect_fit;
pub mod rectangle;
pub mod rectangular_strip;
pub mod ruled_pitch;
pub mod ruled_strip;
pub mod tessellated_triangle_panel;

pub use approximated_circle::{ApproximatedCircle, DEFAULT_SEGMENTS, MIN_SEGMENTS};
pub use clipped_quad_panel::ClippedQuadPanel;
pub use clipped_rectangular_strip::{ClippedRectangularStrip, ClippedRectangularStripPiece};
pub use clipped_ruled_strip::{ClippedRuledStrip, ClippedStripPiece};
pub use clipped_tessellated_triangle::ClippedTessellatedTriangle;
pub use panel_complex::{
	shared_edges, PanelComplex, PanelComplexJointPolicy, PanelComplexValidation, PanelMesh,
	PanelPoint, PanelPointId, PanelQuadMesh, PanelTriangle, ParsePanelComplexError, SharedEdge,
	DEFAULT_PANEL_THICKNESS,
};
pub use quad_panel::QuadPanel;
pub use quad_panel_complex::QuadPanelComplex;
pub use rect_fit::{fit_rectangle, fit_rectangle_corners, FittedRect, RectInset};
pub use rectangle::{ClippedRectangle, Rectangle};
pub use rectangular_strip::RectangularStrip;
pub use ruled_pitch::RuledPitch;
pub use ruled_strip::RuledStrip;
pub use tessellated_triangle_panel::TessellatedTrianglePanel;
