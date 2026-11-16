//! Gameplay plugin: register Self, attach ledgers, and honor owner lifetime.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use player::Player;

use crate::activation::{
	LubikanActivationDenied, LubikanActivationFinished, LubikanActivationOutcome,
	LubikanActivationPhase, LubikanActivationReport, LubikanActivationRequest,
	LubikanActivationSession, LubikanActivationSessionId,
};
use crate::catalog::{LubikanCatalog, LubikanSelfConfig};
use crate::change::{LubikanChargeChanged, LubikanChargeReason};
use crate::identity::LubikanAccountId;
use crate::ledger::{
	LubikanAccountBinding, LubikanBodies, LubikanBodyOf, LubikanCharacter, LubikanLedger,
	LubikanOwner,
};
use crate::lifetime::{LubikanOwnerLifetime, LubikanOwnerLifetimeKind};
use crate::{LubikanActivationRefused, LubikanInterfacePlugin, LubikanSystems};

/// Play-session handle for the durable [`LubikanCharacter`] entity.
///
/// The ledger lives on that entity, not on the body. Body replacement
/// re-points [`LubikanBodyOf`]; the character and ledger stay put, so
/// same-frame spends are not cloned through a retiring cache.
#[derive(Resource, Default, Debug)]
pub struct LubikanSessionStore {
	character: Option<Entity>,
	last_body: Option<Entity>,
}

impl LubikanSessionStore {
	pub fn character(&self) -> Option<Entity> {
		self.character
	}
}

/// Live ledger access for one activation session.
///
/// [`Self::close`] mutates the matching ledger at most once and returns the
/// presentation pulse. Heal publishes that pulse; [`apply_activation_reports`]
/// calls the same mutation so a synchronous Self close is not finished again.
#[derive(SystemParam)]
pub(crate) struct LubikanActivationAccess<'w, 's> {
	ledgers: Query<'w, 's, &'static mut LubikanLedger>,
}

impl LubikanActivationAccess<'_, '_> {
	pub fn session_phase(&self, id: LubikanActivationSessionId) -> Option<LubikanActivationPhase> {
		self.ledgers.iter().find_map(|ledger| ledger.activation_phase(id))
	}

	pub fn account_phase(&self, account: LubikanAccountId) -> Option<LubikanActivationPhase> {
		self.ledgers
			.iter()
			.find_map(|ledger| ledger.account(account).map(|owned| owned.activation().phase))
	}

	/// Close the captured session. Mutates at most once; always returns the
	/// [`LubikanActivationFinished`] pulse so the caller can publish it.
	pub fn close(
		&mut self,
		session: LubikanActivationSession,
		outcome: LubikanActivationOutcome,
	) -> LubikanActivationFinished {
		let _ = self.close_session(session.id, outcome);
		LubikanActivationFinished::from_session(session, outcome)
	}

	/// Same mutation as [`Self::close`] when the consumer only has a report.
	/// Already-closed sessions (including a same-frame Self idle reset) are
	/// no-ops.
	pub fn close_session(
		&mut self,
		id: LubikanActivationSessionId,
		outcome: LubikanActivationOutcome,
	) -> bool {
		for mut ledger in self.ledgers.iter_mut() {
			if ledger.account_id_for_session(id).is_some() {
				return ledger.close_activation(id, outcome);
			}
		}
		false
	}
}

/// Registers Self and keeps per-player charge independent of the HUD.
///
/// This plugin is the only place that creates or restores a player's ledger,
/// and the sole owner of [`LubikanActivationRequest`]s: it spends once, emits
/// [`LubikanActivationSession`] or [`LubikanActivationRefused`], and closes
/// accepted sessions through one accessor on the session owner. Heal mutates
/// synchronously and returns the notification; report consumers use the same
/// close so they do not finish an already-closed session. Consumers order after
/// [`LubikanSystems::Owner`] (PreUpdate). Does not depend on Bikanwarda.
pub struct LubikansPlugin;

impl Plugin for LubikansPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LubikanInterfacePlugin>() {
			app.add_plugins(LubikanInterfacePlugin);
		}
		app.init_resource::<LubikanSelfConfig>().init_resource::<LubikanSessionStore>();
		if app.world().get_resource::<LubikanCatalog>().is_none() {
			let config = *app.world().resource::<LubikanSelfConfig>();
			app.insert_resource(LubikanCatalog::with_self(&config));
		}
		app.configure_sets(PreUpdate, LubikanSystems::Owner)
			.configure_sets(Update, LubikanSystems::Activate)
			.add_systems(
				PreUpdate,
				(
					remember_removed_owners,
					attach_new_owners,
					apply_lifetime_notices,
					sync_account_bindings,
				)
					.chain()
					.in_set(LubikanSystems::Owner),
			)
			.add_systems(
				Update,
				(handle_activation_requests, apply_activation_reports)
					.chain()
					.in_set(LubikanSystems::Activate),
			);
		crate::award::configure(app);
		crate::heal::configure(app);
	}
}

fn remember_removed_owners(
	mut players: RemovedComponents<Player>,
	mut owners: RemovedComponents<LubikanOwner>,
	body_of: Query<&LubikanBodyOf>,
	mut store: ResMut<LubikanSessionStore>,
	mut commands: Commands,
) {
	for entity in players.read().chain(owners.read()) {
		if let Ok(of) = body_of.get(entity) {
			store.character = Some(of.character());
			store.last_body = Some(entity);
		}
		commands
			.entity(entity)
			.try_remove::<(LubikanLedger, LubikanOwner, LubikanAccountBinding, LubikanBodyOf)>();
	}
}

fn attach_new_owners(
	added_players: Query<Entity, Added<Player>>,
	added_owners: Query<Entity, Added<LubikanOwner>>,
	body_of: Query<&LubikanBodyOf>,
	mut ledgers: Query<&mut LubikanLedger>,
	orphans: Query<Entity, (With<LubikanCharacter>, Without<LubikanBodies>)>,
	mut store: ResMut<LubikanSessionStore>,
	config: Res<LubikanSelfConfig>,
	mut commands: Commands,
	mut lives: MessageWriter<LubikanOwnerLifetime>,
) {
	let mut added: Vec<Entity> = added_players.iter().collect();
	for entity in &added_owners {
		if !added.contains(&entity) {
			added.push(entity);
		}
	}
	let mut claimed: Vec<Entity> = Vec::new();
	for entity in added {
		if let Ok(of) = body_of.get(entity) {
			store.character = Some(of.character());
			continue;
		}
		if let Ok(ledger) = ledgers.get(entity) {
			let ledger = ledger.clone();
			commands.entity(entity).try_remove::<(LubikanLedger, LubikanAccountBinding)>();
			let binding = LubikanAccountBinding::from_selection(ledger.selection(), entity);
			let character = commands.spawn((LubikanCharacter, ledger, binding)).id();
			claimed.push(character);
			store.character = Some(character);
			commands.entity(entity).insert((LubikanOwner, LubikanBodyOf(character)));
			lives.write(LubikanOwnerLifetime::new(
				None,
				Some(entity),
				LubikanOwnerLifetimeKind::Equip,
			));
			continue;
		}
		let orphan = store
			.character
			.filter(|character| orphans.contains(*character) && !claimed.contains(character))
			.or_else(|| orphans.iter().find(|character| !claimed.contains(character)));
		if let Some(character) = orphan {
			claimed.push(character);
			store.character = Some(character);
			if let Ok(mut ledger) = ledgers.get_mut(character) {
				ledger.apply_lifetime(config.lifetime.on_body_replace);
			}
			let previous = store.last_body;
			commands.entity(entity).insert((LubikanOwner, LubikanBodyOf(character)));
			lives.write(LubikanOwnerLifetime::new(
				previous,
				Some(entity),
				LubikanOwnerLifetimeKind::ReplaceBody,
			));
			continue;
		}
		let ledger = LubikanLedger::with_self(&config);
		let binding = LubikanAccountBinding::from_selection(ledger.selection(), entity);
		let character = commands.spawn((LubikanCharacter, ledger, binding)).id();
		claimed.push(character);
		store.character = Some(character);
		commands.entity(entity).insert((LubikanOwner, LubikanBodyOf(character)));
		lives.write(LubikanOwnerLifetime::new(None, Some(entity), LubikanOwnerLifetimeKind::Equip));
	}
}

fn apply_lifetime_notices(
	mut notices: MessageReader<LubikanOwnerLifetime>,
	mut ledgers: Query<&mut LubikanLedger>,
	characters: Query<Entity, With<LubikanCharacter>>,
	mut store: ResMut<LubikanSessionStore>,
	config: Res<LubikanSelfConfig>,
) {
	for notice in notices.read() {
		if notice.kind != LubikanOwnerLifetimeKind::RestartSession {
			continue;
		}
		let persistence = config.lifetime.on_session_restart;
		for mut ledger in &mut ledgers {
			ledger.apply_lifetime(persistence);
		}
		if store.character.is_some_and(|character| !characters.contains(character)) {
			store.character = None;
		}
	}
}

fn sync_account_bindings(world: &mut World) {
	let mut updates = Vec::new();
	{
		let mut ledgers = world.query_filtered::<
			(Entity, &LubikanLedger, Option<&LubikanBodies>),
			Or<(With<LubikanCharacter>, With<LubikanOwner>)>,
		>();
		for (holder, ledger, bodies) in ledgers.iter(world) {
			let owner = bodies.and_then(LubikanBodies::current).unwrap_or(holder);
			updates
				.push((holder, LubikanAccountBinding::from_selection(ledger.selection(), owner)));
			if let Some(bodies) = bodies {
				for body in bodies.iter() {
					updates.push((
						body,
						LubikanAccountBinding::from_selection(ledger.selection(), body),
					));
				}
			}
		}
	}
	for (entity, binding) in updates {
		if world.get::<LubikanAccountBinding>(entity) != Some(&binding) {
			if let Ok(mut entity) = world.get_entity_mut(entity) {
				entity.insert(binding);
			}
		}
	}
}

fn handle_activation_requests(
	mut requests: MessageReader<LubikanActivationRequest>,
	mut ledgers: Query<&mut LubikanLedger>,
	body_of: Query<&LubikanBodyOf>,
	catalog: Res<LubikanCatalog>,
	mut accepted: MessageWriter<LubikanActivationSession>,
	mut refused: MessageWriter<LubikanActivationRefused>,
	mut changes: MessageWriter<LubikanChargeChanged>,
) {
	for request in requests.read() {
		let holder = body_of
			.get(request.owner)
			.map(LubikanBodyOf::character)
			.unwrap_or(request.owner);
		let Ok(mut ledger) = ledgers.get_mut(holder) else {
			refused.write(LubikanActivationRefused {
				owner: request.owner,
				lubikan: request.lubikan,
				reason: LubikanActivationDenied::UnknownAccount,
			});
			continue;
		};
		match ledger.try_activate(&catalog, request.lubikan) {
			Ok(grant) => {
				accepted.write(grant.session(request.owner));
				changes.write(LubikanChargeChanged::new(
					request.owner,
					grant.account,
					grant.spent,
					LubikanChargeReason::ACTIVATION,
					None,
				));
			}
			Err(reason) => {
				refused.write(LubikanActivationRefused {
					owner: request.owner,
					lubikan: request.lubikan,
					reason,
				});
			}
		}
	}
}

fn apply_activation_reports(
	mut reports: MessageReader<LubikanActivationReport>,
	mut activations: LubikanActivationAccess,
) {
	for report in reports.read() {
		let _ = activations.close_session(report.id, report.outcome);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::activation::LubikanActivationPhase;
	use crate::identity::{LubikanAccountId, LubikanDefinition, LubikanDefinitionId};
	use crate::lifetime::LubikanPersistence;
	use crate::{
		LubikanActivationReport, LubikanActivationRequest, LubikanBodyOf, LubikanCharacter,
		LubikanChargeChanged, LubikanChargeReason,
	};
	use anyhow::{Result, anyhow};
	use damage::{DamagePlugin, Health, Hit};

	fn gameplay() -> App {
		let mut app = App::new();
		app.add_plugins(LubikansPlugin);
		app
	}

	fn spawn_player(app: &mut App) -> Entity {
		app.world_mut().spawn(Player).id()
	}

	fn ledger(app: &App, owner: Entity) -> Result<&LubikanLedger> {
		LubikanBodyOf::ledger(app.world(), owner)
			.ok_or_else(|| anyhow!("missing ledger on {owner}"))
	}

	fn ledger_mut(app: &mut App, owner: Entity) -> Result<Mut<'_, LubikanLedger>> {
		LubikanBodyOf::ledger_mut(app.world_mut(), owner).ok_or_else(|| anyhow!("ledger"))
	}

	#[test]
	fn normal_game_registers_self_without_hud() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let catalog = app.world().resource::<LubikanCatalog>();
		if !catalog.self_definition().is_self() {
			return Err(anyhow!("catalog must register Self"));
		}
		let ledger = ledger(&app, owner)?;
		assert!(ledger.selection().is_self());
		assert_eq!(ledger.equipped().id(), LubikanAccountId::SELF);
		assert_eq!(ledger.equipped().charge().current(), 0.0);
		assert!(app.world().get::<LubikanOwner>(owner).is_some());
		assert!(
			LubikanBodyOf::binding(app.world(), owner).is_some_and(|binding| binding.is_self())
		);
		Ok(())
	}

	#[test]
	fn stable_ledger_projects_damage_binding_onto_the_current_body() -> Result<()> {
		let mut app = gameplay();
		let first = spawn_player(&mut app);
		app.update();
		let character = LubikanBodyOf::holder(app.world(), first);
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(7), 40.0, 2.0);
		let extra_id = {
			let mut ledger = ledger_mut(&mut app, first)?;
			let account = ledger.ensure_owned(extra);
			assert!(ledger.select(Some(account)));
			account
		};
		app.update();
		assert_eq!(
			app.world().get::<LubikanAccountBinding>(first).map(|b| b.account),
			Some(extra_id)
		);
		app.world_mut().entity_mut(first).despawn();
		let second = spawn_player(&mut app);
		app.update();
		assert_eq!(LubikanBodyOf::holder(app.world(), second), character);
		let binding = app
			.world()
			.get::<LubikanAccountBinding>(second)
			.ok_or_else(|| anyhow!("body binding"))?;
		assert_eq!(binding.owner, second);
		assert_eq!(binding.account, extra_id);
		Ok(())
	}

	#[test]
	fn charge_mutations_clamp_and_block_activation() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let cost = app.world().resource::<LubikanCatalog>().self_definition().activation_cost;
		{
			let mut ledger = ledger_mut(&mut app, owner)?;
			let high = ledger.apply_to_equipped(10_000.0);
			if !high.clamped_capacity {
				return Err(anyhow!("capacity must clamp"));
			}
			let low = ledger.apply_to_equipped(-10_000.0);
			if !low.clamped_floor {
				return Err(anyhow!("floor must clamp"));
			}
		}
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		if ledger(&app, owner)?.equipped().activation().phase != LubikanActivationPhase::Idle {
			return Err(anyhow!("empty charge cannot activate"));
		}
		{
			let mut ledger = ledger_mut(&mut app, owner)?;
			ledger.apply_to_equipped(cost);
		}
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		assert_eq!(
			ledger(&app, owner)?.equipped().activation().phase,
			LubikanActivationPhase::Active
		);
		assert_eq!(ledger(&app, owner)?.equipped().charge().current(), 0.0);
		Ok(())
	}

	#[test]
	fn selection_keeps_accounts_distinct() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(3), 40.0, 5.0);
		app.world_mut().resource_mut::<LubikanCatalog>().register(extra);
		let other = {
			let mut ledger = ledger_mut(&mut app, owner)?;
			ledger.apply_to(LubikanAccountId::SELF, 11.0);
			let other = ledger.ensure_owned(extra);
			ledger.apply_to(other, 7.0);
			if !ledger.select(Some(other)) {
				return Err(anyhow!("select other"));
			}
			other
		};
		app.update();
		let ledger = ledger(&app, owner)?;
		assert_eq!(ledger.equipped().id(), other);
		assert_eq!(
			ledger.account(LubikanAccountId::SELF).map(|account| account.charge().current()),
			Some(11.0)
		);
		assert_eq!(ledger.account(other).map(|account| account.charge().current()), Some(7.0));
		assert!(
			LubikanBodyOf::binding(app.world(), owner)
				.is_some_and(|binding| binding.account == other)
		);
		Ok(())
	}

	#[test]
	fn body_replace_retains_charge_and_edited_order_by_default() -> Result<()> {
		let mut app = gameplay();
		let first = spawn_player(&mut app);
		app.update();
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(8), 40.0, 2.0);
		let extra_id = {
			let mut ledger = ledger_mut(&mut app, first)?;
			ledger.apply_to_equipped(18.0);
			let extra_id = ledger.ensure_owned(extra);
			let mut order = crate::LubikanOrder::from_ledger(&ledger);
			assert!(order.move_account(extra_id, -1));
			assert!(ledger.apply_order(&order));
			extra_id
		};
		app.update();
		let character = LubikanBodyOf::holder(app.world(), first);
		assert_ne!(character, first);
		app.world_mut().entity_mut(first).remove::<Player>();
		app.update();
		let second = spawn_player(&mut app);
		app.update();
		assert_eq!(ledger(&app, second)?.equipped().charge().current(), 18.0);
		assert_eq!(
			ledger(&app, second)?.iter().map(|account| account.id()).collect::<Vec<_>>(),
			vec![extra_id, LubikanAccountId::SELF]
		);
		assert!(app.world().get::<LubikanLedger>(first).is_none());
		assert_eq!(LubikanBodyOf::holder(app.world(), second), character);
		assert!(app.world().get::<LubikanCharacter>(character).is_some());
		Ok(())
	}

	#[test]
	fn session_restart_resets_charge_and_keeps_self() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(8), 40.0, 2.0);
		let extra_id = {
			let mut ledger = ledger_mut(&mut app, owner)?;
			let extra_id = ledger.ensure_owned(extra);
			ledger.apply_to(LubikanAccountId::SELF, 11.0);
			ledger.apply_to(extra_id, 20.0);
			if !ledger.select(Some(extra_id)) {
				return Err(anyhow!("select extra before restart"));
			}
			extra_id
		};
		app.update();
		assert_eq!(ledger(&app, owner)?.equipped().id(), extra_id);
		assert_eq!(ledger(&app, owner)?.equipped().charge().current(), 20.0);
		app.world_mut().write_message(LubikanOwnerLifetime::new(
			None,
			None,
			LubikanOwnerLifetimeKind::RestartSession,
		));
		app.update();
		let ledger = ledger(&app, owner)?;
		assert!(ledger.contains(LubikanAccountId::SELF));
		assert!(ledger.contains(extra_id));
		assert_eq!(ledger.equipped().charge().current(), 0.0);
		assert_eq!(
			ledger.account(LubikanAccountId::SELF).map(|account| account.charge().current()),
			Some(0.0)
		);
		assert_eq!(
			app.world().resource::<LubikanSelfConfig>().lifetime.on_session_restart,
			LubikanPersistence::ResetCharge
		);
		Ok(())
	}

	#[test]
	fn marked_owner_without_player_still_gets_self() -> Result<()> {
		let mut app = gameplay();
		let owner = app.world_mut().spawn(LubikanOwner).id();
		app.update();
		assert_eq!(ledger(&app, owner)?.equipped().id(), LubikanAccountId::SELF);
		Ok(())
	}

	#[test]
	fn replacing_a_marked_owner_restores_charge() -> Result<()> {
		let mut app = gameplay();
		let first = app.world_mut().spawn(LubikanOwner).id();
		app.update();
		{
			let mut ledger = ledger_mut(&mut app, first)?;
			ledger.apply_to_equipped(14.0);
		}
		app.update();
		app.world_mut().entity_mut(first).remove::<LubikanOwner>();
		app.update();
		let second = app.world_mut().spawn(LubikanOwner).id();
		app.update();
		assert_eq!(ledger(&app, second)?.equipped().charge().current(), 14.0);
		Ok(())
	}

	#[test]
	fn pause_preserves_charge_activation_and_selection() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(4), 40.0, 5.0);
		app.world_mut().resource_mut::<LubikanCatalog>().register(extra);
		let cost = app.world().resource::<LubikanCatalog>().self_definition().activation_cost;
		let other = {
			let mut ledger = ledger_mut(&mut app, owner)?;
			ledger.apply_to(LubikanAccountId::SELF, cost);
			let other = ledger.ensure_owned(extra);
			ledger.apply_to(other, 8.0);
			other
		};
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		{
			let mut ledger = ledger_mut(&mut app, owner)?;
			if !ledger.select(Some(other)) {
				return Err(anyhow!("switch after activate"));
			}
		}
		app.update();
		app.update();
		let snapshot = ledger(&app, owner)?;
		assert_eq!(
			snapshot
				.account(LubikanAccountId::SELF)
				.map(|account| account.activation().phase),
			Some(LubikanActivationPhase::Active)
		);
		assert_eq!(
			snapshot
				.account(LubikanAccountId::SELF)
				.map(|account| account.charge().current()),
			Some(0.0)
		);
		assert_eq!(snapshot.equipped().id(), other);
		assert_eq!(snapshot.account(other).map(|account| account.charge().current()), Some(8.0));
		assert_eq!(
			snapshot.account(other).map(|account| account.activation().phase),
			Some(LubikanActivationPhase::Idle)
		);
		Ok(())
	}

	#[test]
	fn activation_handoff_spends_once_and_completes_the_captured_account() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(5), 40.0, 5.0);
		app.world_mut().resource_mut::<LubikanCatalog>().register(extra);
		let cost = app.world().resource::<LubikanCatalog>().self_definition().activation_cost;
		let other = {
			let mut ledger = ledger_mut(&mut app, owner)?;
			ledger.apply_to(LubikanAccountId::SELF, cost);
			let other = ledger.ensure_owned(extra);
			ledger.apply_to(other, 6.0);
			other
		};
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		let session = app
			.world_mut()
			.resource_mut::<Messages<LubikanActivationSession>>()
			.drain()
			.next()
			.ok_or_else(|| anyhow!("accepted session"))?;
		assert_eq!(session.owner, owner);
		assert_eq!(session.account, LubikanAccountId::SELF);
		let spend = app
			.world_mut()
			.resource_mut::<Messages<LubikanChargeChanged>>()
			.drain()
			.find(|change| change.reason == LubikanChargeReason::ACTIVATION)
			.ok_or_else(|| anyhow!("activation spend"))?;
		assert_eq!(spend.lubikan, LubikanAccountId::SELF);
		assert_eq!(spend.amount, -cost);
		{
			let mut ledger = ledger_mut(&mut app, owner)?;
			if !ledger.select(Some(other)) {
				return Err(anyhow!("switch after activate"));
			}
		}
		app.world_mut().write_message(LubikanActivationReport::completed(session.id));
		app.update();
		{
			let snapshot = ledger(&app, owner)?;
			assert_eq!(
				snapshot
					.account(LubikanAccountId::SELF)
					.map(|account| account.activation().phase),
				Some(LubikanActivationPhase::Idle),
				"Self complete idles in the same close as heal so a later activate can spend"
			);
			assert_eq!(
				snapshot.account(other).map(|account| account.activation().phase),
				Some(LubikanActivationPhase::Idle)
			);
			assert_eq!(
				snapshot.account(other).map(|account| account.charge().current()),
				Some(6.0)
			);
		}
		app.world_mut().write_message(LubikanActivationReport::completed(session.id));
		app.update();
		assert_eq!(
			ledger(&app, owner)?
				.account(LubikanAccountId::SELF)
				.map(|account| account.activation().phase),
			Some(LubikanActivationPhase::Idle)
		);
		assert_eq!(
			ledger(&app, owner)?
				.account(LubikanAccountId::SELF)
				.map(|account| account.charge().current()),
			Some(0.0)
		);
		Ok(())
	}

	#[test]
	fn non_self_report_completes_once_and_stays_completed() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(5), 40.0, 5.0);
		app.world_mut().resource_mut::<LubikanCatalog>().register(extra);
		let other = {
			let mut ledger = ledger_mut(&mut app, owner)?;
			let other = ledger.ensure_owned(extra);
			ledger.apply_to(other, extra.activation_cost);
			other
		};
		app.world_mut()
			.write_message(LubikanActivationRequest::for_account(owner, other));
		app.update();
		let session = app
			.world_mut()
			.resource_mut::<Messages<LubikanActivationSession>>()
			.drain()
			.next()
			.ok_or_else(|| anyhow!("accepted session"))?;
		assert_eq!(session.account, other);
		app.world_mut().write_message(LubikanActivationReport::completed(session.id));
		app.update();
		assert_eq!(
			ledger(&app, owner)?.account(other).map(|account| account.activation().phase),
			Some(LubikanActivationPhase::Completed)
		);
		app.world_mut().write_message(LubikanActivationReport::completed(session.id));
		app.update();
		assert_eq!(
			ledger(&app, owner)?.account(other).map(|account| account.activation().phase),
			Some(LubikanActivationPhase::Completed),
			"a repeated completed report must not reset a non-Self session"
		);
		Ok(())
	}

	#[test]
	fn cancelled_report_idles_once() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		let cost = app.world().resource::<LubikanCatalog>().self_definition().activation_cost;
		{
			let mut ledger = ledger_mut(&mut app, owner)?;
			ledger.apply_to_equipped(cost);
		}
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		let session = app
			.world_mut()
			.resource_mut::<Messages<LubikanActivationSession>>()
			.drain()
			.next()
			.ok_or_else(|| anyhow!("accepted session"))?;
		app.world_mut().write_message(LubikanActivationReport::cancelled(session.id));
		app.update();
		assert_eq!(
			ledger(&app, owner)?.equipped().activation().phase,
			LubikanActivationPhase::Idle
		);
		assert_eq!(ledger(&app, owner)?.equipped().charge().current(), 0.0);
		app.world_mut().write_message(LubikanActivationReport::cancelled(session.id));
		app.update();
		assert_eq!(
			ledger(&app, owner)?.equipped().activation().phase,
			LubikanActivationPhase::Idle
		);
		Ok(())
	}

	#[test]
	fn denial_emits_refused_and_does_not_spend() -> Result<()> {
		let mut app = gameplay();
		let owner = spawn_player(&mut app);
		app.update();
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		let refused = app
			.world_mut()
			.resource_mut::<Messages<LubikanActivationRefused>>()
			.drain()
			.next()
			.ok_or_else(|| anyhow!("refused"))?;
		assert_eq!(refused.reason, LubikanActivationDenied::InsufficientCharge);
		assert_eq!(ledger(&app, owner)?.equipped().charge().current(), 0.0);
		assert_eq!(
			ledger(&app, owner)?.equipped().activation().phase,
			LubikanActivationPhase::Idle
		);
		Ok(())
	}

	#[test]
	fn activate_and_award_same_frame_as_death_survive_replace() -> Result<()> {
		let mut app = gameplay();
		app.add_plugins(DamagePlugin);
		let owner = spawn_player(&mut app);
		app.world_mut().entity_mut(owner).insert(Health { current: 20.0, max: 100.0 });
		app.update();
		let cost = app.world().resource::<LubikanCatalog>().self_definition().activation_cost;
		{
			let mut ledger = ledger_mut(&mut app, owner)?;
			ledger.apply_to_equipped(cost);
		}
		let npc = app.world_mut().spawn(Health { current: 40.0, max: 40.0 }).id();
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.world_mut().write_message(Hit {
			target: npc,
			source: Some(owner),
			amount: 10.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		app.world_mut().write_message(Hit {
			target: owner,
			source: None,
			amount: 200.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		app.update();
		assert_eq!(
			ledger(&app, owner)?.equipped().charge().current(),
			10.0,
			"spend and the same-frame combat award must both land before retirement"
		);
		app.world_mut().entity_mut(owner).remove::<Player>();
		app.update();
		let replacement = spawn_player(&mut app);
		app.update();
		assert_eq!(
			ledger(&app, replacement)?.equipped().charge().current(),
			10.0,
			"replacement must not refund activation or drop the award"
		);
		Ok(())
	}
}
