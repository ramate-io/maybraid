//! HCSG boss generation for authored Bikanwarda encounter.
//!
//! Implements the generation contract for [#1117](https://github.com/ramate-io/maybraid/issues/1117).
//! Boss position, bounds, attack configuration, and actor recipe are deterministic
//! functions of identity and session roots.

use bevy::math::{DVec3, Vec3};
use bevy::prelude::*;
use bevy::shape::Aabb3d;
use bikanwarda_character::{BikanwardaActorRecipe, BikanwardaForm};
use lod::gen::{Id, OriginalId};
use lod::hcsg::{GenerationContext, GenerationScheme};

use crate::contract::{BossActorRecipe, BossAttackConfig, BossEncounterContract};
use crate::identity::chained_encounter_id;
use crate::scripted::Scripted;

/// Generation index scale: boss encounters are not tiled.
const BOSS_SCALE: DVec3 = DVec3::new(1.0, 1.0, 1.0);

/// Authored Bikanwarda boss encounter.
///
/// Generated Arc contains stable identity, bounds, ground dependency,
/// actor recipe, and attack configuration. Mutable health, chain durability,
/// unlocks, and phase progress live outside this value.
#[derive(Clone, Debug, PartialEq)]
pub struct BikanwardaBossEncounter {
	/// Encounter identity (stable across regeneration).
	pub id: encounters::EncounterId,
	
	/// World-space bounds for the entire encounter (boss + arena).
	pub bounds: Aabb3d,
	
	/// Boss center position in world space.
	pub boss_position: Vec3,
	
	/// Authored boss configuration.
	pub boss: Scripted<BossEncounterContract>,
}

impl BikanwardaBossEncounter {
	/// The authored Bikanwarda chained encounter.
	pub fn chained() -> Self {
		let id = chained_encounter_id();
		
		// Authored boss configuration
		let actor_recipe = BossActorRecipe {
			form: BikanwardaForm::Encounter,
			recipe: BikanwardaActorRecipe::for_form(BikanwardaForm::Encounter),
		};
		
		let attack_config = BossAttackConfig {
			placeholder: "attacks defined in bikanwarda-attacks crate".to_string(),
		};
		
		let boss = Scripted::new(BossEncounterContract {
			actor: actor_recipe,
			attacks: attack_config,
		});
		
		// Placeholder bounds (20m boss, ~40m arena)
		// World placement (#1117) will anchor this near initial spawn
		let boss_position = Vec3::ZERO;
		let bounds = Aabb3d::new(
			boss_position,
			Vec3::new(20.0, 10.0, 20.0),
		);
		
		Self { id, bounds, boss_position, boss }
	}
}

impl GenerationScheme for BikanwardaBossEncounter {
	lod::hcsg_index_scale!(BOSS_SCALE);
	
	fn original_ids_for(_cx: &mut GenerationContext, _region: Aabb3d) -> Vec<OriginalId> {
		// Authored encounter: single universal identity
		// Discovery filters by region in presentation
		vec![OriginalId::new(Id::Bytes(lod::gen::Bytes(
			chained_encounter_id().as_bytes()
		)))]
	}
	
	fn build_with_id(_cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		// Only build the authored chained encounter
		if id != Id::Bytes(lod::gen::Bytes(chained_encounter_id().as_bytes())) {
			return None;
		}
		
		// Deterministic from session roots (no dependencies yet)
		let encounter = Self::chained();
		Some((encounter.clone(), encounter.bounds))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use lod::hcsg::HcsgStorage;

	#[test]
	fn chained_encounter_is_deterministic() {
		let a = BikanwardaBossEncounter::chained();
		let b = BikanwardaBossEncounter::chained();
		assert_eq!(a.id, b.id);
		assert_eq!(a.bounds, b.bounds);
		assert_eq!(a.boss_position, b.boss_position);
	}

	#[test]
	fn generation_builds_only_chained_id() {
		let storage = HcsgStorage::default();
		let mut cx = GenerationContext::new(&storage);
		
		let chained_id = Id::Bytes(lod::gen::Bytes(chained_encounter_id().as_bytes()));
		let result = BikanwardaBossEncounter::build_with_id(&mut cx, chained_id);
		assert!(result.is_some());
		
		let other_id = Id::Universal;
		let result = BikanwardaBossEncounter::build_with_id(&mut cx, other_id);
		assert!(result.is_none());
	}

	#[test]
	fn boss_position_determines_bounds() {
		let encounter = BikanwardaBossEncounter::chained();
		let pos_bounds = Aabb3d::new(encounter.boss_position, Vec3::splat(0.1));
		assert!(encounter.bounds.intersects(&pos_bounds));
	}
}
