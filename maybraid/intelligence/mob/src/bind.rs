//! Bind High plants to a roster slot and copy pack affiliations onto the member.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use combat_targeting::CombatTargeting;
use meandering_intelligence::MeanderingIntelligenceUser;
use npc_intelligence::{NpcInstall, NpcIntelligence, Personality};
use poi_intelligence::{PoiIntelligenceUser, PoiInterests};
use tether_intelligence::{TetherIntelligenceUser, TetherObjective};
use threat_intelligence::{
	ThreatId, ThreatIntelligenceUser, ThreatKnowledge, ThreatRegistry, ThreatSubject,
};

use crate::host::{Mob, MobId};
use crate::member::{resolve_host, MemberOf, MobMemberBody, MobSlot};
use crate::roster::{MobAffiliations, MobInterests, MobRoster};
use crate::share::{MobKnowledge, MobSharePolicy};

type Wish<'a> = (
	Entity,
	&'a MobSlot,
	Option<&'a MobId>,
	Option<&'a Transform>,
	Option<&'a GlobalTransform>,
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
	affiliations: Query<'w, 's, &'static MobAffiliations, With<Mob>>,
	policies: Query<'w, 's, &'static MobSharePolicy, With<Mob>>,
	boards: Query<'w, 's, &'static MobKnowledge, With<Mob>>,
	mixers: Query<'w, 's, &'static mut NpcIntelligence>,
	tethers: Query<'w, 's, &'static mut TetherIntelligenceUser>,
	learners: Query<'w, 's, &'static mut PoiIntelligenceUser>,
	meanderers: Query<'w, 's, &'static mut MeanderingIntelligenceUser>,
	brains: Query<
		'w,
		's,
		(
			&'static ThreatIntelligenceUser,
			&'static mut ThreatKnowledge,
			Option<&'static mut CombatTargeting>,
		),
	>,
}

pub(crate) fn bind_mob_members(
	mut commands: Commands,
	time: Option<Res<Time>>,
	registry: Option<Res<ThreatRegistry>>,
	wishes: Query<Wish<'_>, Without<MemberOf>>,
	mut bind: BindWorld,
) {
	let mut claimed: Vec<(Entity, u16)> = Vec::new();
	for (plant, slot, wish_id, transform, global, body, has_mixer) in &wishes {
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
		let Some((install, personality, armed, interests)) = claim_roster_slot(
			&mut bind,
			host,
			plant,
			slot.0,
			wish_id.is_none(),
			transform,
			global,
			body,
		) else {
			continue;
		};
		claimed.push((host, slot.0));

		commands.entity(plant).insert(MemberOf { mob: host, slot: slot.0 });
		if !has_mixer {
			personality.install(&mut commands, plant, install);
		} else {
			MemberTetherRetarget {
				subject: host,
				plant,
				slot: slot.0,
				locked: false,
				personality: Some(personality),
			}
			.apply(&mut bind.mixers, &mut bind.tethers);
			if let Ok(mut learner) = bind.learners.get_mut(plant) {
				learner.interests = interests;
			}
			if let Ok(mut meandering) = bind.meanderers.get_mut(plant) {
				meandering.selection_salt = install.selection_salt;
			}
		}
		if let Ok(pack) = bind.affiliations.get(host) {
			let id = ThreatId(plant.to_bits());
			let mut affiliations = pack.for_member(id);
			if let (Some(time), Some(registry)) = (time.as_deref(), registry.as_deref()) {
				drain_bound_member(
					&mut commands,
					plant,
					host,
					has_mixer,
					armed && personality.spec().combat.is_some(),
					&mut affiliations,
					&mut bind,
					registry,
					time.elapsed_secs(),
				);
			}
			commands.entity(plant).insert((ThreatSubject::new(id), affiliations));
		}
	}
}

pub(crate) fn propagate_mob_membership(
	mut commands: Commands,
	changed_affiliations: Query<(Entity, &MobAffiliations), Changed<MobAffiliations>>,
	changed_interests: ChangedMobInterests,
	members: Query<(Entity, &MemberOf)>,
	rosters: Query<&MobRoster>,
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
	for (host, _interests) in &changed_interests {
		let Ok(roster) = rosters.get(host) else {
			continue;
		};
		for (plant, membership) in &members {
			if membership.mob != host {
				continue;
			}
			let Some(member) = roster.get(membership.slot) else {
				continue;
			};
			if let Ok(mut learner) = learners.get_mut(plant) {
				learner.interests = member.interests.clone();
			}
		}
	}
}

fn claim_roster_slot(
	bind: &mut BindWorld,
	host: Entity,
	plant: Entity,
	slot: u16,
	prefer_global: bool,
	transform: Option<&Transform>,
	global: Option<&GlobalTransform>,
	body: Option<&MobMemberBody>,
) -> Option<(NpcInstall, Personality, bool, PoiInterests)> {
	let Ok(mut roster) = bind.rosters.get_mut(host) else {
		return None;
	};
	let member = roster.get_mut(slot)?;
	if member.entity.is_some_and(|existing| existing != plant) {
		return None;
	}
	let at = if prefer_global {
		global
			.map(GlobalTransform::translation)
			.or_else(|| transform.map(|transform| transform.translation))
			.unwrap_or(member.pose)
	} else {
		transform
			.map(|transform| transform.translation)
			.or_else(|| global.map(GlobalTransform::translation))
			.unwrap_or(member.pose)
	};
	member.entity = Some(plant);
	member.pose = at;
	member.respawn_at = None;
	member.spawn_requested = false;
	let mut install = member.npc_install(host, at, body.map(|body| body.0).unwrap_or_default());
	install.selection_salt = MeanderingIntelligenceUser::salt_for_slot(slot);
	let personality = member.personality;
	let armed = install.armed;
	let interests = member.interests.clone();
	Some((install, personality, armed, interests))
}

fn drain_bound_member(
	commands: &mut Commands,
	plant: Entity,
	host: Entity,
	has_mixer: bool,
	has_combat: bool,
	affiliations: &mut threat_intelligence::Affiliations,
	bind: &mut BindWorld,
	registry: &ThreatRegistry,
	now: f32,
) {
	if !bind.policies.get(host).is_ok_and(|policy| policy.enabled) {
		return;
	}
	let Ok(board) = bind.boards.get(host) else {
		return;
	};
	if has_mixer {
		if let Ok((user, mut knowledge, mut targeting)) = bind.brains.get_mut(plant) {
			board.drain_into(
				plant,
				affiliations,
				user,
				&mut knowledge,
				targeting.as_deref_mut(),
				registry,
				now,
			);
		}
		return;
	}
	let user = ThreatIntelligenceUser::default();
	let mut knowledge = ThreatKnowledge::default();
	let mut targeting = CombatTargeting::default();
	board.drain_into(
		plant,
		affiliations,
		&user,
		&mut knowledge,
		has_combat.then_some(&mut targeting),
		registry,
		now,
	);
	commands.entity(plant).insert(knowledge);
	if has_combat {
		commands.entity(plant).insert(targeting);
	}
}

#[derive(Clone, Copy)]
pub(crate) struct MemberTetherRetarget {
	pub subject: Entity,
	pub plant: Entity,
	pub slot: u16,
	pub locked: bool,
	pub personality: Option<Personality>,
}

impl MemberTetherRetarget {
	pub(crate) fn apply(
		self,
		mixers: &mut Query<&mut NpcIntelligence>,
		tethers: &mut Query<&mut TetherIntelligenceUser>,
	) {
		let idle = self.idle_objective();
		let engaged = self.engaged_objective();
		if let Ok(mut mixer) = mixers.get_mut(self.plant) {
			if let Some(objective) = idle {
				mixer.idle_tether = Some(objective);
			} else if let Some(current) = mixer.idle_tether.as_mut() {
				*current = self.adjust(*current);
			}
			if let Some(objective) = engaged {
				mixer.engaged_tether = Some(objective);
			} else if let Some(current) = mixer.engaged_tether.as_mut() {
				*current = self.adjust(*current);
			}
		}
		if let Ok(mut tether) = tethers.get_mut(self.plant) {
			tether.objective = idle.unwrap_or_else(|| self.adjust(tether.objective));
		}
	}

	fn idle_objective(self) -> Option<TetherObjective> {
		self.personality
			.map(|personality| self.adjust(personality.spec().tether.objective(self.subject)))
	}

	fn engaged_objective(self) -> Option<TetherObjective> {
		self.personality.and_then(|personality| {
			personality
				.spec()
				.engaged_tether_radius
				.map(|radius| self.adjust(TetherObjective::Tether(self.subject, radius)))
		})
	}

	fn adjust(self, objective: TetherObjective) -> TetherObjective {
		let retargeted = objective.with_subject(self.subject);
		if self.locked {
			retargeted.with_lock_standoff(self.slot)
		} else {
			retargeted
		}
	}
}
