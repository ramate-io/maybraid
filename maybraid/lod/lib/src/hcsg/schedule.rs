//! Schedule, budgets, and messages shared by every generate subscriber.

use std::marker::PhantomData;
use std::time::Duration;

use bevy::prelude::*;

use crate::gen::Id;
use crate::jobs::ensure_lod_job_counter;
use crate::lod_ref::{LodNodePlugin, LodNodeSystems};

/// One origin id inserted by generate.
///
/// Present consumes this impulse instead of broadphasing the unchanged keep
/// ring every frame.
#[derive(Message, Debug, Clone, Copy)]
pub struct LodGenerated<T: Send + Sync + 'static> {
	pub id: Id,
	pub _marker: PhantomData<T>,
}

impl<T: Send + Sync + 'static> LodGenerated<T> {
	pub fn new(id: Id) -> Self {
		Self { id, _marker: PhantomData }
	}
}

/// How many origin ids each generate drain may materialize per frame
/// for channel `C`.
///
/// Independent of scene and presentation. Each channel has its own
/// resource, so assemblers set a value without last-insert-wins.
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

/// Independent wall-clock guard applied by each generate drain.
///
/// [`LodGenerateBudget`] remains an ID ceiling. The drain stops before its next
/// region or ID quantum after this duration has elapsed.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodGenerateTimeBudget {
	/// Per-drain wall time. Zero disables the time limit.
	pub time_per_frame: Duration,
	/// Warn when one non-interruptible region/ID quantum exceeds this duration.
	/// Zero disables warnings.
	pub max_atomic_cost: Duration,
}

impl Default for LodGenerateTimeBudget {
	fn default() -> Self {
		Self { time_per_frame: Duration::from_millis(2), max_atomic_cost: Duration::from_millis(3) }
	}
}

/// Ordering inside the generate layer (not the scene stack).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodGenerateSystems {
	Produce,
	Drain,
}

pub(crate) fn ensure_generate_sets(app: &mut App) {
	if app.is_plugin_added::<LodGenerateSetsPlugin>() {
		return;
	}
	app.add_plugins(LodGenerateSetsPlugin);
}

struct LodGenerateSetsPlugin;

impl Plugin for LodGenerateSetsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LodNodePlugin>() {
			app.add_plugins(LodNodePlugin);
		}
		ensure_lod_job_counter(app);
		app.init_resource::<LodGenerateTimeBudget>().configure_sets(
			Update,
			(LodGenerateSystems::Produce, LodGenerateSystems::Drain)
				.chain()
				.after(LodNodeSystems::Track),
		);
	}
}
