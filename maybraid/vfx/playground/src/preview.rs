//! Optional recipe registration and controls local to the playground.
use bevy::ecs::template::template;
use bevy::prelude::*;
use bevy::scene::prelude::bsn;
use maybraid_vfx::{EffectDefinition, EffectRecipe, Vfx, VfxSpawn};
use std::collections::{BTreeMap, BTreeSet};

use crate::commands::VfxShowRequest;
use crate::inspector::{LayerRow, PreviewInspector};

type SpawnPreview = Box<dyn FnOnce(&mut Commands, EffectDefinition, Transform, VfxSpawn) -> Entity>;

pub(crate) struct PreviewEffect {
	pub definition: EffectDefinition,
	spawn: SpawnPreview,
}

impl PreviewEffect {
	pub fn typed<T: EffectRecipe + Component + Clone>(recipe: T) -> Self {
		Self {
			definition: recipe.effect_definition(),
			spawn: Box::new(move |commands, definition, transform, spawn| {
				commands
					.spawn_scene(bsn! {
						~{transform}
						template(move |_ctx| Ok(recipe.clone()))
						@{Vfx { definition, spawn }}
					})
					.id()
			}),
		}
	}

	pub fn definition(definition: EffectDefinition) -> Self {
		Self {
			definition,
			spawn: Box::new(|commands, definition, transform, spawn| {
				commands.spawn_scene(bsn! { ~{transform} @{Vfx { definition, spawn }} }).id()
			}),
		}
	}

	pub fn spawn(
		mut self,
		commands: &mut Commands,
		request: &VfxShowRequest,
		selection: &PreviewSelection,
		inspector: &mut PreviewInspector,
	) -> (Entity, VfxSpawn) {
		let origin = if self.definition.name == maybraid_vfx::DUST {
			Vec3::ZERO
		} else {
			crate::show::SHOW_ORIGIN
		};
		let spawn = VfxSpawn {
			intensity: request.intensity,
			seed: request.seed,
			playback: request.playback,
			..default()
		}
		.resolved();
		let transform = Transform::from_translation(origin + Vec3::Z * request.distance)
			.with_scale(Vec3::splat(request.scale));
		let mut active = Vec::new();
		let mut rows = Vec::new();
		for (index, layer) in self.definition.layers.drain(..).enumerate() {
			let enabled = selection.includes(index);
			rows.push(LayerRow::new(&layer, enabled.then_some(active.len())));
			if enabled {
				active.push(layer);
			}
		}
		self.definition.layers = active;
		let root = (self.spawn)(commands, self.definition, transform, spawn.clone());
		*inspector = PreviewInspector { root: Some(root), layers: rows, failure: None };
		(root, spawn)
	}
}

/// Preview factories are evaluated on every show/rebuild; runtime spawning needs no registry.
#[derive(Resource, Default)]
pub struct PreviewRecipes(BTreeMap<String, Box<dyn Fn() -> PreviewEffect + Send + Sync>>);

impl PreviewRecipes {
	pub(crate) fn build(&self, name: &str) -> Option<PreviewEffect> {
		self.0.get(name).map(|factory| factory())
	}

	pub fn names(&self) -> impl Iterator<Item = &str> {
		self.0.keys().map(String::as_str)
	}
}

/// Register a typed preview with no command parser edits or per-recipe systems.
pub trait VfxPreviewAppExt {
	fn register_vfx_preview<T, F>(&mut self, name: impl Into<String>, factory: F) -> &mut Self
	where
		T: EffectRecipe + Component + Clone,
		F: Fn() -> T + Send + Sync + 'static;
}

impl VfxPreviewAppExt for App {
	fn register_vfx_preview<T, F>(&mut self, name: impl Into<String>, factory: F) -> &mut Self
	where
		T: EffectRecipe + Component + Clone,
		F: Fn() -> T + Send + Sync + 'static,
	{
		self.init_resource::<PreviewRecipes>();
		self.world_mut()
			.resource_mut::<PreviewRecipes>()
			.0
			.insert(name.into(), Box::new(move || PreviewEffect::typed(factory())));
		self
	}
}

#[derive(Resource, Default)]
pub(crate) struct PreviewSelection {
	pub solo: Option<usize>,
	pub muted: BTreeSet<usize>,
}

impl PreviewSelection {
	fn includes(&self, index: usize) -> bool {
		self.solo.is_none_or(|solo| solo == index) && !self.muted.contains(&index)
	}
}

/// One explicit simulation step, in seconds, while the preview remains paused.
#[derive(Resource, Default)]
pub(crate) struct PreviewStep(pub Option<std::time::Duration>);

pub(crate) fn step_preview(
	mut step: ResMut<PreviewStep>,
	mut virtual_time: ResMut<Time<Virtual>>,
	mut time: ResMut<Time>,
) {
	if let Some(delta) = step.0.take() {
		virtual_time.pause();
		virtual_time.advance_by(delta);
		*time = virtual_time.as_generic();
	}
}

#[cfg(test)]
mod tests;
