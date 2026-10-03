//! Placed scenes, group roots, surface fit, and the High LOD pulse.

use std::collections::HashSet;
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use lod::gen::{Id, Version};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};
use lod::{
	update_lod_host_levels, LodGenerateSystems, LodNode, LodNodePose, LodPresentCullPlugin,
	LodPresentPlugin, LodPresentSystems, LodRefreshDomain, LodRefreshSystems, LodSceneRefreshAabb,
	LodSceneRefreshRegion, LodSceneRefreshRegionPlugin, LodViewer,
};
use lod_gimme::GimmeLodSceneRefreshPlugin;
use mob_layer_model::MobCellPresented;
use mob_layer_presentation::{MobPresenterState, PresentedMobCell};
use mob_scenes::{MobLodRefreshMode, MobScene, MobSceneSystems};
use terrain_layer_model::{TerrainModel, TerrainView};

use crate::index::{MobCell, MobIndex};
use crate::plugin::MobGroupsPlugin;
use crate::stream::MobLodChan;

/// Half-extent of the High produce cube. Sized a margin past the 200 m High sphere.
pub const MOB_HIGH_LOD_REFRESH_RADIUS: f32 = 250.0;
pub const MOB_HIGH_LOD_REFRESH_INTERVAL: Duration = Duration::from_millis(250);
pub const MOB_HIGH_LOD_RECONCILE_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, Default)]
pub struct MobHighLodChan;

#[derive(Resource, Default)]
pub struct MobHighLodRegion;

impl MobHighLodRegion {
	pub fn region_at(center: Vec3) -> Aabb3d {
		let half = Vec3::splat(MOB_HIGH_LOD_REFRESH_RADIUS);
		Aabb3d::from_min_max(center - half, center + half)
	}
}

impl LodRefreshRegions for MobHighLodRegion {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if lod_ref.previous_transform.translation == lod_ref.current_transform.translation {
			LodRefreshRegionsStatus::Unchanged
		} else {
			LodRefreshRegionsStatus::Changed(Self::region_at(lod_ref.current_transform.translation))
		}
	}
}

#[derive(Component, Clone, Copy, Debug)]
pub struct MobCellRoot;

#[derive(Component, Clone, Copy, Debug)]
pub struct MobGroupRoot;

#[derive(SystemParam)]
pub struct BarkingPresenter<'w, 's, G: TerrainModel> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, MobPresenterState>,
	surface: TerrainView<'w, 's, G>,
	presented: MessageWriter<'w, MobCellPresented>,
}

impl<G: TerrainModel> RegionPresenter<MobCell, MobIndex> for BarkingPresenter<'_, '_, G> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented_version(id)
	}

	fn handle(&mut self, id: Id, version: Version, cell: &MobCell, _lod_ref: &LodRef) {
		if let Some(previous) = self.state.retire(id) {
			for entity in &previous {
				self.commands.entity(*entity).insert(Visibility::Hidden);
			}
			self.state.queue(previous);
		}
		let cell_index = cell.extent.index();
		let cell_root = self
			.commands
			.spawn((
				Name::new(format!("mob-cell {},{}", cell_index.0, cell_index.1)),
				MobCellRoot,
				Transform::default(),
				Visibility::default(),
			))
			.id();
		let mut entities = vec![cell_root];
		let mut hosts = Vec::new();
		for group in &cell.groups {
			let group_root = self
				.commands
				.spawn((
					Name::new(format!("{:?} mob group", group.kind)),
					MobGroupRoot,
					ChildOf(cell_root),
					Transform::default(),
					Visibility::default(),
				))
				.id();
			for placed in &group.mobs {
				let mut transform = placed.transform;
				let xz = Vec2::new(transform.translation.x, transform.translation.z);
				transform.translation.y = self.surface.height_or_fallback(xz);
				let mob = placed.scene.spawn(&mut self.commands, transform);
				self.commands.entity(mob).insert((ChildOf(group_root), PresentedMobCell(id)));
				entities.push(mob);
				hosts.push(mob);
			}
		}
		self.state.remember(id, version, entities);
		self.presented.write(MobCellPresented { id, hosts });
	}

	fn hide(&mut self, id: Id) {
		for entity in self.state.hide(id) {
			self.commands.entity(entity).insert(Visibility::Hidden);
		}
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.state.is_hidden(id)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented_ids()
	}

	fn remove_stale(&mut self, wanted: &HashSet<Id>) {
		let stale: Vec<_> = self
			.state
			.presented_ids()
			.into_iter()
			.filter(|id| !wanted.contains(id))
			.collect();
		for id in stale {
			self.state.queue_remove(id);
		}
	}
}

pub fn fit_mob_hosts_to_surface<G: TerrainModel>(
	surface: TerrainView<G>,
	mut hosts: Query<&mut Transform, (With<MobScene>, Changed<Transform>)>,
) {
	for mut transform in &mut hosts {
		let xz = Vec2::new(transform.translation.x, transform.translation.z);
		let y = surface.height_or_fallback(xz);
		if y.is_finite() && (transform.translation.y - y).abs() > 1e-3 {
			transform.translation.y = y;
		}
	}
}

pub fn pulse_mob_high_lod(
	nodes: Query<&LodNodePose, (With<LodNode>, With<LodViewer>)>,
	mut refresh: MessageWriter<LodSceneRefreshRegion<MobHighLodChan>>,
	mut bus: MessageWriter<LodSceneRefreshAabb>,
) {
	let regions = nodes.iter().map(|pose| MobHighLodRegion::region_at(pose.current.translation));
	let union = regions.reduce(|a, b| Aabb3d::from_min_max(a.min.min(b.min), a.max.max(b.max)));
	if let Some(region) = union {
		refresh.write(LodSceneRefreshRegion::new(region));
		bus.write(LodSceneRefreshAabb { region, domain: LodRefreshDomain::of::<MobHighLodChan>() });
	}
}

pub fn install_barking_presentation<G: TerrainModel>(app: &mut App) {
	// Indexed must be visible when MobScenesPlugin builds. Guarding
	// MobGroupsPlugin would hide an assembler that already added groups
	// under FullScan.
	app.insert_resource(MobLodRefreshMode::Indexed);
	app.add_plugins(MobGroupsPlugin);
	app.add_message::<MobCellPresented>();
	// Cull only queues hosts; Last must despawn them and their members.
	mob_layer_presentation::install_mob_cell_teardown(app);
	app.init_resource::<MobPresenterState>()
		.add_plugins(LodPresentPlugin::<
			MobCell,
			MobIndex,
			BarkingPresenter<'_, '_, G>,
			MobLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentCullPlugin::<
			MobCell,
			MobIndex,
			BarkingPresenter<'_, '_, G>,
			MobLodChan,
		>::default())
		.add_plugins(LodSceneRefreshRegionPlugin::<
			MobHighLodRegion,
			With<LodViewer>,
			MobHighLodChan,
		>::default())
		.add_plugins(GimmeLodSceneRefreshPlugin::<MobScene, MobHighLodChan, With<LodViewer>>::default())
		.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain));
	app.add_systems(Update, fit_mob_hosts_to_surface::<G>.in_set(MobSceneSystems::Surface))
		.add_systems(
			Update,
			pulse_mob_high_lod
				.run_if(on_timer(MOB_HIGH_LOD_REFRESH_INTERVAL))
				.in_set(LodRefreshSystems::ProduceRegions),
		)
		.add_systems(
			Update,
			update_lod_host_levels::<MobScene, (), With<LodViewer>>
				.run_if(on_timer(MOB_HIGH_LOD_RECONCILE_INTERVAL))
				.in_set(LodRefreshSystems::UpdateLevels),
		);
}
