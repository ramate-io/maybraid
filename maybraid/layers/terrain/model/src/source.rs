//! A layer source fills one model. It names that model, installs its systems
//! into [`SourceSystems`], and wipes only the entries it wrote.
//!
//! Nothing here is a mode, a state, or a run condition. The world decides when
//! the set runs and when [`LayerSource::wipe`] runs.

use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;

/// Systems [`LayerSource::install`] adds for `S`, on [`Update`].
///
/// The set is only a label. It does not know which mode is live. The impls do
/// not bound `S`, so a source type stays a marker.
pub struct SourceSystems<S: 'static>(PhantomData<fn() -> S>);

impl<S: 'static> Default for SourceSystems<S> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<S: 'static> Clone for SourceSystems<S> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<S: 'static> Copy for SourceSystems<S> {}

impl<S: 'static> PartialEq for SourceSystems<S> {
	fn eq(&self, _other: &Self) -> bool {
		true
	}
}

impl<S: 'static> Eq for SourceSystems<S> {}

impl<S: 'static> Hash for SourceSystems<S> {
	fn hash<H: Hasher>(&self, _state: &mut H) {}
}

impl<S: 'static> Debug for SourceSystems<S> {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "SourceSystems<{}>", std::any::type_name::<S>())
	}
}

impl<S: 'static> SystemSet for SourceSystems<S> {
	fn dyn_clone(&self) -> Box<dyn SystemSet> {
		Box::new(*self)
	}
}

/// Fills `Model` from its own systems and removes only what those systems wrote.
pub trait LayerSource: Send + Sync + 'static {
	type Model;

	/// Add systems in [`SourceSystems<Self>`] on [`Update`].
	fn install(app: &mut App);

	/// Remove only the entries this source wrote.
	fn wipe(world: &mut World);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Resource, Default)]
	struct StubEntries {
		count: u32,
		wipes: u32,
	}

	struct StubSource;

	impl LayerSource for StubSource {
		type Model = ();

		fn install(app: &mut App) {
			app.init_resource::<StubEntries>()
				.add_systems(Update, write_entry.in_set(SourceSystems::<Self>::default()));
		}

		fn wipe(world: &mut World) {
			let mut entries = world.resource_mut::<StubEntries>();
			entries.count = 0;
			entries.wipes += 1;
		}
	}

	fn write_entry(mut entries: ResMut<StubEntries>) {
		entries.count += 1;
	}

	#[test]
	fn install_writes_entries_and_wipe_clears_only_those() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		StubSource::install(&mut app);
		app.update();
		let wrote = app.world().resource::<StubEntries>().count;
		if wrote != 1 {
			return Err(anyhow::anyhow!("install should write one entry, wrote {wrote}"));
		}
		StubSource::wipe(app.world_mut());
		let entries = app.world().resource::<StubEntries>();
		if entries.count != 0 || entries.wipes != 1 {
			return Err(anyhow::anyhow!("wipe should clear the entry once"));
		}
		app.update();
		let entries = app.world().resource::<StubEntries>();
		if entries.count != 1 {
			return Err(anyhow::anyhow!("the installed system still runs after wipe"));
		}
		Ok(())
	}
}
