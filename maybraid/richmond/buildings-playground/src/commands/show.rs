//! `/show` subcommand: preview a partition leaf or authored building.

pub mod arc_180;
pub mod arc_90;
pub mod bedroom;
pub mod linear;
pub mod linear_wall;
pub mod noisy_polyline_wall;
pub mod panel_complex;
pub mod pitch;
pub mod polyline;
pub mod polyline_wall;
pub mod quad_panel;
pub mod quad_panel_complex;
pub mod ruled_pitch;
pub mod slice_90;
pub mod stacked_rings;
pub mod clipped_quad_panel;
pub mod clipped_ruled_strip;
pub mod clipped_tessellated_triangle;
pub mod tessellated_triangle;
pub mod tessellated_triangle_3d;
pub mod transform;
pub mod wizards_tower;

use bevy::prelude::*;
use clap::Subcommand;

pub use transform::ShowTransform;

use crate::preview::PreviewConfig;

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Show {
	/// Straight rough-stonework linear segment (`panels/.../rectangle_001.glb`).
	Linear(linear::Linear),
	/// 90° rough-stonework arc (`arcs/.../arc_90_001.glb`).
	Arc90(arc_90::Arc90),
	/// 180° rough-stonework arc (`arcs/.../arc_180_001.glb`).
	Arc180(arc_180::Arc180),
	/// 90° slice rough-stonework (`arcs/.../arc_90_slice_001.glb`).
	Slice90(slice_90::Slice90),
	/// Shepherd's-thatch pitched face (`rise`/`run`/`length`/`left`/`right`).
	Pitch(pitch::Pitch),
	/// Arbitrary panel-space triangle filled with right-triangle kits.
	TessellatedTriangle(tessellated_triangle::TessellatedTriangle),
	/// World-space triangle → 2D panel tessellation → posed onto the plane.
	TessellatedTriangle3d(tessellated_triangle_3d::TessellatedTriangle3d),
	/// World-space triangle minus a closed clip (may extend outside) → [`PanelComplex`].
	ClippedTessellatedTriangle(clipped_tessellated_triangle::ClippedTessellatedTriangle),
	/// Ruled quad minus a closed clip → [`PanelComplex`].
	ClippedQuadPanel(clipped_quad_panel::ClippedQuadPanel),
	/// Multi-bay ruled strip with optional per-bay clips.
	ClippedRuledStrip(clipped_ruled_strip::ClippedRuledStrip),
	/// Two lines → two tessellated triangles + optional crease `JointNode`.
	QuadPanel(quad_panel::QuadPanel),
	/// Point-id triangle mesh → panels + crease joints (`id=(x,y,z) ... {a,b,c}`).
	PanelComplex(panel_complex::PanelComplex),
	/// Point-id quad mesh → panels + crease joints (`id=(x,y,z) ... {a0,a1,b0,b1}`).
	QuadPanelComplex(quad_panel_complex::QuadPanelComplex),
	/// Equal eave/ridge polylines → ruled quad strip + crease joints.
	RuledPitch(ruled_pitch::RuledPitch),
	/// L-shaped `Partition::polyline` (posed linears + joints).
	Polyline(polyline::Polyline),
	/// Portal-sensitive straight [`richmond_buildings::LinearWall`].
	LinearWall(linear_wall::LinearWall),
	/// Portal-sensitive [`richmond_buildings::PolylineWall`] (L-path + door).
	PolylineWall(polyline_wall::PolylineWall),
	/// Noisy distance-budget path with allowed X/Y/Z turn angles.
	NoisyPolylineWall(noisy_polyline_wall::NoisyPolylineWall),
	/// Full Wizard's Tower (noise-derived floor count).
	WizardsTower(wizards_tower::WizardsTower),
	/// Stacked circular wall rings (validates kit radius/height scaling).
	StackedRings(stacked_rings::StackedRings),
	/// Hierarchical bedroom (closet / bed / nightstand / ensuite placeholders).
	Bedroom(bedroom::Bedroom),
}

impl Show {
	pub fn react(self, commands: &mut Commands) {
		let (subject, transform) = match self {
			Self::Linear(cmd) => cmd.into_preview(),
			Self::Arc90(cmd) => cmd.into_preview(),
			Self::Arc180(cmd) => cmd.into_preview(),
			Self::Slice90(cmd) => cmd.into_preview(),
			Self::Pitch(cmd) => cmd.into_preview(),
			Self::TessellatedTriangle(cmd) => cmd.into_preview(),
			Self::TessellatedTriangle3d(cmd) => cmd.into_preview(),
			Self::ClippedTessellatedTriangle(cmd) => cmd.into_preview(),
			Self::ClippedQuadPanel(cmd) => cmd.into_preview(),
			Self::ClippedRuledStrip(cmd) => cmd.into_preview(),
			Self::QuadPanel(cmd) => cmd.into_preview(),
			Self::PanelComplex(cmd) => cmd.into_preview(),
			Self::QuadPanelComplex(cmd) => cmd.into_preview(),
			Self::RuledPitch(cmd) => cmd.into_preview(),
			Self::Polyline(cmd) => cmd.into_preview(),
			Self::LinearWall(cmd) => cmd.into_preview(),
			Self::PolylineWall(cmd) => cmd.into_preview(),
			Self::NoisyPolylineWall(cmd) => cmd.into_preview(),
			Self::WizardsTower(cmd) => cmd.into_preview(),
			Self::StackedRings(cmd) => cmd.into_preview(),
			Self::Bedroom(cmd) => cmd.into_preview(),
		};
		commands.insert_resource(PreviewConfig { subject, transform });
	}
}
