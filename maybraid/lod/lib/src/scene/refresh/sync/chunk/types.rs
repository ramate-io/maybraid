//! Markers, fulfill plans, and per-frame budgets.

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::scene::level::LodSceneLevel;

/// Marker: this [`crate::LodLevelRoot`] is still awaiting warm-swap completion.
///
/// Content may already be [`LodLevelRootStreamed`] while nested hosts catch up.
/// Cold-start roots may be visible while pending.
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodLevelRootPending;

/// This level root's chunk plan is fully spawned (full scene representation).
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodLevelRootStreamed;

/// This [`crate::LodSceneHost`] has a full scene representation available (Streamed).
///
/// Means at least one level root finished content streaming and its next-level
/// nested hosts were Streamed. Does **not** require the host to be at its
/// current desired [`LodSceneLevel`].
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodSceneHostStreamed;

/// Entity is tearing down under [`super::super::cull::drain_lod_cull`].
///
/// Frozen fulfill plans stay until [`Self::started`] so sticky desired-level
/// resume can continue the same job. Once teardown spends budget, the plan is
/// dropped and sticky no longer applies.
#[derive(Debug, Clone, Copy, Component)]
pub struct LodWantsCull {
	/// True after the first teardown step (plan cleared / child despawned).
	pub started: bool,
}

/// Remaining weighted primitives for a pending level root.
///
/// Frozen at begin: host mutability does not rewrite this queue mid-job.
#[derive(Component)]
pub struct LodChunkFulfillment {
	pub queue: VecDeque<(u32, Box<dyn bevy::scene::Scene>)>,
	/// Primitive count at job begin (Streamed when `spawned == expected`).
	pub expected: usize,
	pub spawned: usize,
	/// No ready sibling root when the job began (cold / presence work).
	pub cold: bool,
}

impl LodChunkFulfillment {
	pub(super) fn is_content_complete(&self) -> bool {
		self.queue.is_empty() && self.spawned >= self.expected
	}
}

/// Per-frame weight / begin budgets for spawn vs cull (independent clocks).
#[derive(Resource, Debug, Clone, Copy)]
pub struct LodChunkFulfillBudget {
	/// Relative weight units for drain each frame.
	pub spawn_weights_per_frame: u32,
	/// Relative weight units for cull drain each frame.
	pub cull_weights_per_frame: u32,
	/// Max new fulfill jobs started per frame (shared across all host `T`).
	pub begins_per_frame: u32,
}

impl Default for LodChunkFulfillBudget {
	fn default() -> Self {
		Self {
			spawn_weights_per_frame: 512,
			cull_weights_per_frame: 64,
			begins_per_frame: 48,
		}
	}
}

/// Remaining spawn / cull weight for the current frame.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct LodChunkBudgetClock {
	pub spawn_remaining: u32,
	pub cull_remaining: u32,
}

/// Shared begin admission quotas for the current frame (all host `T`s).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct LodChunkBeginClock {
	pub presence_remaining: u32,
	pub level_remaining: u32,
	/// When true, begin systems prefer cold (presence) hosts first.
	pub presence_first: bool,
}

/// Round-robin cursors + frame parity for drain scheduling.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct LodChunkDrainCursor {
	pub frame: u64,
	pub presence: u32,
	pub high: u32,
	pub medium: u32,
	pub low: u32,
	pub ultra: u32,
	pub other: u32,
}

/// Diagnostic: last `scene_chunks_with_level` timing (scene build, not apply).
#[derive(Resource, Debug, Default)]
pub struct LodChunkFulfillDiag {
	pub last_scene_chunks_ms: f64,
	pub last_level: Option<LodSceneLevel>,
}
