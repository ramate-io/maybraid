//! Y chooses which equipped queue the D-Pad cycles.
//!
//! This resource is world-owned so gameplay still works when `combat-hud` is
//! absent. The HUD only reads a synced copy.

use bevy::prelude::*;
use combat_hud::{idle_cycle_outline, CombatHudCyclePanel, CombatHudInventoryCycleCategory};
use lubikans::LubikanLedger;
use maybraid_character_controller::CharacterIntent;
use maybraid_skill_map::{SkillMapFirearmHud, SkillMapViewport};
use world_player::Player as VegetationPlayer;

use crate::control::WorldGameplayEnabled;

/// Which HUD system the D-Pad is currently cycling.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InventoryCycleTarget {
	#[default]
	Weapons,
	SkillMaps,
	Lubikans,
}

impl InventoryCycleTarget {
	/// Y advances Weapons → SkillMaps → Lubikans → Weapons.
	pub const fn next(self) -> Self {
		match self {
			Self::Weapons => Self::SkillMaps,
			Self::SkillMaps => Self::Lubikans,
			Self::Lubikans => Self::Weapons,
		}
	}

	pub const fn to_hud(self) -> CombatHudInventoryCycleCategory {
		match self {
			Self::Weapons => CombatHudInventoryCycleCategory::Weapons,
			Self::SkillMaps => CombatHudInventoryCycleCategory::SkillMaps,
			Self::Lubikans => CombatHudInventoryCycleCategory::Lubikans,
		}
	}
}

pub(crate) fn cycle_inventory_target(
	mut intents: MessageReader<CharacterIntent>,
	gameplay: Res<WorldGameplayEnabled>,
	mut target: ResMut<InventoryCycleTarget>,
) {
	if !gameplay.0 {
		return;
	}
	let steps = intents
		.read()
		.filter(|intent| matches!(intent, CharacterIntent::CycleInventoryTarget))
		.count();
	for _ in 0..steps {
		*target = target.next();
	}
}

/// HUD-only snapshot. Missing `combat-hud` is a no-op.
pub(crate) fn sync_combat_hud_cycle_category(
	target: Res<InventoryCycleTarget>,
	hud: Option<ResMut<CombatHudInventoryCycleCategory>>,
) {
	let Some(mut hud) = hud else {
		return;
	};
	let next = target.to_hud();
	if *hud != next {
		*hud = next;
	}
}

/// Attach the generic cycle marker to existing weapons / skill-map panel roots.
pub(crate) fn stamp_cycle_panel_markers(
	mut commands: Commands,
	firearms: Query<Entity, (With<SkillMapFirearmHud>, Without<CombatHudCyclePanel>)>,
	maps: Query<Entity, (With<SkillMapViewport>, Without<CombatHudCyclePanel>)>,
) {
	for entity in &firearms {
		commands.entity(entity).insert((
			CombatHudCyclePanel(CombatHudInventoryCycleCategory::Weapons),
			idle_cycle_outline(),
		));
	}
	for entity in &maps {
		commands.entity(entity).insert((
			CombatHudCyclePanel(CombatHudInventoryCycleCategory::SkillMaps),
			idle_cycle_outline(),
		));
	}
}

pub(crate) fn cycle_lubikans(
	mut intents: MessageReader<CharacterIntent>,
	gameplay: Res<WorldGameplayEnabled>,
	target: Option<Res<InventoryCycleTarget>>,
	mut players: Query<&mut LubikanLedger, With<VegetationPlayer>>,
) {
	if !gameplay.0 || !target.is_some_and(|target| *target == InventoryCycleTarget::Lubikans) {
		return;
	}
	let Some(dir) = intents.read().find_map(|intent| match *intent {
		CharacterIntent::CycleSkillMap(dir) => Some(dir),
		_ => None,
	}) else {
		return;
	};
	for mut ledger in &mut players {
		cycle_ledger(&mut ledger, dir);
	}
}

fn cycle_ledger(ledger: &mut LubikanLedger, dir: i8) {
	let ids: Vec<_> = ledger.iter().map(|account| account.id()).collect();
	if ids.len() < 2 {
		return;
	}
	let current = ledger.equipped().id();
	let Some(index) = ids.iter().position(|id| *id == current) else {
		return;
	};
	let next = if dir >= 0 {
		ids[(index + 1) % ids.len()]
	} else {
		ids[(index + ids.len() - 1) % ids.len()]
	};
	let _ = ledger.select(Some(next));
}

pub(crate) fn configure(app: &mut App) {
	app.init_resource::<InventoryCycleTarget>().add_systems(
		Update,
		(
			cycle_inventory_target.after(maybraid_character_controller::CharacterControlSystems),
			sync_combat_hud_cycle_category,
			stamp_cycle_panel_markers,
			cycle_lubikans.after(maybraid_character_controller::CharacterControlSystems),
		),
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use lubikans::{LubikanDefinition, LubikanDefinitionId};

	fn press_y(world: &mut World) -> anyhow::Result<()> {
		world.insert_resource(Messages::<CharacterIntent>::default());
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::CycleInventoryTarget);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(cycle_inventory_target)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn y_advances_and_wraps_the_cycle_target() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(InventoryCycleTarget::Weapons);
		press_y(&mut world)?;
		assert_eq!(*world.resource::<InventoryCycleTarget>(), InventoryCycleTarget::SkillMaps);
		press_y(&mut world)?;
		assert_eq!(*world.resource::<InventoryCycleTarget>(), InventoryCycleTarget::Lubikans);
		press_y(&mut world)?;
		assert_eq!(*world.resource::<InventoryCycleTarget>(), InventoryCycleTarget::Weapons);
		Ok(())
	}

	#[test]
	fn hud_sync_is_a_noop_when_combat_hud_is_absent() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(InventoryCycleTarget::Lubikans);
		world
			.run_system_once(sync_combat_hud_cycle_category)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get_resource::<CombatHudInventoryCycleCategory>().is_none());
		assert_eq!(*world.resource::<InventoryCycleTarget>(), InventoryCycleTarget::Lubikans);
		Ok(())
	}

	#[test]
	fn hud_sync_copies_the_world_target() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(InventoryCycleTarget::SkillMaps);
		world.init_resource::<CombatHudInventoryCycleCategory>();
		world
			.run_system_once(sync_combat_hud_cycle_category)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			*world.resource::<CombatHudInventoryCycleCategory>(),
			CombatHudInventoryCycleCategory::SkillMaps
		);
		Ok(())
	}

	#[test]
	fn dpad_cycles_owned_lubikans_when_that_target_is_active() -> anyhow::Result<()> {
		use lubikans::LubikanSelfConfig;

		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(InventoryCycleTarget::Lubikans);
		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(2), 25.0, 25.0);
		let mut ledger = LubikanLedger::with_self(&LubikanSelfConfig::default());
		let other = ledger.ensure_owned(extra);
		assert!(ledger.select(None));
		let first = ledger.equipped().id();
		world.spawn((VegetationPlayer, ledger));

		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::CycleSkillMap(1));
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(cycle_lubikans)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let equipped = world
			.query::<&LubikanLedger>()
			.iter(&world)
			.next()
			.map(|ledger| ledger.equipped().id())
			.ok_or_else(|| anyhow::anyhow!("ledger"))?;
		assert_eq!(equipped, other);
		assert_ne!(equipped, first);
		Ok(())
	}
}
