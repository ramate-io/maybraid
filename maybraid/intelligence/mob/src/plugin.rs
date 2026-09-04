use bevy::prelude::*;
use poi_intelligence::PoiSystems;
use tether_intelligence::TetherSystems;

use crate::bind::{bind_mob_members, propagate_mob_membership};
use crate::host::MobIdAlloc;
use crate::lifecycle::{respawn_mob_members, write_back_mob_roster};
use crate::roster::MobMemberNeeded;
use crate::travel::travel_mobs;

/// Pack brain cadence. Bind runs before NPC mixers see a new plant.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MobSystems {
	Bind,
	Propagate,
	Writeback,
	Respawn,
	Travel,
}

pub struct MobIntelligencePlugin;

impl Plugin for MobIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<MobIdAlloc>()
			.add_message::<MobMemberNeeded>()
			.configure_sets(
				Update,
				(
					MobSystems::Bind,
					MobSystems::Propagate,
					MobSystems::Writeback,
					MobSystems::Respawn,
					MobSystems::Travel,
				)
					.chain()
					.before(TetherSystems::Write)
					.before(PoiSystems::Select),
			)
			.add_systems(PostStartup, bind_mob_members)
			.add_systems(
				Update,
				(
					bind_mob_members.in_set(MobSystems::Bind),
					propagate_mob_membership.in_set(MobSystems::Propagate),
					write_back_mob_roster.in_set(MobSystems::Writeback),
					respawn_mob_members.in_set(MobSystems::Respawn),
					travel_mobs.in_set(MobSystems::Travel),
				),
			);
	}
}
