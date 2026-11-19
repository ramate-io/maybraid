//! Runtime patch queue for world-model adapters.

use bevy::prelude::*;
use mob_scenes::MobScenesPlugin;

use crate::MobGroup;

#[derive(Resource, Default)]
pub struct PendingMobGroups {
	groups: Vec<MobGroup>,
}

impl PendingMobGroups {
	pub fn push(&mut self, group: MobGroup) {
		self.groups.push(group);
	}

	pub fn is_empty(&self) -> bool {
		self.groups.is_empty()
	}
}

pub struct MobGroupsPlugin;

impl Plugin for MobGroupsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MobScenesPlugin>() {
			app.add_plugins(MobScenesPlugin);
		}
		app.init_resource::<PendingMobGroups>()
			.add_systems(Update, spawn_pending_groups);
	}
}

/// Plugin for homeostatic mob density management.
/// Generic over the player marker component P.
pub struct HomeostaticMobsPlugin<P> {
	_marker: std::marker::PhantomData<P>,
}

impl<P> Default for HomeostaticMobsPlugin<P> {
	fn default() -> Self {
		Self { _marker: std::marker::PhantomData }
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HomeostaticMobSystems {
	Lifecycle,
	Assign,
}

impl<P: Component> Plugin for HomeostaticMobsPlugin<P> {
	fn build(&self, app: &mut App) {
		use crate::homeostatic::{
			apply_homeostatic_retreat, despawn_distant_homeostatic_mobs, homeostatic_mobs_enabled,
			release_homeostatic_retreat, reset_homeostatic_mobs, track_player_homeostatic_cell,
			trigger_homeostatic_retreat, HomeostaticMobConfig, HomeostaticMobLifecycle,
			PlayerHomeostaticCell,
		};
		app.init_resource::<HomeostaticMobConfig>()
			.init_resource::<HomeostaticMobLifecycle>()
			.init_resource::<PlayerHomeostaticCell>()
			.configure_sets(
				Update,
				(
					HomeostaticMobSystems::Lifecycle.after(lod::hcsg::HcsgSessionRestarted),
					HomeostaticMobSystems::Assign,
				)
					.chain(),
			)
			.configure_sets(
				Update,
				HomeostaticMobSystems::Assign
					.after(poi_intelligence::PoiSystems::Select)
					.before(mob_intelligence::MobSystems::Prey)
					.before(poi_intelligence::PoiSystems::Drive),
			)
			.add_systems(Update, reset_homeostatic_mobs.in_set(HomeostaticMobSystems::Lifecycle))
			.add_systems(
				Update,
				(
					track_player_homeostatic_cell::<P>,
					release_homeostatic_retreat::<P>,
					trigger_homeostatic_retreat::<P>,
					apply_homeostatic_retreat,
					despawn_distant_homeostatic_mobs::<P>,
				)
					.chain()
					.in_set(HomeostaticMobSystems::Assign)
					.run_if(homeostatic_mobs_enabled),
			);
	}
}

fn spawn_pending_groups(mut commands: Commands, mut pending: ResMut<PendingMobGroups>) {
	for group in std::mem::take(&mut pending.groups) {
		group.spawn(&mut commands);
	}
}
