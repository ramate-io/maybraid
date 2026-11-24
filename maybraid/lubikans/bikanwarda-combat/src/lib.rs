//! Ward and Enraged combat policies for summoned Bikanwarda
//! ([#1137](https://github.com/ramate-io/maybraid/issues/1137)).
//!
//! Reuses [`bikanwarda_attacks`] Parade / God Beam / Bray ids and the
//! [`attacks`] runtime. Target selection and area-damage filtering are
//! separate: selection needs real [`combat_targeting::CombatContact`]
//! memory, while victim filtering is `BeginAttack::allies` plus authored
//! `FriendlyFire::ExcludeOwnerAndAllies`.
//!
//! This crate does not depend on `combat-hud` or `maybraid-world`. Self
//! gameplay must not take a dependency on it.
//!
//! # Open policies
//!
//! [#1137](https://github.com/ramate-io/maybraid/issues/1137) does not close
//! these. Defaults are labeled PLACEHOLDER on [`BikanwardaSummonConfig`] and
//! in the predicates:
//!
//! 1. Ward area safety — owner plus `TargetSource::ALLY` members are excluded.
//!    Affiliation-only allies (`known_allies` without an ALLY targeting row)
//!    still take overlap damage.
//! 2. "Your enemies" — Ward selects from the owner's existing
//!    [`CombatTargeting`] hostile sources ([`WARD_HOSTILE_SOURCES`]).
//!    [`Affiliations::threat_weight`] does not add candidates. Conflicts are
//!    recorded on [`BikanwardaSummonPolicy::conflicts`].
//! 3. Enraged vs owner's allies — Enraged may select and damage them,
//!    including the summoner.
//! 4. Summon self-hit — recipes are not copied; authored friendly-fire still
//!    excludes the summon (the attack owner).
//! 5. Summoner contact — Enraged never fabricates a sighting from a
//!    transform. The summoner is selectable only when a `CombatContact` for
//!    that entity already exists on the owner or the summon.
//! 6. Attack rotation — Parade → God Beam → Bray on `Completed`. Cadence is
//!    the authored cooldown.
//! 7. Input / equip — out of scope ([#1112](https://github.com/ramate-io/maybraid/issues/1112)).
//! 8. Charge rewards — [#1136](https://github.com/ramate-io/maybraid/issues/1136).
//! 9. Column, Ward buff, glow — [#1138](https://github.com/ramate-io/maybraid/issues/1138).
//!
//! Target-selection coverage lives in [`selection`] tests. Area-damage
//! coverage lives in [`victims`] tests.

mod catalog;
mod plugin;
mod policy;

pub use catalog::{
	enraged_definition, enraged_recipe, register_bikanwarda_summons, ward_definition, ward_recipe,
	ENRAGED_DEFINITION_ID, WARD_DEFINITION_ID,
};
pub use plugin::{BikanwardaLubikanPlugin, BikanwardaSummonSystems};
pub use policy::{
	admits_target, admits_victim, allies_for_begin, refresh_policy, sync_summon_targeting,
	BikanwardaSummonConfig, BikanwardaSummonForm, BikanwardaSummonPolicy, PolicyCandidate,
	PolicyConflict, PolicyView, AUTHORED_ATTACKS, WARD_HOSTILE_SOURCES,
};

#[cfg(test)]
mod selection;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod victims;
