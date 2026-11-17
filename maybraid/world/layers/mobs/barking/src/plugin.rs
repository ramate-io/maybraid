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

impl<P: Component> Plugin for HomeostaticMobsPlugin<P> {
	fn build(&self, app: &mut App) {
		use crate::homeostatic::{
			apply_homeostatic_retreat, cleanup_on_gameplay_disable,
			despawn_distant_homeostatic_mobs, release_homeostatic_retreat,
			trigger_homeostatic_retreat, track_player_homeostatic_cell, HomeostaticGameplayState,
			HomeostaticMobConfig, PlayerHomeostaticCell,
		};

		app.init_resource::<HomeostaticMobConfig>()
			.init_resource::<PlayerHomeostaticCell>()
			.init_resource::<HomeostaticGameplayState>()
			.add_systems(
				Update,
				(
					cleanup_on_gameplay_disable,
					track_player_homeostatic_cell::<P>,
					release_homeostatic_retreat::<P>,
					trigger_homeostatic_retreat::<P>,
					apply_homeostatic_retreat,
					despawn_distant_homeostatic_mobs::<P>,
				)
					.chain(),
			);
	}
}

fn spawn_pending_groups(mut commands: Commands, mut pending: ResMut<PendingMobGroups>) {
	for group in std::mem::take(&mut pending.groups) {
		group.spawn(&mut commands);
	}
}
