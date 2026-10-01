//! Presenters subscribe to the generation modes that should draw them.

use std::any::TypeId;
use std::collections::HashSet;
use std::marker::PhantomData;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::generation_mode::{ActiveGenerationMode, GenerationMode};

/// Modes subscribed to presenter `K`. Inserting the same mode twice is a no-op.
#[derive(Resource)]
pub struct ModeSubscribers<K: Send + Sync + 'static> {
	modes: HashSet<TypeId>,
	_marker: PhantomData<fn() -> K>,
}

impl<K: Send + Sync + 'static> Default for ModeSubscribers<K> {
	fn default() -> Self {
		Self { modes: HashSet::new(), _marker: PhantomData }
	}
}

impl<K: Send + Sync + 'static> ModeSubscribers<K> {
	pub fn contains<M: GenerationMode>(&self) -> bool {
		self.modes.contains(&TypeId::of::<M>())
	}
}

/// Subscribe `M` to presenter `K`.
pub fn subscribe_mode<K, M>(app: &mut App)
where
	K: Send + Sync + 'static,
	M: GenerationMode,
{
	app.world_mut()
		.get_resource_or_insert_with(ModeSubscribers::<K>::default)
		.modes
		.insert(TypeId::of::<M>());
}

fn subscribed<K: Send + Sync + 'static>(
	subscribers: Option<&ModeSubscribers<K>>,
	mode: Option<&State<ActiveGenerationMode>>,
) -> bool {
	let Some(id) = mode.and_then(|mode| mode.get().type_id()) else {
		return false;
	};
	subscribers.is_some_and(|subscribers| subscribers.modes.contains(&id))
}

/// True while the active generation mode is subscribed to `K`.
#[derive(SystemParam)]
pub struct ModeSubscription<'w, K: Send + Sync + 'static> {
	subscribers: Option<Res<'w, ModeSubscribers<K>>>,
	mode: Option<Res<'w, State<ActiveGenerationMode>>>,
}

impl<K: Send + Sync + 'static> ModeSubscription<'_, K> {
	pub fn active(&self) -> bool {
		subscribed(self.subscribers.as_deref(), self.mode.as_deref())
	}
}

/// Run condition for systems that should not run while `K` is unsubscribed.
pub fn mode_subscribed<K: Send + Sync + 'static>()
-> impl FnMut(Option<Res<ModeSubscribers<K>>>, Option<Res<State<ActiveGenerationMode>>>) -> bool + Clone
{
	|subscribers, mode| subscribed(subscribers.as_deref(), mode.as_deref())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::generation_mode::GenerationModePlugin;
	use bevy::state::app::StatesPlugin;

	struct Alpha;
	struct Beta;
	struct Presenter;

	impl GenerationMode for Alpha {}
	impl GenerationMode for Beta {}

	#[test]
	fn subscription_follows_the_active_mode() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			GenerationModePlugin::<Beta>::default(),
		));
		subscribe_mode::<Presenter, Alpha>(&mut app);
		app.update();
		{
			let mut state = bevy::ecs::system::SystemState::<ModeSubscription<Presenter>>::new(
				app.world_mut(),
			);
			anyhow::ensure!(
				state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
				"alpha is subscribed"
			);
		}

		app.world_mut()
			.resource_mut::<NextState<ActiveGenerationMode>>()
			.set(ActiveGenerationMode::of::<Beta>());
		app.update();
		{
			let mut state = bevy::ecs::system::SystemState::<ModeSubscription<Presenter>>::new(
				app.world_mut(),
			);
			anyhow::ensure!(
				!state.get(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?.active(),
				"beta is not subscribed"
			);
		}
		Ok(())
	}

	#[test]
	fn double_subscription_is_a_noop() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
		));
		subscribe_mode::<Presenter, Alpha>(&mut app);
		subscribe_mode::<Presenter, Alpha>(&mut app);
		let subscribers = app.world().resource::<ModeSubscribers<Presenter>>();
		anyhow::ensure!(subscribers.contains::<Alpha>());
		anyhow::ensure!(subscribers.modes.len() == 1);
		Ok(())
	}
}
