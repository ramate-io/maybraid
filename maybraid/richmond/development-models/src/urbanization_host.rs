//! Budgeted urbanization host spawn (forest present parallel).
//!
//! One urbanization cell per frame by default. Does not bake [`TerrainWithPads`].

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::log::info_span;
use bevy::prelude::*;
use lod::gen::{GeneratingSpatialIndex, Id, SpatialIndex, Version};
use lod::lod_ref::LodRef;
use lod::presentation::LodPresentKeepRegion;
use lod::{LodGenerateSystems, LodPresentSystems};
use richmond_urbanization::{
	SelectedUrbanization, UrbanDevelopmentKind, UrbanizationLodChan, UrbanizationPresentBullseye,
	UrbanizationStreamSpec,
};

use crate::artifact::BuiltDevelopment;
use crate::config::DevelopmentConfig;
use crate::development::DevelopmentCell;
use crate::host::{DevelopmentHostRoot, DevelopmentHosts};
use crate::index::DevelopmentIndex;
use crate::plugin::register_richmond_development_models_plugin;

/// Cap host spawn to this many unpresented urbanization cells per frame.
#[derive(Resource, Clone, Copy, Debug)]
pub struct UrbanizationHostBudget {
	pub cells_per_frame: usize,
}

impl Default for UrbanizationHostBudget {
	fn default() -> Self {
		Self { cells_per_frame: 1 }
	}
}

#[derive(Resource, Default)]
pub struct UrbanizationPresenterState {
	presented: HashMap<Id, PresentedUrbanization>,
	pending_despawn: VecDeque<Vec<Entity>>,
}

struct PresentedUrbanization {
	version: Version,
	entities: Vec<Entity>,
}

impl UrbanizationPresenterState {
	pub fn clear(&mut self, commands: &mut Commands) {
		for presented in self.presented.values() {
			for entity in &presented.entities {
				commands.entity(*entity).despawn();
			}
		}
		self.presented.clear();
		for entities in self.pending_despawn.drain(..) {
			for entity in entities {
				commands.entity(entity).despawn();
			}
		}
	}

	fn retire(&mut self, id: Id) -> Option<PresentedUrbanization> {
		self.presented.remove(&id)
	}

	pub fn presented_version(&self, id: Id) -> Option<Version> {
		self.presented.get(&id).map(|entry| entry.version)
	}

	pub fn remove_stale(&mut self, commands: &mut Commands, wanted: &HashSet<Id>) {
		let stale: Vec<Id> =
			self.presented.keys().copied().filter(|id| !wanted.contains(id)).collect();
		for id in stale {
			if let Some(entry) = self.presented.remove(&id) {
				self.pending_despawn.push_back(entry.entities);
			}
		}
		while let Some(entities) = self.pending_despawn.pop_front() {
			for entity in entities {
				commands.entity(entity).despawn();
			}
		}
	}

	/// Generate leaf developments and spawn hosts for one urbanization cell.
	pub fn present_urbanization(
		&mut self,
		commands: &mut Commands,
		development: &mut DevelopmentIndex,
		id: Id,
		version: Version,
		selected: &SelectedUrbanization,
		lod_ref: &LodRef,
	) -> Vec<Entity> {
		if let Some(previous) = self.retire(id) {
			self.pending_despawn.push_back(previous.entities);
		}

		let mut entities = Vec::new();
		for leaf in &selected.leaves {
			if leaf.kind == UrbanDevelopmentKind::Empty {
				continue;
			}
			let leaf_id = leaf.id();
			{
				let _span = info_span!("richmond_development_generation").entered();
				if GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate(
					development,
					leaf_id,
					lod_ref,
				)
				.is_none()
				{
					continue;
				}
				if GeneratingSpatialIndex::<BuiltDevelopment>::get_or_generate(
					development,
					leaf_id,
					lod_ref,
				)
				.is_none()
				{
					continue;
				}
			}
			let Some(built) = SpatialIndex::<BuiltDevelopment>::get(development, leaf_id) else {
				continue;
			};
			{
				let _span = info_span!("richmond_host_spawn").entered();
				entities.extend(spawn_tagged_hosts(commands, built));
			}
		}

		self.presented
			.insert(id, PresentedUrbanization { version, entities: entities.clone() });
		entities
	}
}

fn spawn_tagged_hosts(commands: &mut Commands, development: &impl DevelopmentHosts) -> Vec<Entity> {
	let mut spawned = Vec::new();
	for host in development.hosts() {
		for entity in host.spawn(commands) {
			commands.entity(entity).insert(DevelopmentHostRoot);
			spawned.push(entity);
		}
	}
	spawned
}

/// Expand tracked urbanization cells into [`BuiltDevelopment`] hosts and cull stale ones.
pub fn present_urbanization_hosts(
	mut commands: Commands,
	present: Res<UrbanizationPresentBullseye>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	budget: Res<UrbanizationHostBudget>,
	spec: Option<Res<UrbanizationStreamSpec>>,
	mut development: DevelopmentIndex,
	mut state: ResMut<UrbanizationPresenterState>,
) {
	if !present.enabled {
		return;
	}
	let Some(region) = keep.region else {
		return;
	};

	if let Some(spec) = spec.as_deref() {
		development.urbanization.noise = spec.noise;
		development.urbanization.kind = spec.kind;
	}

	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};

	let tracked: Vec<(Id, Version)> =
		SpatialIndex::<SelectedUrbanization>::tracked_ids_for(&*development.urbanization, region)
			.into_iter()
			.filter_map(|tracked| {
				let id = tracked.0;
				let version =
					SpatialIndex::<SelectedUrbanization>::version(&*development.urbanization, id)?;
				Some((id, version))
			})
			.collect();

	let wanted: HashSet<Id> = tracked.iter().map(|(id, _)| *id).collect();
	let mut remaining = budget.cells_per_frame;
	for (id, version) in tracked {
		if state.presented_version(id) == Some(version) {
			continue;
		}
		if remaining == 0 {
			continue;
		}
		let Some(selected) = development.urbanization.get(id).cloned() else {
			continue;
		};
		state.present_urbanization(
			&mut commands,
			&mut development,
			id,
			version,
			&selected,
			&lod_ref,
		);
		remaining -= 1;
	}
	state.remove_stale(&mut commands, &wanted);
}

/// Building hosts for urbanization present keep. One cell per frame by default.
///
/// Does not bake padded terrain. Inserts world [`lod::Bullseye`] / [`lod::OpenLattice`]
/// after this plugin if the host must keep vegetation's 2 km outer ring.
pub struct UrbanizationHostPlugin;

impl Plugin for UrbanizationHostPlugin {
	fn build(&self, app: &mut App) {
		register_richmond_development_models_plugin(app);
		if let Some(spec) = app.world().get_resource::<UrbanizationStreamSpec>().copied() {
			let mut config = app.world_mut().resource_mut::<DevelopmentConfig>();
			config.seed = spec.noise.seed.max(0) as u32;
			config.use_urbanization = true;
		}
		app.init_resource::<UrbanizationPresenterState>()
			.init_resource::<UrbanizationHostBudget>()
			.add_systems(
				Update,
				present_urbanization_hosts
					.after(LodGenerateSystems::Drain)
					.after(LodPresentSystems::Produce),
			);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn host_budget_defaults_to_one_cell_per_frame() {
		assert_eq!(UrbanizationHostBudget::default().cells_per_frame, 1);
	}
}
