//! Drive [`lod::LodPresentGate`] from a presenter subscription.

use std::marker::PhantomData;

use bevy::prelude::*;
use lod::{LodPresentGate, LodPresentSystems};

use crate::ModeSubscription;

/// Runs in `Update` after the mode changes and before present produce.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LodPresentGateSync;

/// Copy `K`'s subscription into channel `C`'s present gate.
pub fn sync_lod_present_gate<K, C>(
	subscription: ModeSubscription<K>,
	mut gate: ResMut<LodPresentGate<C>>,
) where
	K: Send + Sync + 'static,
	C: Send + Sync + 'static,
{
	let open = subscription.active();
	if gate.open != open {
		gate.open = open;
	}
}

/// Init [`LodPresentGate<C>`] and sync it from `K` before present produce.
pub fn install_lod_present_gate<K, C>(app: &mut App)
where
	K: Send + Sync + 'static,
	C: Send + Sync + 'static,
{
	if app.is_plugin_added::<LodPresentGatePlugin<K, C>>() {
		return;
	}
	app.add_plugins(LodPresentGatePlugin::<K, C>::default());
}

/// One sync per `(K, C)` pair.
pub struct LodPresentGatePlugin<K, C>(PhantomData<fn() -> (K, C)>);

impl<K, C> Default for LodPresentGatePlugin<K, C> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<K, C> Plugin for LodPresentGatePlugin<K, C>
where
	K: Send + Sync + 'static,
	C: Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		app.init_resource::<LodPresentGate<C>>()
			.configure_sets(
				Update,
				LodPresentGateSync.before(LodPresentSystems::Produce),
			)
			.add_systems(Update, sync_lod_present_gate::<K, C>.in_set(LodPresentGateSync));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		subscribe_mode, ActiveGenerationMode, GenerationMode, GenerationModePlugin,
	};
	use bevy::state::app::StatesPlugin;

	struct Alpha;
	struct Beta;
	struct Presenter;
	struct Chan;

	impl GenerationMode for Alpha {}
	impl GenerationMode for Beta {}

	#[test]
	fn gate_follows_the_subscription() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
			LodPresentGatePlugin::<Presenter, Chan>::default(),
		));
		subscribe_mode::<Presenter, Alpha>(&mut app);
		app.update();
		anyhow::ensure!(app.world().resource::<LodPresentGate<Chan>>().open, "alpha opens");

		app.world_mut()
			.resource_mut::<bevy::prelude::NextState<ActiveGenerationMode>>()
			.set(ActiveGenerationMode::of::<Beta>());
		app.update();
		anyhow::ensure!(
			!app.world().resource::<LodPresentGate<Chan>>().open,
			"beta closes"
		);
		Ok(())
	}
}
