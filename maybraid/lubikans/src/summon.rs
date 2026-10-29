//! Summon activation consumer: spawn and retire owned instances.
//!
//! Consumes non-Self [`LubikanActivationSession`]s from the existing Activate
//! owner. Does not spend charge or call [`crate::LubikanLedger::try_activate`].
//! Self sessions stay with [`crate::LubikanSystems::Heal`].
//!
//! Ownership, admission, session lifetime, and typed departure live here.
//! Capsule meshes and fixture materials are registered by
//! [`install_fixture_summon`], not by the consumer.
//!
//! # Open policies ([#1135](https://github.com/ramate-io/maybraid/issues/1135))
//!
//! Count limits, lifetime, despawn, targeting, who can summon, Self overlap,
//! dismissal, placement, and refund-on-reject are **not agreed** in the issue
//! or its comments. Every knob below is a labeled PLACEHOLDER default.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use damage::{Died, Downed, Health, ResponsibleOwner};

use crate::LubikanSystems;
use crate::activation::{
	LubikanActivationFinished, LubikanActivationOutcome, LubikanActivationPhase,
	LubikanActivationReport, LubikanActivationSession, LubikanActivationSessionId,
};
use crate::change::{LubikanChargeReason, LubikanChargeRequest};
use crate::identity::{LubikanAccountId, LubikanDefinition, LubikanDefinitionId};
use crate::ledger::{LubikanAccountBinding, LubikanOwner};
use crate::lifetime::{LubikanOwnerLifetime, LubikanOwnerLifetimeKind};
use crate::plugin::LubikanActivationAccess;

mod fixture;

pub use fixture::install_fixture_summon;

/// PLACEHOLDER fixture catalog id. Not Bikanwarda; content crates pick their own.
pub const FIXTURE_SUMMON_DEFINITION_ID: LubikanDefinitionId =
	LubikanDefinitionId::from_raw(0xFFFF_FF01);

/// PLACEHOLDER fixture capacity. Balancing should change [`LubikanSummonConfig`].
pub const PLACEHOLDER_SUMMON_CAPACITY: f32 = 100.0;

/// PLACEHOLDER fixture activation cost. Spent by Activate, not this consumer.
pub const PLACEHOLDER_SUMMON_ACTIVATION_COST: f32 = 25.0;

/// PLACEHOLDER lifetime in game-time seconds. `0` means no timeout.
pub const PLACEHOLDER_SUMMON_LIFETIME_SECS: f32 = 20.0;

/// PLACEHOLDER concurrent live summons per owner.
pub const PLACEHOLDER_SUMMON_MAX_CONCURRENT: u32 = 1;

/// PLACEHOLDER forward offset from the owner, meters.
pub const PLACEHOLDER_SUMMON_FORWARD_OFFSET: f32 = 2.0;

/// PLACEHOLDER fixture health. Combat policies are [#1137](https://github.com/ramate-io/maybraid/issues/1137).
pub const PLACEHOLDER_SUMMON_HEALTH: f32 = 50.0;

/// PLACEHOLDER capsule radius for the fixture mesh, meters.
pub const PLACEHOLDER_SUMMON_RADIUS: f32 = 0.35;

/// PLACEHOLDER capsule cylinder length, meters. Total height is length + 2×radius.
pub const PLACEHOLDER_SUMMON_CAPSULE_LENGTH: f32 = 0.90;

/// Catalog row for the fixture summon. Tests and examples register this;
/// a normal Self-only game does not.
pub fn fixture_summon_definition() -> LubikanDefinition {
	LubikanDefinition::new(
		FIXTURE_SUMMON_DEFINITION_ID,
		PLACEHOLDER_SUMMON_CAPACITY,
		PLACEHOLDER_SUMMON_ACTIVATION_COST,
	)
}

/// Pause / menu gate. World copies `WorldGameplayEnabled` onto this; when
/// `false`, lifetime freezes without dismissing the instance.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LubikanSummonEnabled(pub bool);

impl Default for LubikanSummonEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// Why spawn was rejected or an instance retired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LubikanSummonDepartureReason {
	Timeout,
	OwnerRetired,
	WorldReset,
	Dismissed,
	InvalidPlacement,
	Interrupted,
	LimitReached,
	MissingRecipe,
	BlockedBySelf,
	/// Captured session closed without a combat death (external cancel).
	Cancelled,
}

/// PLACEHOLDER invalid-spawn handling. Charge was already spent by Activate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LubikanSummonInvalidPlacement {
	/// Refund the catalog cost through Award and cancel the captured session.
	RefundAndCancel,
	/// Keep the spend and cancel the captured session. Spawn nothing.
	KeepSpendAndCancel,
}

/// PLACEHOLDER targeting. Ward / Enraged policies are #1137.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LubikanSummonTargeting {
	/// No combat AI or victim filter.
	Passive,
}

/// PLACEHOLDER who may consume a summon session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LubikanSummonAuthority {
	/// Any [`LubikanOwner`] whose non-Self account just paid.
	AnyOwner,
}

/// Configurable summon policies. Defaults are labeled placeholders.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct LubikanSummonConfig {
	pub max_concurrent: u32,
	pub lifetime_secs: f32,
	pub forward_offset: f32,
	pub health: f32,
	/// PLACEHOLDER: refund spent charge when spawn is rejected.
	pub invalid_placement: LubikanSummonInvalidPlacement,
	/// PLACEHOLDER: keep the activation session Active while the instance lives.
	pub session_follows_instance: bool,
	/// PLACEHOLDER: changing equipped selection does not dismiss.
	pub dismiss_on_select_change: bool,
	/// PLACEHOLDER: a live Self heal does not block a summon session.
	pub block_while_self_active: bool,
	/// PLACEHOLDER: owner death / replace / session restart retire owned summons.
	pub retire_on_owner_lifetime: bool,
	/// PLACEHOLDER: owner must be living (not dead / downed) to spawn.
	pub require_living_owner: bool,
	pub targeting: LubikanSummonTargeting,
	pub authority: LubikanSummonAuthority,
}

impl Default for LubikanSummonConfig {
	fn default() -> Self {
		Self {
			max_concurrent: PLACEHOLDER_SUMMON_MAX_CONCURRENT,
			lifetime_secs: PLACEHOLDER_SUMMON_LIFETIME_SECS,
			forward_offset: PLACEHOLDER_SUMMON_FORWARD_OFFSET,
			health: PLACEHOLDER_SUMMON_HEALTH,
			invalid_placement: LubikanSummonInvalidPlacement::RefundAndCancel,
			session_follows_instance: true,
			dismiss_on_select_change: false,
			block_while_self_active: false,
			retire_on_owner_lifetime: true,
			require_living_owner: true,
			targeting: LubikanSummonTargeting::Passive,
			authority: LubikanSummonAuthority::AnyOwner,
		}
	}
}

impl LubikanSummonConfig {
	pub fn max_concurrent(self) -> u32 {
		self.max_concurrent.max(1)
	}

	/// `None` means the instance lives until dismissed or the owner retires.
	pub fn lifetime_secs(self) -> Option<f32> {
		if self.lifetime_secs.is_finite() && self.lifetime_secs > 0.0 {
			Some(self.lifetime_secs)
		} else {
			None
		}
	}

	pub fn forward_offset(self) -> f32 {
		if self.forward_offset.is_finite() {
			self.forward_offset
		} else {
			PLACEHOLDER_SUMMON_FORWARD_OFFSET
		}
	}

	pub fn health(self) -> f32 {
		if self.health.is_finite() { self.health.max(1.0) } else { PLACEHOLDER_SUMMON_HEALTH }
	}
}

/// How a recipe is drawn. Production GLB requirements are on the variant docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LubikanSummonPresentation {
	/// Bevy `Capsule3d` placeholder. **Not** a shipped Blender asset.
	///
	/// A production GLB replacing this should:
	/// - sit at the entity origin with +Y up and +Z forward
	/// - fit an approximate capsule of radius
	///   [`PLACEHOLDER_SUMMON_RADIUS`] and cylinder
	///   [`PLACEHOLDER_SUMMON_CAPSULE_LENGTH`] (about 1.6 m tall)
	/// - keep materials self-contained (no Bikanwarda-only textures)
	/// - treat departure glow / fade as [#1138](https://github.com/ramate-io/maybraid/issues/1138)
	/// - omit authored `.blend` files from this crate
	PlaceholderCapsule,
}

/// Lubikan-supplied spawn recipe. Missing recipes reject the session.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LubikanSummonRecipe {
	pub definition: LubikanDefinitionId,
	pub health: f32,
	pub lifetime_secs: f32,
	pub presentation: LubikanSummonPresentation,
}

impl LubikanSummonRecipe {
	pub fn fixture() -> Self {
		Self {
			definition: FIXTURE_SUMMON_DEFINITION_ID,
			health: PLACEHOLDER_SUMMON_HEALTH,
			lifetime_secs: PLACEHOLDER_SUMMON_LIFETIME_SECS,
			presentation: LubikanSummonPresentation::PlaceholderCapsule,
		}
	}

	pub fn health(self) -> f32 {
		if self.health.is_finite() { self.health.max(1.0) } else { PLACEHOLDER_SUMMON_HEALTH }
	}

	pub fn lifetime_secs(self) -> Option<f32> {
		if self.lifetime_secs.is_finite() && self.lifetime_secs > 0.0 {
			Some(self.lifetime_secs)
		} else {
			None
		}
	}
}

/// Registered recipes. Empty in a Self-only game.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct LubikanSummonCatalog {
	recipes: Vec<LubikanSummonRecipe>,
}

impl LubikanSummonCatalog {
	pub fn get(&self, id: LubikanDefinitionId) -> Option<LubikanSummonRecipe> {
		self.recipes.iter().copied().find(|recipe| recipe.definition == id)
	}

	pub fn register(&mut self, recipe: LubikanSummonRecipe) {
		if let Some(existing) =
			self.recipes.iter_mut().find(|entry| entry.definition == recipe.definition)
		{
			*existing = recipe;
		} else {
			self.recipes.push(recipe);
		}
	}

	pub fn with_fixture() -> Self {
		let mut catalog = Self::default();
		catalog.register(LubikanSummonRecipe::fixture());
		catalog
	}

	pub fn iter(&self) -> impl Iterator<Item = LubikanSummonRecipe> + '_ {
		self.recipes.iter().copied()
	}
}

/// Stable id for one spawned instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LubikanSummonInstanceId(u64);

impl LubikanSummonInstanceId {
	pub fn mint() -> Self {
		static NEXT: AtomicU64 = AtomicU64::new(1);
		Self(NEXT.fetch_add(1, Ordering::Relaxed))
	}

	pub const fn from_raw(raw: u64) -> Self {
		Self(raw)
	}

	pub const fn raw(self) -> u64 {
		self.0
	}
}

/// Live owned summon. Independent of the owner's later equipped selection.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct LubikanSummon {
	pub owner: Entity,
	pub account: LubikanAccountId,
	pub definition: LubikanDefinitionId,
	pub session: LubikanActivationSessionId,
	pub instance: LubikanSummonInstanceId,
	pub elapsed: f32,
	pub lifetime: f32,
	/// Applied activation spend captured on the session (negative).
	pub spent: f32,
}

impl LubikanSummon {
	pub fn session_ref(self) -> LubikanActivationSession {
		LubikanActivationSession::new(self.owner, self.account, self.definition, self.session)
			.with_spent(self.spent)
	}

	pub fn timed_out(self) -> bool {
		self.lifetime > 0.0 && self.elapsed >= self.lifetime
	}
}

/// Typed departure. Distinct from [`Died`] — do not emit kill rewards here.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LubikanSummonDeparture {
	/// `None` when spawn was rejected before an entity existed.
	pub summon: Option<Entity>,
	pub owner: Entity,
	pub account: LubikanAccountId,
	pub definition: LubikanDefinitionId,
	pub session: LubikanActivationSessionId,
	pub reason: LubikanSummonDepartureReason,
}

impl LubikanSummonDeparture {
	pub fn from_live(
		live: LubikanSummon,
		summon: Entity,
		reason: LubikanSummonDepartureReason,
	) -> Self {
		Self {
			summon: Some(summon),
			owner: live.owner,
			account: live.account,
			definition: live.definition,
			session: live.session,
			reason,
		}
	}

	pub fn rejected(
		session: LubikanActivationSession,
		reason: LubikanSummonDepartureReason,
	) -> Self {
		Self {
			summon: None,
			owner: session.owner,
			account: session.account,
			definition: session.definition,
			session: session.id,
			reason,
		}
	}

	pub fn is_spawn_reject(self) -> bool {
		self.summon.is_none()
	}
}

/// HUD-readable snapshot of live summons. Presentation does not mutate charge.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct LubikanSummons {
	pub live: u32,
}

#[derive(SystemParam)]
struct SummonClose<'w, 's> {
	activations: LubikanActivationAccess<'w, 's>,
	reports: MessageWriter<'w, LubikanActivationReport>,
	finished: MessageWriter<'w, LubikanActivationFinished>,
	departures: MessageWriter<'w, LubikanSummonDeparture>,
	refunds: MessageWriter<'w, LubikanChargeRequest>,
	config: Res<'w, LubikanSummonConfig>,
	commands: Commands<'w, 's>,
}

impl SummonClose<'_, '_> {
	fn reject(&mut self, session: LubikanActivationSession, reason: LubikanSummonDepartureReason) {
		self.cancel_session(session);
		if self.config.invalid_placement == LubikanSummonInvalidPlacement::RefundAndCancel {
			let refund = session.refund_amount();
			if refund > 0.0 {
				self.refunds.write(LubikanChargeRequest::new(
					session.owner,
					Some(session.account),
					refund,
					LubikanChargeReason::ACTIVATION_REFUND,
					None,
				));
			}
		}
		self.departures.write(LubikanSummonDeparture::rejected(session, reason));
	}

	fn retire(
		&mut self,
		entity: Entity,
		live: LubikanSummon,
		reason: LubikanSummonDepartureReason,
	) {
		if self.config.session_follows_instance {
			match reason {
				LubikanSummonDepartureReason::Timeout | LubikanSummonDepartureReason::Dismissed => {
					self.complete_session(live.session_ref());
				}
				_ => self.cancel_session(live.session_ref()),
			}
		}
		self.departures.write(LubikanSummonDeparture::from_live(live, entity, reason));
		if reason != LubikanSummonDepartureReason::Interrupted {
			// Typed departure is not combat death: strip Health so
			// `down_zero_health` cannot synthesize [`Died`].
			self.commands.entity(entity).try_remove::<(Health, Downed)>();
		}
		self.commands.entity(entity).try_despawn();
	}

	fn complete_session(&mut self, session: LubikanActivationSession) {
		self.apply_close(session, LubikanActivationOutcome::Completed);
	}

	fn cancel_session(&mut self, session: LubikanActivationSession) {
		self.apply_close(session, LubikanActivationOutcome::Cancelled);
	}

	fn apply_close(
		&mut self,
		session: LubikanActivationSession,
		outcome: LubikanActivationOutcome,
	) {
		let pulse = self.activations.close_summon(session, outcome);
		self.reports.write(LubikanActivationReport { id: session.id, outcome });
		self.finished.write(pulse);
	}
}

pub(crate) fn configure(app: &mut App) {
	if !app.is_plugin_added::<bevy::time::TimePlugin>() {
		app.add_plugins(bevy::time::TimePlugin);
	}
	if app.world().get_resource::<Messages<Died>>().is_none() {
		app.add_message::<Died>();
	}
	if app.world().get_resource::<Messages<LubikanSummonDeparture>>().is_none() {
		app.add_message::<LubikanSummonDeparture>();
	}
	app.init_resource::<LubikanSummonConfig>()
		.init_resource::<LubikanSummonEnabled>()
		.init_resource::<LubikanSummonCatalog>()
		.init_resource::<LubikanSummons>()
		.configure_sets(Update, LubikanSystems::Summon.after(LubikanSystems::Activate))
		.add_systems(
			Update,
			(accept_summon_sessions, retire_summons, tick_summons, publish_summon_count)
				.chain()
				.in_set(LubikanSystems::Summon),
		);
}

#[derive(SystemParam)]
struct SummonAccept<'w, 's> {
	sessions: MessageReader<'w, 's, LubikanActivationSession>,
	owners: Query<'w, 's, (), With<LubikanOwner>>,
	transforms: Query<'w, 's, &'static Transform>,
	healths: Query<'w, 's, &'static Health>,
	downed: Query<'w, 's, (), With<Downed>>,
	summons: Query<'w, 's, &'static LubikanSummon>,
	recipes: Res<'w, LubikanSummonCatalog>,
}

fn accept_summon_sessions(mut close: SummonClose, mut accept: SummonAccept) {
	let mut reserved: HashMap<Entity, usize> = HashMap::new();
	for session in accept.sessions.read() {
		if session.definition.is_self() {
			continue;
		}
		if close.activations.session_phase(session.id) != Some(LubikanActivationPhase::Active) {
			// Already closed (Heal / external cancel). Do not spawn or refund.
			continue;
		}
		if close.config.block_while_self_active
			&& close.activations.owner_phase(session.owner, LubikanAccountId::SELF)
				== Some(LubikanActivationPhase::Active)
		{
			close.reject(*session, LubikanSummonDepartureReason::BlockedBySelf);
			continue;
		}
		if accept.owners.get(session.owner).is_err() {
			close.reject(*session, LubikanSummonDepartureReason::OwnerRetired);
			continue;
		}
		if close.config.require_living_owner
			&& owner_unavailable(session.owner, &accept.healths, &accept.downed)
		{
			close.reject(*session, LubikanSummonDepartureReason::OwnerRetired);
			continue;
		}
		let Some(recipe) = accept.recipes.get(session.definition) else {
			// Not a summon definition. Leave the session for another consumer.
			continue;
		};
		let live = accept.summons.iter().filter(|summon| summon.owner == session.owner).count();
		let pending = reserved.get(&session.owner).copied().unwrap_or(0);
		if live + pending >= close.config.max_concurrent() as usize {
			close.reject(*session, LubikanSummonDepartureReason::LimitReached);
			continue;
		}
		let Ok(origin) = accept.transforms.get(session.owner) else {
			close.reject(*session, LubikanSummonDepartureReason::InvalidPlacement);
			continue;
		};
		let Some(pose) = placement_from(origin, close.config.forward_offset()) else {
			close.reject(*session, LubikanSummonDepartureReason::InvalidPlacement);
			continue;
		};
		let lifetime = recipe
			.lifetime_secs()
			.unwrap_or_else(|| close.config.lifetime_secs().unwrap_or(0.0));
		let instance = LubikanSummonInstanceId::mint();
		let live = LubikanSummon {
			owner: session.owner,
			account: session.account,
			definition: session.definition,
			session: session.id,
			instance,
			elapsed: 0.0,
			lifetime,
			spent: session.spent,
		};
		close.commands.spawn((
			live,
			pose,
			GlobalTransform::from(pose),
			LubikanAccountBinding::new(session.account, session.definition, session.owner),
			ResponsibleOwner::new(session.owner),
			Health::from_max(recipe.health()),
			Name::new("lubikan-summon"),
		));
		*reserved.entry(session.owner).or_insert(0) += 1;
		if !close.config.session_follows_instance {
			close.complete_session(*session);
		}
	}
}

fn retire_summons(
	mut close: SummonClose,
	summons: Query<(Entity, &LubikanSummon)>,
	owners: Query<(), With<LubikanOwner>>,
	healths: Query<&Health>,
	downed: Query<(), With<Downed>>,
	mut deaths: MessageReader<Died>,
	mut lives: MessageReader<LubikanOwnerLifetime>,
) {
	let deaths: Vec<Died> = deaths.read().copied().collect();
	let notices: Vec<LubikanOwnerLifetime> = lives.read().copied().collect();
	let mut retired = Vec::new();
	for (entity, live) in &summons {
		if deaths.iter().any(|death| death.entity == entity) {
			retired.push((entity, *live, LubikanSummonDepartureReason::Interrupted));
			continue;
		}
		if close.config.retire_on_owner_lifetime
			&& should_retire_for_owner(*live, &owners, &healths, &downed, &deaths, &notices)
		{
			let reason = if notices
				.iter()
				.any(|notice| notice.kind == LubikanOwnerLifetimeKind::RestartSession)
			{
				LubikanSummonDepartureReason::WorldReset
			} else {
				LubikanSummonDepartureReason::OwnerRetired
			};
			retired.push((entity, *live, reason));
			continue;
		}
		if close.config.session_follows_instance
			&& close.activations.session_phase(live.session) != Some(LubikanActivationPhase::Active)
		{
			retired.push((entity, *live, LubikanSummonDepartureReason::Cancelled));
			continue;
		}
		if close.config.dismiss_on_select_change
			&& close
				.activations
				.owner_selection(live.owner)
				.is_some_and(|account| account != live.account)
		{
			retired.push((entity, *live, LubikanSummonDepartureReason::Dismissed));
		}
	}
	for (entity, live, reason) in retired {
		close.retire(entity, live, reason);
	}
}

fn tick_summons(
	time: Res<Time>,
	enabled: Res<LubikanSummonEnabled>,
	mut close: SummonClose,
	mut summons: Query<(Entity, &mut LubikanSummon)>,
) {
	if !enabled.0 {
		return;
	}
	let dt = time.delta_secs().max(0.0);
	let mut expired = Vec::new();
	for (entity, mut live) in &mut summons {
		live.elapsed += dt;
		if live.timed_out() {
			expired.push((entity, *live));
		}
	}
	for (entity, live) in expired {
		close.retire(entity, live, LubikanSummonDepartureReason::Timeout);
	}
}

fn publish_summon_count(
	summons: Query<(), With<LubikanSummon>>,
	mut summary: ResMut<LubikanSummons>,
) {
	summary.live = summons.iter().count() as u32;
}

fn owner_unavailable(
	owner: Entity,
	healths: &Query<&Health>,
	downed: &Query<(), With<Downed>>,
) -> bool {
	downed.get(owner).is_ok() || healths.get(owner).is_ok_and(|health| health.is_dead())
}

fn should_retire_for_owner(
	live: LubikanSummon,
	owners: &Query<(), With<LubikanOwner>>,
	healths: &Query<&Health>,
	downed: &Query<(), With<Downed>>,
	deaths: &[Died],
	notices: &[LubikanOwnerLifetime],
) -> bool {
	if owners.get(live.owner).is_err() {
		return true;
	}
	if owner_unavailable(live.owner, healths, downed) {
		return true;
	}
	if deaths.iter().any(|death| death.entity == live.owner) {
		return true;
	}
	notices.iter().any(|notice| match notice.kind {
		LubikanOwnerLifetimeKind::RestartSession => true,
		LubikanOwnerLifetimeKind::ReplaceBody | LubikanOwnerLifetimeKind::Unequip => {
			notice.previous == Some(live.owner)
		}
		LubikanOwnerLifetimeKind::Equip => false,
	})
}

fn placement_from(owner: &Transform, offset: f32) -> Option<Transform> {
	if !offset.is_finite() {
		return None;
	}
	let origin = owner.translation + *owner.forward() * offset;
	if !origin.is_finite() {
		return None;
	}
	Some(Transform::from_translation(origin).looking_to(*owner.forward(), Vec3::Y))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::LubikanSystems;
	use crate::activation::{LubikanActivationDenied, LubikanActivationPhase};
	use crate::catalog::LubikanCatalog;
	use crate::plugin::LubikansPlugin;
	use crate::{
		LubikanActivationFinished, LubikanActivationRefused, LubikanActivationReport,
		LubikanActivationRequest, LubikanChargeChanged,
	};
	use anyhow::{Result, anyhow};
	use bevy::time::{TimeUpdateStrategy, Virtual};
	use damage::DamagePlugin;
	use std::time::Duration;

	const DT: f32 = 1.0;

	fn summon_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)))
			.add_plugins(LubikansPlugin);
		app.world_mut()
			.resource_mut::<Time<Virtual>>()
			.set_max_delta(Duration::from_secs(30));
		install_fixture_summon(&mut app);
		app
	}

	fn spawn_owner(app: &mut App) -> Result<Entity> {
		let owner = app
			.world_mut()
			.spawn((
				LubikanOwner,
				Transform::from_xyz(0.0, 0.0, 0.0).looking_to(Vec3::NEG_Z, Vec3::Y),
			))
			.id();
		app.update();
		Ok(owner)
	}

	fn unlock_fixture(app: &mut App, owner: Entity) -> Result<LubikanAccountId> {
		let definition = fixture_summon_definition();
		let account = {
			let mut ledger = crate::LubikanBodyOf::ledger_mut(app.world_mut(), owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			let account = ledger.ensure_owned(definition);
			if !ledger.select(Some(account)) {
				return Err(anyhow!("select fixture"));
			}
			account
		};
		Ok(account)
	}

	fn fund_and_activate(
		app: &mut App,
		owner: Entity,
		account: LubikanAccountId,
	) -> Result<LubikanActivationSession> {
		let cost = app.world().resource::<LubikanCatalog>().activation_cost(account.definition());
		{
			let mut ledger = crate::LubikanBodyOf::ledger_mut(app.world_mut(), owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			ledger.apply_to(account, cost).ok_or_else(|| anyhow!("fund"))?;
		}
		app.world_mut()
			.write_message(LubikanActivationRequest::for_account(owner, account));
		app.update();
		app.world_mut()
			.resource_mut::<Messages<LubikanActivationSession>>()
			.drain()
			.find(|session| session.account == account)
			.ok_or_else(|| anyhow!("summon session"))
	}

	fn phase(
		app: &App,
		owner: Entity,
		account: LubikanAccountId,
	) -> Result<LubikanActivationPhase> {
		crate::LubikanBodyOf::ledger(app.world(), owner)
			.and_then(|ledger| ledger.account(account))
			.map(|account| account.activation().phase)
			.ok_or_else(|| anyhow!("activation phase"))
	}

	fn charge(app: &App, owner: Entity, account: LubikanAccountId) -> Result<f32> {
		crate::LubikanBodyOf::ledger(app.world(), owner)
			.and_then(|ledger| ledger.account(account))
			.map(|account| account.charge().current())
			.ok_or_else(|| anyhow!("charge"))
	}

	fn live_summons(app: &mut App) -> Vec<(Entity, LubikanSummon)> {
		let world = app.world_mut();
		let mut query = world.query::<(Entity, &LubikanSummon)>();
		query.iter(world).map(|(entity, summon)| (entity, *summon)).collect()
	}

	fn departures(app: &mut App) -> Vec<LubikanSummonDeparture> {
		app.world_mut()
			.resource_mut::<Messages<LubikanSummonDeparture>>()
			.drain()
			.collect()
	}

	fn finished(app: &mut App) -> Vec<LubikanActivationFinished> {
		app.world_mut()
			.resource_mut::<Messages<LubikanActivationFinished>>()
			.drain()
			.collect()
	}

	fn died(app: &mut App) -> Vec<Died> {
		app.world_mut().resource_mut::<Messages<Died>>().drain().collect()
	}

	#[test]
	fn self_session_is_ignored_by_the_summon_consumer() -> Result<()> {
		let mut app = summon_app();
		let owner = spawn_owner(&mut app)?;
		app.world_mut().entity_mut(owner).insert(Health { current: 40.0, max: 100.0 });
		let cost = app.world().resource::<LubikanCatalog>().self_definition().activation_cost;
		{
			let mut ledger = crate::LubikanBodyOf::ledger_mut(app.world_mut(), owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			ledger.apply_to_equipped(cost);
		}
		app.world_mut().write_message(LubikanActivationRequest::equipped(owner));
		app.update();
		assert!(live_summons(&mut app).is_empty());
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Active);
		assert_eq!(charge(&app, owner, LubikanAccountId::SELF)?, 0.0);
		assert!(departures(&mut app).is_empty());
		let spends: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<LubikanChargeChanged>>()
			.drain()
			.filter(|change| {
				change.reason == LubikanChargeReason::ACTIVATION
					|| change.reason == LubikanChargeReason::ACTIVATION_REFUND
			})
			.collect();
		assert_eq!(spends.len(), 1);
		assert_eq!(spends[0].reason, LubikanChargeReason::ACTIVATION);
		assert_eq!(spends[0].amount, -cost);
		assert!(app.world().resource::<crate::LubikanSelfHeal>().active);
		Ok(())
	}

	#[test]
	fn fixture_activation_spends_once_and_spawns_owned_instance() -> Result<()> {
		let mut app = summon_app();
		let owner = spawn_owner(&mut app)?;
		let account = unlock_fixture(&mut app, owner)?;
		let cost = app.world().resource::<LubikanCatalog>().activation_cost(account.definition());
		let session = fund_and_activate(&mut app, owner, account)?;
		assert_eq!(session.definition, FIXTURE_SUMMON_DEFINITION_ID);
		let spends: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<LubikanChargeChanged>>()
			.drain()
			.filter(|change| change.reason == LubikanChargeReason::ACTIVATION)
			.collect();
		assert_eq!(spends.len(), 1);
		assert_eq!(spends[0].amount, -cost);
		assert_eq!(charge(&app, owner, account)?, 0.0);
		assert_eq!(phase(&app, owner, account)?, LubikanActivationPhase::Active);
		let live = live_summons(&mut app);
		assert_eq!(live.len(), 1);
		assert_eq!(live[0].1.owner, owner);
		assert_eq!(live[0].1.account, account);
		assert_eq!(live[0].1.session, session.id);
		let binding = app
			.world()
			.get::<LubikanAccountBinding>(live[0].0)
			.copied()
			.ok_or_else(|| anyhow!("binding"))?;
		assert_eq!(binding.account, account);
		assert_eq!(binding.owner, owner);
		let responsible = app
			.world()
			.get::<ResponsibleOwner>(live[0].0)
			.copied()
			.ok_or_else(|| anyhow!("responsible owner"))?;
		assert_eq!(responsible.actor, owner);
		let pose = app
			.world()
			.get::<Transform>(live[0].0)
			.copied()
			.ok_or_else(|| anyhow!("summon transform"))?;
		assert!(
			(pose.translation - Vec3::new(0.0, 0.0, -PLACEHOLDER_SUMMON_FORWARD_OFFSET)).length()
				< 1e-4
		);
		Ok(())
	}

	#[test]
	fn missing_transform_rejects_without_orphan_and_refunds() -> Result<()> {
		let mut app = summon_app();
		let owner = app.world_mut().spawn(LubikanOwner).id();
		app.update();
		let account = unlock_fixture(&mut app, owner)?;
		fund_and_activate(&mut app, owner, account)?;
		assert!(live_summons(&mut app).is_empty());
		assert_eq!(phase(&app, owner, account)?, LubikanActivationPhase::Idle);
		let rejected = departures(&mut app);
		assert_eq!(rejected.len(), 1);
		assert_eq!(rejected[0].reason, LubikanSummonDepartureReason::InvalidPlacement);
		assert!(rejected[0].is_spawn_reject());
		app.update();
		assert_eq!(charge(&app, owner, account)?, PLACEHOLDER_SUMMON_ACTIVATION_COST);
		assert!(died(&mut app).is_empty());
		Ok(())
	}

	#[test]
	fn refund_uses_spent_amount_not_later_catalog_cost() -> Result<()> {
		let mut app = summon_app();
		let owner = app.world_mut().spawn(LubikanOwner).id();
		app.update();
		let account = unlock_fixture(&mut app, owner)?;
		app.add_systems(
			Update,
			inflate_fixture_cost
				.after(LubikanSystems::Activate)
				.before(LubikanSystems::Summon),
		);
		fund_and_activate(&mut app, owner, account)?;
		assert!(live_summons(&mut app).is_empty());
		assert_eq!(
			departures(&mut app).last().map(|event| event.reason),
			Some(LubikanSummonDepartureReason::InvalidPlacement)
		);
		app.update();
		assert_eq!(charge(&app, owner, account)?, PLACEHOLDER_SUMMON_ACTIVATION_COST);
		assert_ne!(
			app.world().resource::<LubikanCatalog>().activation_cost(account.definition()),
			PLACEHOLDER_SUMMON_ACTIVATION_COST
		);
		Ok(())
	}

	fn inflate_fixture_cost(mut catalog: ResMut<LubikanCatalog>) {
		catalog.register(LubikanDefinition::new(
			FIXTURE_SUMMON_DEFINITION_ID,
			PLACEHOLDER_SUMMON_CAPACITY,
			99.0,
		));
	}

	#[test]
	fn missing_recipe_is_not_consumed() -> Result<()> {
		let mut app = summon_app();
		let owner = spawn_owner(&mut app)?;
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(3), 40.0, 5.0);
		app.world_mut().resource_mut::<LubikanCatalog>().register(extra);
		let account = {
			let mut ledger = crate::LubikanBodyOf::ledger_mut(app.world_mut(), owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			let account = ledger.ensure_owned(extra);
			if !ledger.select(Some(account)) {
				return Err(anyhow!("select extra"));
			}
			account
		};
		fund_and_activate(&mut app, owner, account)?;
		assert!(live_summons(&mut app).is_empty());
		assert!(departures(&mut app).is_empty());
		assert_eq!(charge(&app, owner, account)?, 0.0);
		assert_eq!(phase(&app, owner, account)?, LubikanActivationPhase::Active);
		Ok(())
	}

	#[test]
	fn rapid_second_activate_is_already_active() -> Result<()> {
		let mut app = summon_app();
		let owner = spawn_owner(&mut app)?;
		let account = unlock_fixture(&mut app, owner)?;
		fund_and_activate(&mut app, owner, account)?;
		assert_eq!(live_summons(&mut app).len(), 1);
		{
			let mut ledger = crate::LubikanBodyOf::ledger_mut(app.world_mut(), owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			ledger.apply_to(account, PLACEHOLDER_SUMMON_ACTIVATION_COST);
		}
		app.world_mut()
			.write_message(LubikanActivationRequest::for_account(owner, account));
		app.update();
		let refused = app
			.world_mut()
			.resource_mut::<Messages<LubikanActivationRefused>>()
			.drain()
			.next()
			.ok_or_else(|| anyhow!("refused"))?;
		assert_eq!(refused.reason, LubikanActivationDenied::AlreadyActive);
		assert_eq!(live_summons(&mut app).len(), 1);
		assert_eq!(charge(&app, owner, account)?, PLACEHOLDER_SUMMON_ACTIVATION_COST);
		Ok(())
	}

	#[test]
	fn concurrent_limit_rejects_a_second_owner_instance() -> Result<()> {
		let mut app = summon_app();
		app.world_mut().resource_mut::<LubikanSummonConfig>().session_follows_instance = false;
		let owner = spawn_owner(&mut app)?;
		let account = unlock_fixture(&mut app, owner)?;
		fund_and_activate(&mut app, owner, account)?;
		assert_eq!(live_summons(&mut app).len(), 1);
		assert_eq!(phase(&app, owner, account)?, LubikanActivationPhase::Idle);
		fund_and_activate(&mut app, owner, account)?;
		assert_eq!(live_summons(&mut app).len(), 1);
		assert_eq!(
			departures(&mut app).last().map(|event| event.reason),
			Some(LubikanSummonDepartureReason::LimitReached)
		);
		app.update();
		assert_eq!(charge(&app, owner, account)?, PLACEHOLDER_SUMMON_ACTIVATION_COST);
		Ok(())
	}

	#[test]
	fn same_batch_activations_respect_concurrent_limit() -> Result<()> {
		let mut app = summon_app();
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(0xFFFF_FF02), 100.0, 25.0);
		app.world_mut().resource_mut::<LubikanCatalog>().register(extra);
		app.world_mut()
			.resource_mut::<LubikanSummonCatalog>()
			.register(LubikanSummonRecipe {
				definition: extra.id,
				..LubikanSummonRecipe::fixture()
			});
		let owner = spawn_owner(&mut app)?;
		let first = unlock_fixture(&mut app, owner)?;
		let second = {
			let mut ledger = crate::LubikanBodyOf::ledger_mut(app.world_mut(), owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			let second = ledger.ensure_owned(extra);
			ledger
				.apply_to(first, PLACEHOLDER_SUMMON_ACTIVATION_COST)
				.ok_or_else(|| anyhow!("fund first"))?;
			ledger
				.apply_to(second, extra.activation_cost)
				.ok_or_else(|| anyhow!("fund second"))?;
			second
		};
		app.world_mut()
			.write_message(LubikanActivationRequest::for_account(owner, first));
		app.world_mut()
			.write_message(LubikanActivationRequest::for_account(owner, second));
		app.update();
		assert_eq!(live_summons(&mut app).len(), 1);
		let rejected = departures(&mut app);
		assert_eq!(
			rejected
				.iter()
				.filter(|event| event.reason == LubikanSummonDepartureReason::LimitReached)
				.count(),
			1
		);
		app.update();
		let first_charge = charge(&app, owner, first)?;
		let second_charge = charge(&app, owner, second)?;
		assert_eq!(first_charge + second_charge, extra.activation_cost);
		assert!(first_charge == 0.0 || second_charge == 0.0);
		Ok(())
	}

	#[test]
	fn core_spawn_does_not_attach_fixture_meshes() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)))
			.add_plugins(LubikansPlugin);
		app.world_mut()
			.resource_mut::<Time<Virtual>>()
			.set_max_delta(Duration::from_secs(30));
		app.world_mut()
			.resource_mut::<LubikanCatalog>()
			.register(fixture_summon_definition());
		app.world_mut()
			.resource_mut::<LubikanSummonCatalog>()
			.register(LubikanSummonRecipe::fixture());
		let owner = spawn_owner(&mut app)?;
		let account = unlock_fixture(&mut app, owner)?;
		fund_and_activate(&mut app, owner, account)?;
		let summon = live_summons(&mut app)
			.first()
			.map(|(entity, _)| *entity)
			.ok_or_else(|| anyhow!("summon"))?;
		assert!(app.world().get::<Mesh3d>(summon).is_none());
		Ok(())
	}

	#[test]
	fn selection_change_does_not_dismiss_or_retarget() -> Result<()> {
		let mut app = summon_app();
		let owner = spawn_owner(&mut app)?;
		let account = unlock_fixture(&mut app, owner)?;
		let session = fund_and_activate(&mut app, owner, account)?;
		{
			let mut ledger = crate::LubikanBodyOf::ledger_mut(app.world_mut(), owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			if !ledger.select(None) {
				return Err(anyhow!("back to self"));
			}
		}
		app.update();
		let live = live_summons(&mut app);
		assert_eq!(live.len(), 1);
		assert_eq!(live[0].1.account, account);
		assert_eq!(live[0].1.session, session.id);
		assert_eq!(phase(&app, owner, account)?, LubikanActivationPhase::Active);
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Idle);
		Ok(())
	}

	#[test]
	fn timeout_departs_without_died_and_idles_the_captured_account() -> Result<()> {
		let mut app = summon_app();
		app.add_plugins(DamagePlugin);
		app.world_mut().resource_mut::<LubikanSummonConfig>().lifetime_secs = 2.0;
		app.world_mut()
			.resource_mut::<LubikanSummonCatalog>()
			.register(LubikanSummonRecipe { lifetime_secs: 2.0, ..LubikanSummonRecipe::fixture() });
		let owner = spawn_owner(&mut app)?;
		let account = unlock_fixture(&mut app, owner)?;
		let session = fund_and_activate(&mut app, owner, account)?;
		assert_eq!(live_summons(&mut app).len(), 1);
		app.update();
		assert!(live_summons(&mut app).is_empty());
		let left = departures(&mut app);
		assert_eq!(left.len(), 1);
		assert_eq!(left[0].reason, LubikanSummonDepartureReason::Timeout);
		assert!(left[0].summon.is_some());
		assert!(died(&mut app).is_empty());
		assert_eq!(phase(&app, owner, account)?, LubikanActivationPhase::Idle);
		let pulse = finished(&mut app);
		assert_eq!(pulse.last().map(|event| event.id), Some(session.id));
		assert_eq!(
			pulse.last().map(|event| event.outcome),
			Some(LubikanActivationOutcome::Completed)
		);
		Ok(())
	}

	#[test]
	fn cancelled_session_departs_without_died() -> Result<()> {
		let mut app = summon_app();
		app.add_plugins(DamagePlugin);
		let owner = spawn_owner(&mut app)?;
		let account = unlock_fixture(&mut app, owner)?;
		let session = fund_and_activate(&mut app, owner, account)?;
		assert_eq!(live_summons(&mut app).len(), 1);
		app.world_mut().write_message(LubikanActivationReport::cancelled(session.id));
		app.update();
		assert!(live_summons(&mut app).is_empty());
		assert_eq!(
			departures(&mut app).last().map(|event| event.reason),
			Some(LubikanSummonDepartureReason::Cancelled)
		);
		assert!(died(&mut app).is_empty());
		assert_eq!(phase(&app, owner, account)?, LubikanActivationPhase::Idle);
		Ok(())
	}

	#[test]
	fn owner_retirement_and_session_end_retire_owned_state() -> Result<()> {
		let mut app = summon_app();
		let first = spawn_owner(&mut app)?;
		let account = unlock_fixture(&mut app, first)?;
		fund_and_activate(&mut app, first, account)?;
		assert_eq!(live_summons(&mut app).len(), 1);
		app.world_mut().entity_mut(first).remove::<LubikanOwner>();
		app.update();
		assert!(live_summons(&mut app).is_empty());
		assert_eq!(
			departures(&mut app).last().map(|event| event.reason),
			Some(LubikanSummonDepartureReason::OwnerRetired)
		);

		let owner = spawn_owner(&mut app)?;
		let account = unlock_fixture(&mut app, owner)?;
		fund_and_activate(&mut app, owner, account)?;
		app.world_mut().write_message(LubikanOwnerLifetime::new(
			None,
			None,
			LubikanOwnerLifetimeKind::RestartSession,
		));
		app.update();
		assert!(live_summons(&mut app).is_empty());
		assert_eq!(
			departures(&mut app).last().map(|event| event.reason),
			Some(LubikanSummonDepartureReason::WorldReset)
		);
		Ok(())
	}

	#[test]
	fn lethal_hit_is_interrupted_departure_not_a_summon_kill_path() -> Result<()> {
		let mut app = summon_app();
		let owner = spawn_owner(&mut app)?;
		let account = unlock_fixture(&mut app, owner)?;
		fund_and_activate(&mut app, owner, account)?;
		let summon = live_summons(&mut app)
			.first()
			.map(|(entity, _)| *entity)
			.ok_or_else(|| anyhow!("summon"))?;
		app.world_mut()
			.write_message(Died { entity: summon, source: None, point: Vec3::ZERO });
		app.update();
		assert!(live_summons(&mut app).is_empty());
		assert_eq!(
			departures(&mut app).last().map(|event| event.reason),
			Some(LubikanSummonDepartureReason::Interrupted)
		);
		assert_eq!(phase(&app, owner, account)?, LubikanActivationPhase::Idle);
		Ok(())
	}

	#[test]
	fn pause_freezes_lifetime_without_dismissing() -> Result<()> {
		let mut app = summon_app();
		app.world_mut().resource_mut::<LubikanSummonConfig>().lifetime_secs = 2.0;
		app.world_mut()
			.resource_mut::<LubikanSummonCatalog>()
			.register(LubikanSummonRecipe { lifetime_secs: 2.0, ..LubikanSummonRecipe::fixture() });
		let owner = spawn_owner(&mut app)?;
		let account = unlock_fixture(&mut app, owner)?;
		fund_and_activate(&mut app, owner, account)?;
		app.world_mut().resource_mut::<LubikanSummonEnabled>().0 = false;
		app.update();
		app.update();
		assert_eq!(live_summons(&mut app).len(), 1);
		assert_eq!(phase(&app, owner, account)?, LubikanActivationPhase::Active);
		app.world_mut().resource_mut::<LubikanSummonEnabled>().0 = true;
		app.update();
		app.update();
		assert!(live_summons(&mut app).is_empty());
		Ok(())
	}

	#[test]
	fn self_and_summon_sessions_stay_independent() -> Result<()> {
		let mut app = summon_app();
		let owner = spawn_owner(&mut app)?;
		app.world_mut().entity_mut(owner).insert(Health { current: 40.0, max: 100.0 });
		let account = unlock_fixture(&mut app, owner)?;
		let self_cost = app.world().resource::<LubikanCatalog>().self_definition().activation_cost;
		{
			let mut ledger = crate::LubikanBodyOf::ledger_mut(app.world_mut(), owner)
				.ok_or_else(|| anyhow!("ledger"))?;
			ledger.apply_to(LubikanAccountId::SELF, self_cost);
		}
		app.world_mut()
			.write_message(LubikanActivationRequest::for_account(owner, LubikanAccountId::SELF));
		fund_and_activate(&mut app, owner, account)?;
		assert_eq!(live_summons(&mut app).len(), 1);
		assert_eq!(phase(&app, owner, account)?, LubikanActivationPhase::Active);
		assert_eq!(phase(&app, owner, LubikanAccountId::SELF)?, LubikanActivationPhase::Active);
		assert!(app.world().resource::<crate::LubikanSelfHeal>().active);
		Ok(())
	}
}
