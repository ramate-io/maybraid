//! Shared terminal canopy: noisy ball or plane splay ([`LayeredTerminalCanopy`]).

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use render_item::{CascadeChunk, RenderItem};

/// One terminal foliage item: volumetric ball or radial plane splay.
#[derive(Clone, Debug)]
pub struct LayeredTerminalCanopy<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub ball: ChicoBall<LeafM, LeafS>,
	pub plane_splay: PlaneSplay<LeafM, LeafS>,
}

#[derive(Clone, Debug)]
pub enum LayeredTerminalCanopyItem<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	Ball(ChicoBall<LeafM, LeafS>),
	PlaneSplay(PlaneSplay<LeafM, LeafS>),
}

impl<LeafM, LeafS> LayeredTerminalCanopy<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub fn new(ball: ChicoBall<LeafM, LeafS>, plane_splay: PlaneSplay<LeafM, LeafS>) -> Self {
		Self { ball, plane_splay }
	}

	pub fn ball_item(&self) -> LayeredTerminalCanopyItem<LeafM, LeafS> {
		LayeredTerminalCanopyItem::Ball(self.ball.clone())
	}

	pub fn plane_splay_item(&self) -> LayeredTerminalCanopyItem<LeafM, LeafS> {
		LayeredTerminalCanopyItem::PlaneSplay(self.plane_splay.clone())
	}

	/// Plane splay with per-node variation from [`node_mix_seed`](chico_sbs_geometry::render::mix_seed::node_mix_seed).
	pub fn plane_splay_item_varied(
		&self,
		node_idx: usize,
		position: Vec3,
	) -> LayeredTerminalCanopyItem<LeafM, LeafS> {
		let mut splay = self.plane_splay.clone();
		let seed = chico_sbs_geometry::render::mix_seed::node_mix_seed(node_idx, position);
		splay.icosphere_subdivisions = seed % 2;
		splay.leaf_disc_radius = 0.18 + 0.12 * ((seed % 17) as f32 / 16.0);
		LayeredTerminalCanopyItem::PlaneSplay(splay)
	}
}

impl<LeafM, LeafS> RenderItem for LayeredTerminalCanopyItem<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		match self {
			Self::Ball(b) => b.spawn_render_items(commands, cascade_chunk, transform),
			Self::PlaneSplay(p) => p.spawn_render_items(commands, cascade_chunk, transform),
		}
	}
}
