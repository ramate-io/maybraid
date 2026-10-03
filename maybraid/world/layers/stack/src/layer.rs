//! A layer describes only itself: install, apply, optional clear, and a label.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;

use crate::GenerationMode;

/// One world layer. Wrappers implement this by delegating to a named model.
pub trait Layer: Send + Sync + 'static {
	/// Stable name for the later presentation-layer command. Unique per app.
	const LABEL: &'static str;
	type Config: Clone + Send + Sync + 'static;
	fn install_generation(app: &mut App);
	fn apply_generation(world: &mut World, config: &Self::Config);
	fn clear_generation(_world: &mut World) {}
	fn require_lower(app: &App);
}

/// Presentation install for a [`Layer`]. Extra present gates stay here.
pub trait LayerPresentation: Layer {
	fn install_presentation(app: &mut App);
}

/// A mode's per-layer writes. Modes implement this for the stacks they run.
pub trait Scheme<L: Layer>: GenerationMode {
	fn install(app: &mut App, config: &L::Config);
}

/// Shared generation install for `L`, added once.
pub struct LayerGenerationCore<L>(PhantomData<fn() -> L>);

impl<L> Default for LayerGenerationCore<L> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<L: Layer> Plugin for LayerGenerationCore<L> {
	fn build(&self, _app: &mut App) {}
}

/// Shared presentation install for `L`, added once.
pub struct LayerPresentationCore<L>(PhantomData<fn() -> L>);

impl<L> Default for LayerPresentationCore<L> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<L: LayerPresentation> Plugin for LayerPresentationCore<L> {
	fn build(&self, _app: &mut App) {}
}

/// Per-mode config the scheme systems read, keyed by the same `L` as the plugin.
#[derive(Resource, Clone)]
pub struct LayerModeConfig<Mode, L: Layer> {
	pub config: L::Config,
	_mode: PhantomData<fn() -> Mode>,
}

impl<Mode: GenerationMode, L: Layer> LayerModeConfig<Mode, L> {
	pub fn new(config: L::Config) -> Self {
		Self { config, _mode: PhantomData }
	}
}

/// Ordering set keyed on the same `L` as [`crate::Generate`] / [`crate::Present`].
pub struct LayerSystems<L: Send + Sync + 'static>(PhantomData<fn() -> L>);

impl<L: Send + Sync + 'static> Default for LayerSystems<L> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<L: Send + Sync + 'static> Clone for LayerSystems<L> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<L: Send + Sync + 'static> Copy for LayerSystems<L> {}

impl<L: Send + Sync + 'static> PartialEq for LayerSystems<L> {
	fn eq(&self, _other: &Self) -> bool {
		true
	}
}

impl<L: Send + Sync + 'static> Eq for LayerSystems<L> {}

impl<L: Send + Sync + 'static> Hash for LayerSystems<L> {
	fn hash<H: Hasher>(&self, _state: &mut H) {}
}

impl<L: Send + Sync + 'static> Debug for LayerSystems<L> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "LayerSystems<{}>", std::any::type_name::<L>())
	}
}

impl<L: Send + Sync + 'static> SystemSet for LayerSystems<L> {
	fn dyn_clone(&self) -> Box<dyn SystemSet> {
		Box::new(*self)
	}
}

#[derive(Resource, Default)]
struct LayerLabels {
	by_type: HashMap<TypeId, &'static str>,
	by_label: HashMap<&'static str, TypeId>,
}

/// Record `L::LABEL`. A second layer type with the same label is a programming error.
pub fn register_layer_label<L: Layer>(app: &mut App) {
	let id = TypeId::of::<L>();
	let mut labels = app.world_mut().get_resource_or_insert_with(LayerLabels::default);
	if let Some(&existing) = labels.by_type.get(&id) {
		if existing != L::LABEL {
			panic!(
				"layer {} already registered as {existing:?}; cannot also use {:?}",
				std::any::type_name::<L>(),
				L::LABEL
			);
		}
		return;
	}
	if let Some(&other) = labels.by_label.get(L::LABEL) {
		if other != id {
			panic!(
				"duplicate layer LABEL {:?}: already registered, cannot also register {}",
				L::LABEL,
				std::any::type_name::<L>()
			);
		}
		return;
	}
	labels.by_type.insert(id, L::LABEL);
	labels.by_label.insert(L::LABEL, id);
}
