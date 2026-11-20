//! Target-selection tests. These do not assert area-damage Hits.
//!
//! Victim filtering lives in [`super::victims`].

use anyhow::{Result, anyhow};
use attacks::{AttackEnded, AttackInstance, BeginAttack, CancelAttacks};
use bevy::prelude::*;
use combat_targeting::TargetSource;
use threat_intelligence::{AffiliationStrength, Affiliations, ThreatGroupId, ThreatId};

use crate::test_support::{
	has_attack_user, observe, policy_app, policy_of, spawn_owner, spawn_summon, step,
};
use crate::{
	AUTHORED_ATTACKS, BikanwardaSummonForm, ENRAGED_DEFINITION_ID, PolicyConflict,
	WARD_DEFINITION_ID,
};

fn attach(app: &mut App, owner: Entity, form: BikanwardaSummonForm) -> Result<Entity> {
	let summon = spawn_summon(app, owner, form, Vec3::new(0.0, 0.0, 2.0));
	step(app, 0.05);
	if !has_attack_user(app, summon) {
		return Err(anyhow!("attach must add AttackUser for {:?}", form));
	}
	policy_of(app, summon).ok_or_else(|| anyhow!("attach must add inspectable policy"))?;
	Ok(summon)
}

#[test]
fn ward_selects_owner_enemy_with_contact_not_owner_or_ally() -> Result<()> {
	let mut app = policy_app(false);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let raider = app.world_mut().spawn(Name::new("raider")).id();
	let ally = app.world_mut().spawn(Name::new("ally")).id();
	observe(&mut app, owner, raider, Vec3::new(0.0, 0.0, -4.0), TargetSource::ENEMYSHIP);
	observe(&mut app, owner, ally, Vec3::new(1.0, 0.0, -4.0), TargetSource::ALLY);
	observe(&mut app, owner, owner, Vec3::ZERO, TargetSource::ENEMYSHIP);
	let summon = attach(&mut app, owner, BikanwardaSummonForm::Ward)?;
	let policy = policy_of(&app, summon).ok_or_else(|| anyhow!("policy"))?;
	if policy.selected != Some(raider) {
		return Err(anyhow!("Ward must select the owner's enemy, got {:?}", policy.selected));
	}
	if policy.admitted_targets.contains(&owner) || policy.admitted_targets.contains(&ally) {
		return Err(anyhow!("Ward must not admit owner/ally, got {:?}", policy.admitted_targets));
	}
	if !policy.excluded_victims.contains(&owner) || !policy.excluded_victims.contains(&ally) {
		return Err(anyhow!("Ward allies list must include owner and ally"));
	}
	Ok(())
}

#[test]
fn ward_does_not_select_affiliation_only_hostility() -> Result<()> {
	let mut app = policy_app(false);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let animal = app.world_mut().spawn(Name::new("animal")).id();
	let group = ThreatGroupId::group(30);
	let mut owner_aff = Affiliations::with_self(ThreatId(1));
	owner_aff.antagonize(group, AffiliationStrength::permanent(1.0));
	let mut animal_aff = Affiliations::default();
	animal_aff.join(group, AffiliationStrength::permanent(1.0));
	app.world_mut().entity_mut(owner).insert(owner_aff);
	app.world_mut().entity_mut(animal).insert(animal_aff);
	observe(&mut app, owner, animal, Vec3::new(0.0, 0.0, -3.0), TargetSource::SPOTTING);
	let summon = attach(&mut app, owner, BikanwardaSummonForm::Ward)?;
	let policy = policy_of(&app, summon).ok_or_else(|| anyhow!("policy"))?;
	if policy.selected == Some(animal) {
		return Err(anyhow!("affiliation-only hostility must not become a second enemy list"));
	}
	if !policy.conflicts.iter().any(|(entity, conflict)| {
		*entity == animal && *conflict == PolicyConflict::AffiliationHostileWithoutTargeting
	}) {
		return Err(anyhow!("conflict overlay should record affiliation-only hostility"));
	}
	Ok(())
}

#[test]
fn enraged_can_select_summoner_when_contact_exists() -> Result<()> {
	let mut app = policy_app(false);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let raider = app.world_mut().spawn(Name::new("raider")).id();
	observe(&mut app, owner, raider, Vec3::new(2.0, 0.0, -3.0), TargetSource::ENEMYSHIP);
	let summon = attach(&mut app, owner, BikanwardaSummonForm::Enraged)?;
	observe(&mut app, summon, owner, Vec3::ZERO, TargetSource::NONE);
	step(&mut app, 0.05);
	let policy = policy_of(&app, summon).ok_or_else(|| anyhow!("policy"))?;
	if !policy.admitted_targets.contains(&owner) {
		return Err(anyhow!("Enraged must admit the summoner when contact exists"));
	}
	if policy.excluded_victims.contains(&owner) {
		return Err(anyhow!("Enraged must not list the summoner as an ally exclusion"));
	}
	Ok(())
}

#[test]
fn enraged_does_not_fabricate_summoner_contact() -> Result<()> {
	let mut app = policy_app(false);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let summon = attach(&mut app, owner, BikanwardaSummonForm::Enraged)?;
	let policy = policy_of(&app, summon).ok_or_else(|| anyhow!("policy"))?;
	if policy.admitted_targets.contains(&owner) || policy.selected == Some(owner) {
		return Err(anyhow!("missing summoner contact must not be fabricated"));
	}
	Ok(())
}

#[test]
fn missing_contact_and_retired_target_clear_selection() -> Result<()> {
	let mut app = policy_app(false);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let raider = app.world_mut().spawn(Name::new("raider")).id();
	observe(&mut app, owner, raider, Vec3::new(0.0, 0.0, -4.0), TargetSource::ENEMYSHIP);
	let summon = attach(&mut app, owner, BikanwardaSummonForm::Ward)?;
	if policy_of(&app, summon).and_then(|policy| policy.selected) != Some(raider) {
		return Err(anyhow!("expected raider selected before despawn"));
	}
	app.world_mut().entity_mut(raider).despawn();
	step(&mut app, 0.05);
	let policy = policy_of(&app, summon).ok_or_else(|| anyhow!("policy"))?;
	if policy.selected.is_some() || policy.admitted_targets.contains(&raider) {
		return Err(anyhow!("retired target must not stay selected, got {:?}", policy));
	}
	Ok(())
}

#[test]
fn owner_change_rebinds_ward_protection() -> Result<()> {
	let mut app = policy_app(false);
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	let next_owner = spawn_owner(&mut app, Vec3::new(3.0, 0.0, 0.0));
	let raider = app.world_mut().spawn(Name::new("raider")).id();
	observe(&mut app, owner, raider, Vec3::new(0.0, 0.0, -4.0), TargetSource::ENEMYSHIP);
	let summon = attach(&mut app, owner, BikanwardaSummonForm::Ward)?;
	app.world_mut()
		.entity_mut(summon)
		.get_mut::<lubikans::LubikanSummon>()
		.ok_or_else(|| anyhow!("summon"))?
		.owner = next_owner;
	observe(&mut app, next_owner, raider, Vec3::new(1.0, 0.0, -4.0), TargetSource::ENEMYSHIP);
	step(&mut app, 0.05);
	let policy = policy_of(&app, summon).ok_or_else(|| anyhow!("policy"))?;
	if policy.owner != next_owner {
		return Err(anyhow!("policy owner must follow LubikanSummon.owner"));
	}
	if !policy.excluded_victims.contains(&next_owner) {
		return Err(anyhow!("Ward must protect the new owner"));
	}
	if policy.excluded_victims.contains(&owner) {
		return Err(anyhow!("previous owner must not stay on the ally list"));
	}
	Ok(())
}

#[test]
fn both_forms_begin_the_three_authored_attacks() -> Result<()> {
	for form in [BikanwardaSummonForm::Ward, BikanwardaSummonForm::Enraged] {
		let mut app = policy_app(true);
		let owner = spawn_owner(&mut app, Vec3::ZERO);
		let raider = app.world_mut().spawn(Name::new("raider")).id();
		observe(&mut app, owner, raider, Vec3::new(0.0, 0.0, -4.0), TargetSource::ENEMYSHIP);
		if form == BikanwardaSummonForm::Enraged {
			observe(&mut app, owner, owner, Vec3::ZERO, TargetSource::NONE);
		}
		let summon = attach(&mut app, owner, form)?;
		for expected in AUTHORED_ATTACKS {
			{
				let mut policy = app
					.world_mut()
					.get_mut::<crate::BikanwardaSummonPolicy>(summon)
					.ok_or_else(|| anyhow!("policy"))?;
				policy.next_attack = expected;
			}
			step(&mut app, 0.05);
			let live = {
				let mut query = app.world_mut().query::<&AttackInstance>();
				query.iter(app.world()).map(|instance| instance.definition).collect::<Vec<_>>()
			};
			if !live.contains(&expected) {
				let begins: Vec<_> = app
					.world_mut()
					.resource_mut::<Messages<BeginAttack>>()
					.drain()
					.map(|begin| begin.definition)
					.collect();
				return Err(anyhow!(
					"{form:?} must begin {}, got live={live:?} begins={begins:?}",
					expected.as_str()
				));
			}
			app.world_mut().write_message(CancelAttacks::all(summon));
			step(&mut app, 0.05);
			let _ = app.world_mut().resource_mut::<Messages<AttackEnded>>().drain();
		}
		assert_eq!(
			form.definition_id(),
			match form {
				BikanwardaSummonForm::Ward => WARD_DEFINITION_ID,
				BikanwardaSummonForm::Enraged => ENRAGED_DEFINITION_ID,
			}
		);
	}
	Ok(())
}

#[test]
fn funded_shared_ward_account_spawns_the_owned_combat_policy() -> Result<()> {
	let mut app = App::new();
	app.insert_resource(crate::test_support::short_params());
	app.insert_resource(crate::BikanwardaSummonConfig { auto_cast: false });
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		lubikans::LubikansPlugin,
		crate::BikanwardaLubikanPlugin,
	));
	app.finish();
	let owner = spawn_owner(&mut app, Vec3::ZERO);
	app.update();
	bikanwarda_lubikans::equip_ward(app.world_mut(), owner)?;
	let account = bikanwarda_lubikans::BIKANWARDA_WARD_ACCOUNT;
	{
		let mut ledger = lubikans::LubikanBodyOf::ledger_mut(app.world_mut(), owner)
			.ok_or_else(|| anyhow!("stable owner ledger"))?;
		let _ = ledger.apply_to(account, bikanwarda_lubikans::ward_definition().activation_cost);
	}
	let enemy = app.world_mut().spawn(Name::new("enemy")).id();
	observe(&mut app, owner, enemy, Vec3::NEG_Z * 4.0, TargetSource::ENEMYSHIP);
	app.world_mut()
		.write_message(lubikans::LubikanActivationRequest::equipped(owner));
	step(&mut app, 0.05);
	step(&mut app, 0.05);
	let world = app.world_mut();
	let mut summons = world.query::<(&lubikans::LubikanSummon, &crate::BikanwardaSummonPolicy)>();
	let (summon, policy) = summons.iter(world).next().ok_or_else(|| anyhow!("Ward summon"))?;
	assert_eq!(summon.owner, owner);
	assert_eq!(summon.account, account);
	assert_eq!(policy.form, BikanwardaSummonForm::Ward);
	assert_eq!(policy.selected, Some(enemy));
	assert!(policy.excluded_victims.contains(&owner));
	let ledger = lubikans::LubikanBodyOf::ledger(world, owner)
		.ok_or_else(|| anyhow!("stable ledger after spend"))?;
	assert_eq!(ledger.equipped().charge().current(), 0.0);
	Ok(())
}
