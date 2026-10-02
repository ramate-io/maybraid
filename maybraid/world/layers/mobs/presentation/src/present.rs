//! Mob cell presenter, surface fit, High LOD pulse, and unsubscribed teardown.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{Id, Version};
use lod::lod_ref::LodRef;
use lod::presentation::RegionPresenter;
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};
use lod::{
	LodNode, LodNodePose, LodPresentSystems, LodRefreshDomain, LodSceneRefreshAabb,
	LodSceneRefreshRegion, LodViewer,
};
use mob_scenes::MobScene;
use mob_intelligence::MemberOf;
use mob_layer_model::{MobCell, MobGenerationSystems, MobIndex};
use terrain_layer_model::TerrainView;
use layer_stack::{ActiveGenerationMode, ModeSubscription};
use urbanization_layer_model::UrbanModel;

use crate::MobPresent;

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

#[derive(Resource, Default)]
pub(crate) struct MobPresenterState {
	presented: HashMap<Id, PresentedCell>,
	pending_despawn: VecDeque<Vec<Entity>>,
}

struct PresentedCell {
	version: Version,
	entities: Vec<Entity>,
	hidden: bool,
}

/// Host the presenter spawned for this generated cell.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentedMobCell(pub Id);

#[derive(Component, Clone, Copy, Debug)]
pub struct MobCellRoot;

#[derive(Component, Clone, Copy, Debug)]
pub struct MobGroupRoot;

impl MobPresenterState {
	fn retire(&mut self, id: Id) -> Option<PresentedCell> {
		self.presented.remove(&id)
	}

	fn queue_remove(&mut self, id: Id) {
		if let Some(presented) = self.presented.remove(&id) {
			self.pending_despawn.push_back(presented.entities);
		}
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.presented.keys().copied().collect()
	}

	#[cfg(test)]
	pub(crate) fn presents(&self, id: Id) -> bool {
		self.presented.contains_key(&id)
	}

	#[cfg(test)]
	pub(crate) fn insert_presented(&mut self, id: Id, entities: Vec<Entity>) {
		self.presented.insert(
			id,
			PresentedCell { version: Version(1), entities, hidden: false },
		);
	}

	#[cfg(test)]
	pub(crate) fn push_pending(&mut self, entities: Vec<Entity>) {
		self.pending_despawn.push_back(entities);
	}
}

#[derive(SystemParam)]
pub struct MobPresenter<'w, 's, G: UrbanModel> {
	commands: Commands<'w, 's>,
	state: ResMut<'w, MobPresenterState>,
	surface: TerrainView<'w, 's, G>,
}

impl<G: UrbanModel> RegionPresenter<MobCell, MobIndex> for MobPresenter<'_, '_, G> {
	fn presented_version(&self, id: Id) -> Option<Version> {
		self.state.presented.get(&id).map(|entry| entry.version)
	}

	fn handle(&mut self, id: Id, version: Version, cell: &MobCell, _lod_ref: &LodRef) {
		if let Some(previous) = self.state.retire(id) {
			for entity in &previous.entities {
				self.commands.entity(*entity).insert(Visibility::Hidden);
			}
			self.state.pending_despawn.push_back(previous.entities);
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
			}
		}
		self.state
			.presented
			.insert(id, PresentedCell { version, entities, hidden: false });
	}

	fn hide(&mut self, id: Id) {
		if let Some(entry) = self.state.presented.get_mut(&id) {
			entry.hidden = true;
			for entity in &entry.entities {
				self.commands.entity(*entity).insert(Visibility::Hidden);
			}
		}
	}

	fn is_hidden(&self, id: Id) -> bool {
		self.state.presented.get(&id).is_some_and(|entry| entry.hidden)
	}

	fn presented_ids(&self) -> Vec<Id> {
		self.state.presented.keys().copied().collect()
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

pub fn fit_mob_hosts_to_surface<G: UrbanModel>(
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

/// Teardown registration [`crate::MobPresentationCore`] uses. Tests call this
/// instead of adding the plugins that pull physics and scenes.
pub(crate) fn install_mob_cell_teardown<G: UrbanModel>(app: &mut App) {
	app.init_resource::<MobPresenterState>().add_systems(
		Update,
		(retire_mob_presenters::<G>, retire_mob_cells_on_mode_change)
			.after(MobGenerationSystems)
			.before(LodPresentSystems::Produce),
	)
	.add_systems(Last, drain_retired_mob_cells);
}

/// Queue every presented cell when the active generation mode changes.
///
/// Both modes may subscribe, so [`retire_mob_presenters`] does not run.
/// Ordered `.after(MobGenerationSystems).before(LodPresentSystems::Produce)`
/// so a cell written on the entering frame presents after this retire.
pub(crate) fn retire_mob_cells_on_mode_change(
	mode: Res<State<ActiveGenerationMode>>,
	mut presented: ResMut<MobPresenterState>,
) {
	if !mode.is_changed() {
		return;
	}
	for id in presented.presented_ids() {
		presented.queue_remove(id);
	}
}

/// While the active mode is not subscribed, remove every presented cell and
/// drain `pending_despawn`, every frame, exactly as the generation stream did.
///
/// Ordered `.after(MobGenerationSystems).before(LodPresentSystems::Produce)`.
/// No present-state system runs in that window, so the frame timing matches today.
pub(crate) fn retire_mob_presenters<G: UrbanModel>(
	subscription: ModeSubscription<(G, MobPresent)>,
	mut presented: ResMut<MobPresenterState>,
) {
	if subscription.active() {
		return;
	}
	for id in presented.presented_ids() {
		presented.queue_remove(id);
	}
}

/// Combat, threat, and mob systems queue inserts on hosts and members through
/// `PostUpdate`. Despawn in `Last` so those commands still find their targets.
pub(crate) fn drain_retired_mob_cells(
	mut presented: ResMut<MobPresenterState>,
	members: Query<(Entity, &MemberOf)>,
	mut commands: Commands,
) {
	while let Some(entities) = presented.pending_despawn.pop_front() {
		let hosts: HashSet<Entity> = entities.iter().copied().collect();
		for (entity, member) in &members {
			if hosts.contains(&member.mob) {
				commands.entity(entity).try_despawn();
			}
		}
		for entity in entities {
			commands.entity(entity).try_despawn();
		}
	}
}
