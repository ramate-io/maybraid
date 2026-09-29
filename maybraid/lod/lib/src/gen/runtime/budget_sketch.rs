//! SKETCH — not in the build. Per-channel LOD budgets.
//!
//! Today `LodGenerateBudget` / `LodPresentBudget` are single global resources.
//! Every stream `insert_resource`s its own value, so the last plugin wins:
//!
//! - `register_urbanization_lod` inserts generate = 8,
//! - `register_forest_lod` (added after it by `DevelopmentsOnTerrainPlugin`) inserts 16,
//! - mobs insert nothing and read whatever is there.
//!
//! In the world every channel therefore generates 16 ids / frame, and
//! `generate_urbanization_developments` caps built developments at 16 too.
//! Once layers are separate plugins, insertion order is assembly order, so a
//! global budget would make behavior depend on plugin order. Channel the
//! budgets first; preserve today's effective numbers exactly.

use std::marker::PhantomData;

use bevy::prelude::*;

/// Generate ids admitted per frame for channel `C`.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodGenerateBudget<C> {
	pub ids_per_frame: u32,
	_chan: PhantomData<fn() -> C>,
}

impl<C> LodGenerateBudget<C> {
	pub const fn new(ids_per_frame: u32) -> Self {
		Self { ids_per_frame, _chan: PhantomData }
	}
}

impl<C> Default for LodGenerateBudget<C> {
	/// Same default as the old global.
	fn default() -> Self {
		Self::new(1)
	}
}

// Same shape for `LodPresentBudget<C>` in `presentation/runtime.rs`.

// ── Runtime changes ─────────────────────────────────────────────────────────
//
// generate.rs:260   `budget: Res<LodGenerateBudget>`
//              ->   `budget: Res<LodGenerateBudget<Chan>>`
// runtime.rs:295    same for present.
//
// `LodGeneratePlugin::<T, I, Chan, F>::build` does
// `app.init_resource::<LodGenerateBudget<Chan>>()` (init, never insert), so a
// value set by the assembler before or after the plugin is kept.
//
// ── Call-site migration (values preserve today's effective world numbers) ──
//
// chico/forests/src/stream.rs:87
//     .insert_resource(LodGenerateBudget { ids_per_frame: 16 })
//  -> .insert_resource(LodGenerateBudget::<ForestLodChan>::new(16))
//
// richmond/.../urbanization_stream.rs:96
//     .insert_resource(LodGenerateBudget { ids_per_frame: 8 })
//  -> .insert_resource(LodGenerateBudget::<UrbanizationLodChan>::new(16))
//     // 16, not 8: the forest's insert overwrote 8 in every assembled world.
//     // Restoring 8 is a #720 follow-up, not this refactor.
//
// richmond/.../urbanization_stream.rs:310  (development build cap)
//     budget: Res<LodGenerateBudget>
//  -> budget: Res<LodGenerateBudget<UrbanizationLodChan>>
//
// world/lib/src/mobs.rs  (reads the global implicitly through LodGeneratePlugin)
//  -> .insert_resource(LodGenerateBudget::<MobLodChan>::new(16))
//
// bump-out channels (BumpOutLodChan, MediumBumpOutLodChan) read 16 today too.
//
// chico/forests/src/generation.rs:549 present = 1 -> LodPresentBudget::<ForestLodChan>::new(1),
// and every other present channel keeps the default 1.
//
// Playgrounds that register only one stream got that stream's own number;
// keep those numbers per playground (e.g. developments-only playground: urbanization = 8).
//
// ── Test to add ─────────────────────────────────────────────────────────────
//
// Two channels with different budgets in one App each admit their own count per
// frame (extend gen/runtime/tests.rs; the existing tests switch to `::<TestChan>`).
