pub mod viewport;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::collections::HashMap;
use std::ops::Range;

pub trait SkillmapInput: Component {}

pub trait SkillmapOutput: Component {}

/// The user facing SkillMap identifier.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SkillMapId(pub u32);

/// The internal SkillMap identifier, indicates the SkillMap id has been mapped to a render layer.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReifiedSkillMapId(SkillMapId);

/// The SkillMap identifier that needs to be mapped to a render layer.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnreifiedSkillMapId(SkillMapId);

/// The render layer for a given SkillMap.
#[derive(Component, Clone, PartialEq, Eq)]
pub struct SkillMapRenderLayer(pub RenderLayers);

#[derive(Resource)]
pub struct SkillMapRenderLayers {
	render_layers: HashMap<SkillMapId, RenderLayers>,
	range: Range<u32>,
}

impl SkillMapRenderLayers {
	pub fn new() -> Self {
		Self { render_layers: HashMap::new(), range: 0xcafe..0xdead }
	}

	fn insert(&mut self, skill_map_id: SkillMapId) -> Option<SkillMapRenderLayer> {
		match self.render_layers.get(&skill_map_id) {
			Some(render_layers) => Some(SkillMapRenderLayer(render_layers.clone())),
			None => {
				if let Some(next) = self.range.next() {
					let render_layers = RenderLayers::layer(next as usize);
					self.render_layers.insert(skill_map_id, render_layers.clone());
					Some(SkillMapRenderLayer(render_layers))
				} else {
					None
				}
			}
		}
	}

	fn insert_reified(
		&mut self,
		skill_map_id: SkillMapId,
	) -> Option<(SkillMapRenderLayer, ReifiedSkillMapId)> {
		self.insert(skill_map_id.clone())
			.map(|render_layer| (render_layer, ReifiedSkillMapId(skill_map_id.clone())))
	}

	fn get(&self, skill_map_id: &SkillMapId) -> Option<SkillMapRenderLayer> {
		self.render_layers
			.get(&skill_map_id)
			.map(|render_layers| SkillMapRenderLayer(render_layers.clone()))
	}

	fn get_reified(
		&self,
		skill_map_id: SkillMapId,
	) -> Option<(SkillMapRenderLayer, ReifiedSkillMapId)> {
		self.get(&skill_map_id)
			.map(|render_layers| (render_layers, ReifiedSkillMapId(skill_map_id.clone())))
	}
}

pub struct LazySkillMapRegistrationPlugin;

impl LazySkillMapRegistrationPlugin {
	/// Determines if a SkillMap has been reified, and if not, registers it as unreified.
	pub fn register_skill_map(
		&self,
		mut commands: Commands,
		skillmap_render_layers: Res<SkillMapRenderLayers>,
		query: Query<(Entity, &SkillMapId), Added<SkillMapId>>,
	) {
		for (entity, skill_map_id) in &query {
			if let Some((render_layer, reified_skill_map_id)) =
				skillmap_render_layers.get_reified(skill_map_id.clone())
			{
				commands.entity(entity).insert(reified_skill_map_id);
				commands.entity(entity).insert(render_layer);
			} else {
				commands.entity(entity).insert(UnreifiedSkillMapId(skill_map_id.clone()));
			}
		}
	}

	/// Reifies a SkillMap by mapping it to a render layer.
	pub fn reify_skill_map(
		&self,
		mut commands: Commands,
		mut skillmap_render_layers: ResMut<SkillMapRenderLayers>,
		query: Query<(Entity, &SkillMapId, &UnreifiedSkillMapId), Added<UnreifiedSkillMapId>>,
	) {
		for (entity, _skill_map_id, unreified_skill_map_id) in &query {
			if let Some((render_layer, reified_skill_map_id)) =
				skillmap_render_layers.insert_reified(unreified_skill_map_id.0.clone())
			{
				commands.entity(entity).insert(reified_skill_map_id);
				commands.entity(entity).insert(render_layer);
			}
		}
	}
}
