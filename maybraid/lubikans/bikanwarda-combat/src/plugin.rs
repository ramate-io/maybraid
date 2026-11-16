//! Attach, sync, drive, and retire Ward / Enraged combat on live summons.
//!
//! Does not depend on `combat-hud` or `maybraid-world`.

use attacks::{
	AttackEndReason, AttackEnded, AttackInstance, AttackRetired, AttackSystems, AttackUser,
	AttacksPlugin, BeginAttack, CancelAttacks,
};
use bevy::prelude::*;
use bikanwarda_attacks::BikanwardaAttacksPlugin;
use combat_targeting::{CombatTargeting, CombatTargetingPlugin, CombatTargetingSystems};
use lubikans::{
	LubikanCatalog, LubikanSummon, LubikanSummonCatalog, LubikanSummonDeparture, LubikanSystems,
};
use threat_intelligence::Affiliations;

use crate::catalog::register_bikanwarda_summons;
use crate::policy::{
	refresh_policy, sync_summon_targeting, BikanwardaSummonConfig, BikanwardaSummonForm,
	BikanwardaSummonPolicy, PolicyView,
};

/// Systems that keep form policy current. Hosts order encounter scripts after
/// [`Self::Drive`] if they write extra `BeginAttack`s in the same frame.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BikanwardaSummonSystems {
	Attach,
	Sync,
	Drive,
	Retire,
}

/// Combat policy plugin. Adds the attack runtime and authored recipes when the
/// host has not already done so. Does not add `LubikansPlugin` or a HUD.
pub struct BikanwardaLubikanPlugin;

impl Plugin for BikanwardaLubikanPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<CombatTargetingPlugin>() {
			app.add_plugins(CombatTargetingPlugin);
		}
		if !app.is_plugin_added::<AttacksPlugin>() {
			app.add_plugins(AttacksPlugin);
		}
		if !app.is_plugin_added::<BikanwardaAttacksPlugin>() {
			app.add_plugins(BikanwardaAttacksPlugin { placeholders: false });
		}
		if app.world().get_resource::<Messages<LubikanSummonDeparture>>().is_none() {
			app.add_message::<LubikanSummonDeparture>();
		}
		app.init_resource::<BikanwardaSummonConfig>()
			.configure_sets(PreUpdate, BikanwardaSummonSystems::Attach)
			.configure_sets(
				Update,
				(
					BikanwardaSummonSystems::Sync.after(CombatTargetingSystems::Rank),
					BikanwardaSummonSystems::Drive
						.after(BikanwardaSummonSystems::Sync)
						.before(AttackSystems::Command),
					BikanwardaSummonSystems::Retire.after(LubikanSystems::Summon),
				),
			)
			.add_systems(Startup, register_catalogs)
			.add_systems(PreUpdate, attach_summon_policy.in_set(BikanwardaSummonSystems::Attach))
			.add_systems(Update, sync_summon_policy.in_set(BikanwardaSummonSystems::Sync))
			.add_systems(Update, drive_summon_attacks.in_set(BikanwardaSummonSystems::Drive))
			.add_systems(Update, retire_summon_attacks.in_set(BikanwardaSummonSystems::Retire))
			.add_systems(Update, advance_rotation_on_complete.after(AttackSystems::Advance));
	}
}

fn register_catalogs(
	catalog: Option<ResMut<LubikanCatalog>>,
	recipes: Option<ResMut<LubikanSummonCatalog>>,
) {
	let (Some(mut catalog), Some(mut recipes)) = (catalog, recipes) else {
		return;
	};
	register_bikanwarda_summons(&mut catalog, &mut recipes);
}

pub(crate) fn attach_summon_policy(world: &mut World) {
	let mut pending = Vec::new();
	{
		let mut query = world.query::<(Entity, &LubikanSummon, Option<&BikanwardaSummonPolicy>)>();
		for (entity, live, policy) in query.iter(world) {
			if policy.is_some() {
				continue;
			}
			let Some(form) = BikanwardaSummonForm::from_definition(live.definition) else {
				continue;
			};
			pending.push((entity, form, live.owner));
		}
	}
	for (entity, form, owner) in pending {
		world.entity_mut(entity).insert((
			form,
			BikanwardaSummonPolicy::new(form, owner),
			AttackUser::default(),
			CombatTargeting::default(),
		));
	}
}

#[allow(clippy::type_complexity)]
fn sync_summon_policy(
	time: Res<Time>,
	mut summons: Query<(Entity, &LubikanSummon, &mut BikanwardaSummonPolicy, &mut CombatTargeting)>,
	owners: Query<&CombatTargeting, Without<BikanwardaSummonPolicy>>,
	affiliations: Query<&Affiliations>,
	live: Query<Entity>,
	mut cancels: MessageWriter<CancelAttacks>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	for (summon, live_summon, mut policy, mut targeting) in &mut summons {
		if policy.owner != live_summon.owner {
			cancels.write(CancelAttacks::all(summon));
			commands.entity(summon).insert(AttackRetired);
			*policy = BikanwardaSummonPolicy::new(policy.form, live_summon.owner);
			targeting.memory.clear();
			targeting.active.clear();
			targeting.ranked.clear();
			targeting.clear_engagement();
		}
		let exists = |entity: Entity| live.get(entity).is_ok();
		let owner_targeting = owners.get(policy.owner).ok();
		sync_summon_targeting(
			policy.form,
			policy.owner,
			owner_targeting,
			&mut targeting,
			exists,
			now,
		);
		let candidate_affiliations: Vec<_> = targeting
			.active
			.keys()
			.filter_map(|entity| affiliations.get(*entity).ok().map(|row| (*entity, row.clone())))
			.collect();
		let view = PolicyView {
			form: policy.form,
			summon,
			owner: policy.owner,
			owner_affiliations: affiliations.get(policy.owner).ok(),
			now,
		};
		refresh_policy(
			&mut policy,
			&targeting,
			owner_targeting,
			view,
			&candidate_affiliations,
			exists,
		);
		if let Some(selected) = policy.selected {
			targeting.engage(selected);
		} else {
			targeting.clear_engagement();
		}
	}
}

#[allow(clippy::type_complexity)]
fn drive_summon_attacks(
	config: Res<BikanwardaSummonConfig>,
	summons: Query<(Entity, &BikanwardaSummonPolicy, &AttackUser, Option<&CombatTargeting>)>,
	instances: Query<&AttackInstance>,
	catalog: Res<attacks::AttackCatalog>,
	mut begins: MessageWriter<BeginAttack>,
) {
	if !config.auto_cast {
		return;
	}
	for (summon, policy, user, targeting) in &summons {
		if policy.selected.is_none() {
			continue;
		}
		if !user.ready(policy.next_attack) {
			continue;
		}
		if exclusive_busy(summon, &instances, &catalog) {
			continue;
		}
		if targeting.is_some_and(|targeting| {
			policy.selected.is_some_and(|selected| targeting.contact(selected).is_none())
		}) {
			continue;
		}
		let mut begin = BeginAttack::new(summon, policy.next_attack)
			.with_allies(policy.excluded_victims.clone());
		if let Some(target) = policy.selected {
			begin = begin.with_target(target);
		}
		begins.write(begin);
	}
}

fn exclusive_busy(
	summon: Entity,
	instances: &Query<&AttackInstance>,
	catalog: &attacks::AttackCatalog,
) -> bool {
	instances.iter().any(|instance| {
		instance.owner == summon
			&& catalog
				.get(instance.definition)
				.is_some_and(|definition| definition.requires_exclusive_owner())
	})
}

fn retire_summon_attacks(
	mut departed: MessageReader<LubikanSummonDeparture>,
	mut cancels: MessageWriter<CancelAttacks>,
	mut commands: Commands,
) {
	for notice in departed.read() {
		let Some(summon) = notice.summon else {
			continue;
		};
		cancels.write(CancelAttacks::all(summon));
		if let Ok(mut entity) = commands.get_entity(summon) {
			entity.insert(AttackRetired);
		}
	}
}

fn advance_rotation_on_complete(
	mut ended: MessageReader<AttackEnded>,
	mut policies: Query<&mut BikanwardaSummonPolicy>,
) {
	for event in ended.read() {
		if event.reason != AttackEndReason::Completed {
			continue;
		}
		let Ok(mut policy) = policies.get_mut(event.owner) else {
			continue;
		};
		if event.definition == policy.next_attack {
			policy.advance_attack();
		}
	}
}
