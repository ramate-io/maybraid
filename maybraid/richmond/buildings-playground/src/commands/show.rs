//! `/show` subcommand: preview a partition leaf or authored building.

pub mod approximated_circle;
pub mod arc_180;
pub mod arc_90;
pub mod arc_floor;
pub mod arc_sweep;
pub mod arc_tower;
pub mod opening;
pub mod bedroom;
pub mod connecting_shells;
pub mod linear;
pub mod noisy_rectangular_wall;
pub mod panel_complex;
pub mod pitch;
pub mod pitched_rectangular_roof;
pub mod polyline;
pub mod quad_panel;
pub mod quad_panel_complex;
pub mod rectangle;
pub mod rectangular_n_tube;
pub mod ruled_pitch;
pub mod slice_90;
pub mod stacked_rings;
pub mod clipped_arc_sweep;
pub mod clipped_fitted_rectangle;
pub mod clipped_fitted_rectangular_strip;
pub mod clipped_quad_panel;
pub mod clipped_rectangle;
pub mod clipped_rectangular_strip;
pub mod clipped_ruled_strip;
pub mod clipped_tessellated_triangle;
pub mod connecting_hall;
pub mod fitted_rectangle;
pub mod i_floor;
pub mod rect_floor;
pub mod rounded_rect_floor;
pub mod tessellated_triangle;
pub mod tessellated_triangle_3d;
pub mod transform;
pub mod trazaloid;
pub mod tube;
pub mod wizards_tower;

use bevy::prelude::*;
use clap::Subcommand;
use game_commands::ui::GameCommandStatusText;

pub use transform::ShowTransform;

use crate::preview::PreviewConfig;

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Show {
	/// Straight rough-stonework linear segment (`panels/.../rectangle_001_{high,mid,low}_res.glb`).
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
	/// Trapezoid cross-section polyline → four clipped ruled strip faces.
	Tube(tube::Tube),
	/// One-kink tube between two oriented openings.
	ConnectingHall(connecting_hall::ConnectingHall),
	/// One circular storey shell with optional `--opening` plan entries.
	ArcFloor(arc_floor::ArcFloor),
	/// Stacked circular storey shell (explicit openings; no noise).
	ArcTower(arc_tower::ArcTower),
	/// ArcTower joined to a Trazaloid via a ConnectingHall.
	ConnectingShells(connecting_shells::ConnectingShells),
	/// Two-band trapezoidal-pyramid shell with waist reveal and optional `--opening`s.
	Trazaloid(trazaloid::Trazaloid),
	/// Two-half pitched roof (default: rectangular hip over shepherd's thatch).
	PitchedRectangularRoof(pitched_rectangular_roof::PitchedRectangularRoof),
	/// Orthonormal rectangular storey shell with optional `--opening` / `--door-*`.
	RectFloor(rect_floor::RectFloor),
	/// Rounded-rectangle storey shell (straight rectangle kits + ruled corners).
	RoundedRectFloor(rounded_rect_floor::RoundedRectFloor),
	/// I / T / U / L / Z storey shell from central bar + flanges.
	IFloor(i_floor::IFloorCmd),
	/// Single oriented [`richmond_buildings::Rectangle`] kit (floor / wall / ceiling presets).
	Rectangle(rectangle::Rectangle),
	/// Oriented rectangle kit with an inset framed by rectangle kits.
	ClippedRectangle(clipped_rectangle::ClippedRectangle),
	/// Node-chain oriented rectangle strip with a mid-bay inset frame.
	ClippedRectangularStrip(clipped_rectangular_strip::ClippedRectangularStrip),
	/// Single best-fit [`richmond_buildings::FittedRectangle`] (four corners / presets).
	FittedRectangle(fitted_rectangle::FittedRectangle),
	/// Best-fit rectangle with an inset framed by rectangle kits.
	ClippedFittedRectangle(clipped_fitted_rectangle::ClippedFittedRectangle),
	/// Two-rail best-fit rectangle strip with a mid-bay inset frame.
	ClippedFittedRectangularStrip(clipped_fitted_rectangular_strip::ClippedFittedRectangularStrip),
	/// Closed n-gon cross-section polyline → n clipped rectangle strips.
	RectangularNTube(rectangular_n_tube::RectangularNTube),
	/// N-gon disk / annulus filled with right-triangle panel kits.
	ApproximatedCircle(approximated_circle::ApproximatedCircle),
	/// Circular fitted [`richmond_buildings::arcs::ArcSweep`] (not IR `partitions::ArcSweep`).
	ArcSweep(arc_sweep::ArcSweep),
	/// Circular arc with angular clip openings → solid + slice bands.
	ClippedArcSweep(clipped_arc_sweep::ClippedArcSweep),
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
	/// Noisy path → [`richmond_buildings::NoisyRectangularWall`] rectangle strip demo.
	NoisyRectangularWall(noisy_rectangular_wall::NoisyRectangularWall),
	/// Full Wizard's Tower (noise-derived floor count).
	WizardsTower(wizards_tower::WizardsTower),
	/// Stacked circular wall rings (validates kit radius/height scaling).
	StackedRings(stacked_rings::StackedRings),
	/// Hierarchical bedroom (closet / bed / nightstand / ensuite placeholders).
	Bedroom(bedroom::Bedroom),
}

impl Show {
	pub fn react(self, commands: &mut Commands) {
		let preview = match self {
			Self::Linear(cmd) => Ok(cmd.into_preview()),
			Self::Arc90(cmd) => Ok(cmd.into_preview()),
			Self::Arc180(cmd) => Ok(cmd.into_preview()),
			Self::Slice90(cmd) => Ok(cmd.into_preview()),
			Self::Pitch(cmd) => Ok(cmd.into_preview()),
			Self::TessellatedTriangle(cmd) => Ok(cmd.into_preview()),
			Self::TessellatedTriangle3d(cmd) => Ok(cmd.into_preview()),
			Self::ClippedTessellatedTriangle(cmd) => Ok(cmd.into_preview()),
			Self::ClippedQuadPanel(cmd) => Ok(cmd.into_preview()),
			Self::ClippedRuledStrip(cmd) => Ok(cmd.into_preview()),
			Self::Tube(cmd) => Ok(cmd.into_preview()),
			Self::ConnectingHall(cmd) => Ok(cmd.into_preview()),
			Self::ArcFloor(cmd) => cmd.into_preview(),
			Self::ArcTower(cmd) => Ok(cmd.into_preview()),
			Self::ConnectingShells(cmd) => Ok(cmd.into_preview()),
			Self::Trazaloid(cmd) => cmd.into_preview(),
			Self::PitchedRectangularRoof(cmd) => cmd.into_preview(),
			Self::RectFloor(cmd) => cmd.into_preview(),
			Self::RoundedRectFloor(cmd) => cmd.into_preview(),
			Self::IFloor(cmd) => cmd.into_preview(),
			Self::Rectangle(cmd) => Ok(cmd.into_preview()),
			Self::ClippedRectangle(cmd) => Ok(cmd.into_preview()),
			Self::ClippedRectangularStrip(cmd) => Ok(cmd.into_preview()),
			Self::FittedRectangle(cmd) => Ok(cmd.into_preview()),
			Self::ClippedFittedRectangle(cmd) => Ok(cmd.into_preview()),
			Self::ClippedFittedRectangularStrip(cmd) => Ok(cmd.into_preview()),
			Self::RectangularNTube(cmd) => Ok(cmd.into_preview()),
			Self::ApproximatedCircle(cmd) => Ok(cmd.into_preview()),
			Self::ArcSweep(cmd) => Ok(cmd.into_preview()),
			Self::ClippedArcSweep(cmd) => Ok(cmd.into_preview()),
			Self::QuadPanel(cmd) => Ok(cmd.into_preview()),
			Self::PanelComplex(cmd) => Ok(cmd.into_preview()),
			Self::QuadPanelComplex(cmd) => Ok(cmd.into_preview()),
			Self::RuledPitch(cmd) => Ok(cmd.into_preview()),
			Self::Polyline(cmd) => Ok(cmd.into_preview()),
			Self::NoisyRectangularWall(cmd) => Ok(cmd.into_preview()),
			Self::WizardsTower(cmd) => Ok(cmd.into_preview()),
			Self::StackedRings(cmd) => Ok(cmd.into_preview()),
			Self::Bedroom(cmd) => Ok(cmd.into_preview()),
		};
		match preview {
			Ok((subject, transform)) => {
				commands.insert_resource(PreviewConfig { subject, transform });
			}
			Err(err) => {
				error!("show failed: {err}");
				commands.insert_resource(GameCommandStatusText(format!("show failed: {err}")));
			}
		}
	}
}
