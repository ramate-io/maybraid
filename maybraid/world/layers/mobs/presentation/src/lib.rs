//! [`MobPresentationPlugin`]: hosts, group roots, and surface fit through the model.
//!
//! The `Last` despawn of hosts and [`mob_intelligence::MemberOf`] members stays here.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use layer_stack::{install_lod_present_gate, subscribe_mode, GenerationMode};
use mob_layer_model::{MobGeneration, Mobs};
use terrain_layer_model::TerrainModel;

mod present;

pub use present::{install_mob_cell_teardown, MobPresenterState, PresentedMobCell};

/// Marker for mob-presenter subscriptions on [`Mobs<B>`].
pub struct MobPresent;

/// Spawn placed scenes, group roots, and fit them to the ground.
pub trait MobPresentation: MobGeneration {
	/// Channel whose [`lod::LodPresentGate`] this layer opens and closes.
	type Channel: Send + Sync + 'static;

	fn install_presentation(app: &mut App);
}

/// Presents generated mobs of model `B` while `Mode` is subscribed.
pub struct MobPresentationPlugin<Mode, B>(PhantomData<fn() -> (Mode, B)>);

impl<Mode, B> Default for MobPresentationPlugin<Mode, B> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

pub struct MobPresentationCore<B>(PhantomData<fn() -> B>);

impl<B> Default for MobPresentationCore<B> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<B: MobPresentation> Plugin for MobPresentationCore<B>
where
	Mobs<B>: TerrainModel,
{
	fn build(&self, app: &mut App) {
		B::install_presentation(app);
		install_mob_cell_teardown(app);
	}
}

impl<Mode, B> Plugin for MobPresentationPlugin<Mode, B>
where
	Mode: GenerationMode,
	B: MobPresentation,
	Mobs<B>: TerrainModel,
{
	fn build(&self, app: &mut App) {
		subscribe_mode::<(Mobs<B>, MobPresent), Mode>(app);
		install_lod_present_gate::<(Mobs<B>, MobPresent), B::Channel>(app);
		if !app.is_plugin_added::<MobPresentationCore<B>>() {
			app.add_plugins(MobPresentationCore::<B>::default());
		}
	}

	fn finish(&self, app: &mut App) {
		Mobs::<B>::require_generation(app);
	}
}

#[cfg(test)]
mod tests;
