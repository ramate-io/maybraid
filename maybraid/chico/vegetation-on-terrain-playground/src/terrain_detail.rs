//! Durham-height terrain-detail stream plus isolated `/show` pins.

use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_terrain_detail::{
	spawn_rock, OutcroppingExtent, OutcroppingKind, RockComponent, RockPlacement,
	TerrainDetailIndex, TerrainDetailPresenterState, TerrainDetailStreamLod,
	TerrainDetailStreamSpec, TerrainDetailWorldSample, TerrainOutcropping,
};
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};
use lod::gen::{Id, Version};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use procedural_common::NoiseParams;

use crate::commands::ShowKind;
use crate::groves::{DurhamGroveSample, OwnedDurhamTerrain};
use crate::{PlaygroundConfig, WorldBaseTerrain};

#[derive(Component)]
pub struct TerrainDetailRoot;

impl<T> TerrainDetailWorldSample for DurhamGroveSample<T>
where
	T: chico_groves::GroveTerrain,
{
	fn height_at(&self, position: Vec3) -> f32 {
		chico_groves::GroveWorldSample::height_at(self, position)
	}
}

/// Present outcroppings grown against the live Durham height field.
#[derive(SystemParam)]
pub struct DurhamTerrainDetailPresenter<'w, 's> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, TerrainDetailPresenterState>,
	store: Res<'w, TerrainEntryStore>,
	layout: Res<'w, TerrainCellLayout>,
	base: Res<'w, WorldBaseTerrain>,
}

impl RegionPresenter<TerrainOutcropping, TerrainDetailIndex>
	for DurhamTerrainDetailPresenter<'_, '_>
{
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented_version(id)
	}

	fn handle(
		&mut self,
		id: Id,
		version: Version,
		outcropping: &TerrainOutcropping,
		_lod_ref: &LodRef,
	) {
		let world = DurhamGroveSample::from_terrain(OwnedDurhamTerrain::from_store(
			&self.store,
			&self.layout,
			&self.base.0,
		));
		self.state.present_with_world(&mut self.commands, id, version, outcropping, world);
	}

	fn hide(&mut self, id: Id) {
		self.state.hide(&mut self.commands, id);
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.state.is_hidden(id)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented_ids()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		self.state.remove_stale(&mut self.commands, wanted);
	}

	fn cull(
		&mut self,
		spatial_index: &TerrainDetailIndex,
		keep: &HashSet<Id>,
		despawn_budget: u32,
	) -> u32 {
		self.state.cull(&mut self.commands, spatial_index, keep, despawn_budget)
	}
}

/// Enable the terrain-detail bullseyes when a stream spec is set.
pub fn stream_terrain_detail(
	mut commands: Commands,
	config: Res<PlaygroundConfig>,
	camera: Query<&Transform, With<Camera3d>>,
	mut lod: TerrainDetailStreamLod,
	mut last_key: Local<Option<String>>,
) {
	let spec = config.terrain_detail.as_ref();
	let cam = camera.single().ok().map(|t| t.translation);
	lod.apply_spec(&mut commands, spec, cam, &mut last_key);
}

pub fn spawn_show_pins(
	commands: &mut Commands,
	config: &PlaygroundConfig,
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	fallback: &durham_terrain_models::BaseTerrainNoise,
) -> usize {
	let Some(kind) = config.show else {
		return 0;
	};
	let world = DurhamGroveSample::new(store, layout, fallback);
	if let Some(component) = kind.component() {
		return spawn_component_pins(commands, config, component, &world);
	}
	if let Some(outcropping) = kind.outcropping() {
		return spawn_outcropping_pins(commands, config, outcropping, &world);
	}
	0
}

fn spawn_component_pins(
	commands: &mut Commands,
	config: &PlaygroundConfig,
	component: RockComponent,
	world: &impl TerrainDetailWorldSample,
) -> usize {
	let radius = config.tile_radius.max(0);
	let mut count = 0usize;
	for ix in -radius..=radius {
		for iz in -radius..=radius {
			let xz = Vec2::new(ix as f32 * 12.0, iz as f32 * 12.0);
			let scale = 4.0;
			let height = world.height_at(Vec3::new(xz.x, 0.0, xz.y));
			let entity = spawn_rock(
				commands,
				RockPlacement {
					component,
					translation: Vec3::new(xz.x, height - scale * 0.2, xz.y),
					yaw: 0.0,
					scale,
				},
			);
			commands.entity(entity).insert(TerrainDetailRoot);
			count += 1;
		}
	}
	count
}

fn spawn_outcropping_pins(
	commands: &mut Commands,
	config: &PlaygroundConfig,
	kind: OutcroppingKind,
	world: &impl TerrainDetailWorldSample,
) -> usize {
	let radius = config.tile_radius.max(0);
	let noise = NoiseParams::default();
	let mut count = 0usize;
	for ix in -radius..=radius {
		for iz in -radius..=radius {
			let extent = OutcroppingExtent::from_cell_index(ix, iz);
			for placement in kind.populate(extent, noise, world) {
				let entity = spawn_rock(commands, placement);
				commands.entity(entity).insert(TerrainDetailRoot);
				count += 1;
			}
		}
	}
	count
}

pub fn formation_spec_from_show(kind: ShowKind) -> Option<TerrainDetailStreamSpec> {
	kind.formation().map(|formation| TerrainDetailStreamSpec {
		formation: Some(formation),
		..TerrainDetailStreamSpec::default()
	})
}
