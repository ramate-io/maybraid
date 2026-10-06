//! [`Present<Mode, L>`]: subscribe one label and install presentation once.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};

use crate::layer::{register_layer_label, LayerPresentation, LayerPresentationCore};
use crate::{
	install_lod_present_gate, subscribe_mode, GenerationMode, LayerGenerationCore, RequireLayer,
};

/// Presents layer `L` while `Mode` is subscribed.
pub struct Present<Mode, L>(PhantomData<fn() -> (Mode, L)>);

impl<Mode, L> Default for Present<Mode, L> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<Mode, L> Plugin for Present<Mode, L>
where
	Mode: GenerationMode,
	L: LayerPresentation,
{
	fn build(&self, app: &mut App) {
		register_layer_label::<L>(app);
		subscribe_mode::<L, Mode>(app);
		install_lod_present_gate::<L, L>(app);
		if !app.is_plugin_added::<LayerPresentationCore<L>>() {
			L::install_presentation(app);
			app.add_plugins(LayerPresentationCore::<L>::default());
		}
	}

	fn finish(&self, app: &mut App) {
		app.require_layer::<LayerGenerationCore<L>, Self>();
	}
}
