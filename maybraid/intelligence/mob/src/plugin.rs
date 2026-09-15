use bevy::prelude::*;
use combat_targeting::CombatTargetingSystems;
use firearm_intelligence::FirearmIntelligenceSystems;
use poi_intelligence::PoiSystems;
use routing_intelligence::RoutingSystems;
use tether_intelligence::TetherSystems;
use threat_intelligence::{ThreatRegistry, ThreatSystems};
use threat_management_intelligence::ThreatManagementSystems;

use crate::bind::{bind_mob_members, propagate_mob_membership};
use crate::host::MobIdAlloc;
use crate::lifecycle::{queue_downed_member_deaths, respawn_mob_members, write_back_mob_roster};
use crate::lock::{
	apply_mob_tether_subjects, expire_mob_tether_locks, forget_mob_tether_lock_when_leaving,
	lock_mobs_on_poi_arrival,
};
use crate::prey::{prey_tracks_subject, start_prey_browse};
use crate::roster::MobMemberNeeded;
use crate::share::{share_mob_targets, share_mob_threats};
use crate::travel::travel_mobs;

/// Pack brain cadence. Bind runs before NPC mixers see a new plant.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MobSystems {
	Bind,
	Propagate,
	Share,
	Writeback,
	Respawn,
	Travel,
	Lock,
	Prey,
}

pub struct MobIntelligencePlugin;

impl Plugin for MobIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<MobIdAlloc>()
			.init_resource::<ThreatRegistry>()
			.add_message::<MobMemberNeeded>()
			.configure_sets(
				Update,
				(
					MobSystems::Bind,
					MobSystems::Propagate,
					MobSystems::Writeback,
					MobSystems::Respawn,
					MobSystems::Lock,
				)
					.chain()
					.before(TetherSystems::Write)
					.before(PoiSystems::Select),
			)
			.configure_sets(
				Update,
				// After routing Plan+Write. Do not order before Tether Write:
				// that set already runs before Plan, which would cycle.
				MobSystems::Travel.after(RoutingSystems::Write),
			)
			.configure_sets(
				Update,
				MobSystems::Prey
					.after(PoiSystems::Select)
					.before(PoiSystems::Drive)
					.before(MobSystems::Travel),
			)
			.configure_sets(
				Update,
				MobSystems::Share
					.after(ThreatSystems::Discover)
					.after(MobSystems::Propagate)
					.before(ThreatManagementSystems::Select)
					.before(ThreatSystems::Export),
			)
			.add_systems(PostStartup, bind_mob_members)
			.add_systems(
				Update,
				(
					bind_mob_members.in_set(MobSystems::Bind),
					propagate_mob_membership.in_set(MobSystems::Propagate),
					share_mob_threats.in_set(MobSystems::Share),
					share_mob_targets
						.after(ThreatManagementSystems::Select)
						.before(FirearmIntelligenceSystems::Spotting)
						.before(CombatTargetingSystems::Rank),
					(queue_downed_member_deaths, write_back_mob_roster)
						.chain()
						.in_set(MobSystems::Writeback),
					respawn_mob_members.in_set(MobSystems::Respawn),
					travel_mobs.in_set(MobSystems::Travel),
					(start_prey_browse, prey_tracks_subject).chain().in_set(MobSystems::Prey),
					(
						expire_mob_tether_locks,
						lock_mobs_on_poi_arrival,
						forget_mob_tether_lock_when_leaving,
						apply_mob_tether_subjects,
					)
						.chain()
						.in_set(MobSystems::Lock),
				),
			);
	}
}
