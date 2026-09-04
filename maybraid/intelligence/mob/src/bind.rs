//! Bind High plants to a roster slot and copy pack tables onto the member.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use npc_intelligence::NpcIntelligence;
use poi_intelligence::PoiIntelligenceUser;
use tether_intelligence::TetherIntelligenceUser;
use threat_intelligence::{ThreatId, ThreatSubject};

use crate::host::{Mob, MobId};
use crate::member::{resolve_host, MemberOf, MobMemberBody, MobSlot};
use crate::roster::{MobAffiliations, MobInterests, MobRoster};

type Wish<'a> = (
	Entity,
	&'a MobSlot,
	Option<&'a MobId>,
	Option<&'a Transform>,
	Option<&'a MobMemberBody>,
	Has<NpcIntelligence>,
);

type ChangedMobInterests<'w, 's> =
	Query<'w, 's, (Entity, &'static MobInterests), (With<Mob>, Changed<MobInterests>)>;

#[derive(SystemParam)]
pub(crate) struct BindWorld<'w, 's> {
	child_of: Query<'w, 's, &'static ChildOf>,
	hosts: Query<'w, 's, (Entity, &'static MobId), With<Mob>>,
	mobs: Query<'w, 's, (), With<Mob>>,
	rosters: Query<'w, 's, &'static mut MobRoster>,
	interests: Query<'w, 's, &'static MobInterests, With<Mob>>,
	affiliations: Query<'w, 's, &'static MobAffiliations, With<Mob>>,
	mixers: Query<'w, 's, &'static mut NpcIntelligence>,
	tethers: Query<'w, 's, &'static mut TetherIntelligenceUser>,
	learners: Query<'w, 's, &'static mut PoiIntelligenceUser>,
}

pub(crate) fn bind_mob_members(
	mut commands: Commands,
	wishes: Query<Wish<'_>, Without<MemberOf>>,
	mut bind: BindWorld,
) {
	let mut claimed: Vec<(Entity, u16)> = Vec::new();
	for (plant, slot, wish_id, transform, body, has_mixer) in &wishes {
		let Some(host) =
			resolve_host(plant, wish_id.copied(), &bind.child_of, &bind.hosts, &bind.mobs)
		else {
			continue;
		};
		if claimed
			.iter()
			.any(|(claimed_host, claimed_slot)| *claimed_host == host && *claimed_slot == slot.0)
		{
			continue;
		}
		let Ok(mut roster) = bind.rosters.get_mut(host) else {
			continue;
		};
		let Some(member) = roster.get_mut(slot.0) else {
			continue;
		};
		if member.entity.is_some_and(|existing| existing != plant) {
			continue;
		}

		let at = transform.map(|transform| transform.translation).unwrap_or(member.pose);
		member.entity = Some(plant);
		member.pose = at;
		member.respawn_at = None;
		member.spawn_requested = false;
		let install = member.npc_install(
			host,
			at,
			body.map(|body| body.0).unwrap_or_default(),
			bind.interests
				.get(host)
				.map(|interests| interests.0.clone())
				.unwrap_or_default(),
		);
		let personality = member.personality;
		claimed.push((host, slot.0));

		commands.entity(plant).insert(MemberOf { mob: host, slot: slot.0 });
		if !has_mixer {
			personality.install(&mut commands, plant, install);
		} else {
			retarget_member_tether(host, plant, &mut bind.mixers, &mut bind.tethers);
			if let Ok(mut learner) = bind.learners.get_mut(plant) {
				if let Ok(host_interests) = bind.interests.get(host) {
					learner.interests = host_interests.0.clone();
				}
			}
		}
		if let Ok(pack) = bind.affiliations.get(host) {
			let id = ThreatId(plant.to_bits());
			commands.entity(plant).insert((ThreatSubject::new(id), pack.for_member(id)));
		}
	}
}

pub(crate) fn propagate_mob_membership(
	mut commands: Commands,
	changed_affiliations: Query<(Entity, &MobAffiliations), Changed<MobAffiliations>>,
	changed_interests: ChangedMobInterests,
	members: Query<(Entity, &MemberOf)>,
	mut learners: Query<&mut PoiIntelligenceUser>,
) {
	for (host, affiliations) in &changed_affiliations {
		for (plant, membership) in &members {
			if membership.mob != host {
				continue;
			}
			let id = ThreatId(plant.to_bits());
			commands
				.entity(plant)
				.insert((ThreatSubject::new(id), affiliations.for_member(id)));
		}
	}
	for (host, interests) in &changed_interests {
		for (plant, membership) in &members {
			if membership.mob != host {
				continue;
			}
			if let Ok(mut learner) = learners.get_mut(plant) {
				learner.interests = interests.0.clone();
			}
		}
	}
}

pub(crate) fn retarget_member_tether(
	subject: Entity,
	plant: Entity,
	mixers: &mut Query<&mut NpcIntelligence>,
	tethers: &mut Query<&mut TetherIntelligenceUser>,
) {
	if let Ok(mut mixer) = mixers.get_mut(plant) {
		if let Some(idle) = mixer.idle_tether.as_mut() {
			*idle = idle.with_subject(subject);
		}
		if let Some(engaged) = mixer.engaged_tether.as_mut() {
			*engaged = engaged.with_subject(subject);
		}
	}
	if let Ok(mut tether) = tethers.get_mut(plant) {
		tether.objective = tether.objective.with_subject(subject);
	}
}
