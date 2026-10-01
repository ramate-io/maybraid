//! Which world the shell is showing. Layer crates never see this.
//!
//! [`WorldMode::Discovery`] is the default: menus and the playground never enter
//! Training, and today's `TrainingGrounds` flag defaulted to false, which is
//! Discovery's layout, coverage, and forest radius. There is no `Off`. An off
//! state would have to copy Discovery's settings, and an `OnEnter` of that
//! state would retarget Durham at startup.

use bevy::prelude::*;
use terrain_layer_model::{LayerSource, SourceSystems};

/// Live world session. The shell requests this with its flow; layer flags
/// follow the transition.
#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum WorldMode {
	/// Playable rings. Menus and `maybraid-world-playground` stay here.
	#[default]
	Discovery,
	/// One pinned fine patch.
	Training,
}

impl WorldMode {
	pub fn is_training(self) -> bool {
		matches!(self, Self::Training)
	}
}

/// Initializes [`WorldMode`] to [`WorldMode::Discovery`].
pub(crate) struct WorldModePlugin;

impl Plugin for WorldModePlugin {
	fn build(&self, app: &mut App) {
		app.init_state::<WorldMode>();
	}
}

/// `app.add_world_source::<S>(mode)` installs `S`, runs [`SourceSystems<S>`] only
/// while `mode` is active, and calls [`LayerSource::wipe`] once on exit.
///
/// [`WorldMode`] must already be initialized (`init_state`).
pub trait WorldSourceAppExt {
	fn add_world_source<S: LayerSource>(&mut self, mode: WorldMode) -> &mut Self;
}

impl WorldSourceAppExt for App {
	fn add_world_source<S: LayerSource>(&mut self, mode: WorldMode) -> &mut Self {
		self.configure_sets(Update, SourceSystems::<S>::default().run_if(in_state(mode)));
		S::install(self);
		self.add_systems(OnExit(mode), wipe_world_source::<S>);
		self
	}
}

fn wipe_world_source<S: LayerSource>(world: &mut World) {
	S::wipe(world);
}

#[cfg(test)]
mod tests {
	use bevy::state::app::StatesPlugin;

	use super::*;

	#[derive(Resource, Default)]
	struct StubEntries {
		ticks: u32,
		wipes: u32,
	}

	struct StubSource;

	impl LayerSource for StubSource {
		type Model = ();

		fn install(app: &mut App) {
			app.init_resource::<StubEntries>()
				.add_systems(Update, tick.in_set(SourceSystems::<Self>::default()));
		}

		fn wipe(world: &mut World) {
			let mut entries = world.resource_mut::<StubEntries>();
			entries.ticks = 0;
			entries.wipes += 1;
		}
	}

	fn tick(mut entries: ResMut<StubEntries>) {
		entries.ticks += 1;
	}

	fn mode_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin));
		app.init_state::<WorldMode>();
		app.add_world_source::<StubSource>(WorldMode::Training);
		app
	}

	#[test]
	fn a_source_runs_only_in_its_mode_and_wipes_once_on_exit() -> anyhow::Result<()> {
		let mut app = mode_app();
		app.update();
		let entries = app.world().resource::<StubEntries>();
		if entries.ticks != 0 || entries.wipes != 0 {
			return Err(anyhow::anyhow!("startup in Discovery must not run or wipe the source"));
		}

		app.world_mut().resource_mut::<NextState<WorldMode>>().set(WorldMode::Training);
		app.update();
		if app.world().resource::<StubEntries>().ticks != 1 {
			return Err(anyhow::anyhow!("the source should tick once while Training is active"));
		}
		app.update();
		if app.world().resource::<StubEntries>().ticks != 2 {
			return Err(anyhow::anyhow!("the source should keep ticking in Training"));
		}

		app.world_mut().resource_mut::<NextState<WorldMode>>().set(WorldMode::Discovery);
		app.update();
		let entries = app.world().resource::<StubEntries>();
		if entries.wipes != 1 || entries.ticks != 0 {
			return Err(anyhow::anyhow!("leaving Training wipes once and stops the source"));
		}
		app.update();
		let entries = app.world().resource::<StubEntries>();
		if entries.wipes != 1 || entries.ticks != 0 {
			return Err(anyhow::anyhow!("wipe must not run again while Discovery stays active"));
		}
		Ok(())
	}

	#[derive(Resource, Default)]
	struct TransitionCounts {
		entered: u32,
		exited: u32,
	}

	fn count_enter(mut counts: ResMut<TransitionCounts>) {
		counts.entered += 1;
	}

	fn count_exit(mut counts: ResMut<TransitionCounts>) {
		counts.exited += 1;
	}

	#[test]
	fn setting_the_same_mode_reenters_and_set_if_neq_does_not() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin));
		app.init_resource::<TransitionCounts>();
		app.init_state::<WorldMode>();
		app.add_systems(OnEnter(WorldMode::Training), count_enter);
		app.add_systems(OnExit(WorldMode::Training), count_exit);
		app.update();
		if app.world().resource::<TransitionCounts>().entered != 0 {
			return Err(anyhow::anyhow!("Discovery startup must not enter Training"));
		}

		app.world_mut().resource_mut::<NextState<WorldMode>>().set(WorldMode::Training);
		app.update();
		let counts = app.world().resource::<TransitionCounts>();
		if counts.entered != 1 || counts.exited != 0 {
			return Err(anyhow::anyhow!("the first enter runs OnEnter only"));
		}

		app.world_mut().resource_mut::<NextState<WorldMode>>().set(WorldMode::Training);
		app.update();
		let counts = app.world().resource::<TransitionCounts>();
		if counts.entered != 2 || counts.exited != 1 {
			return Err(anyhow::anyhow!(
				"NextState::set of the current mode runs OnExit and OnEnter"
			));
		}

		{
			let mut next = app.world_mut().resource_mut::<NextState<WorldMode>>();
			NextState::set_if_neq(&mut next, WorldMode::Training);
		}
		app.update();
		let counts = app.world().resource::<TransitionCounts>();
		if counts.entered != 2 || counts.exited != 1 {
			return Err(anyhow::anyhow!("set_if_neq of the current mode does not re-enter"));
		}
		Ok(())
	}
}
