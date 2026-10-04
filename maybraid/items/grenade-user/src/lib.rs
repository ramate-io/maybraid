//! Held grenade, overhand throw, and owned recharge clocks.

mod hold;
mod throw;

use bevy::prelude::*;
use character_items::{Inventory, InventoryItem};
use characters::CharacterMotionSystems;
use player::{PlayerPoseSystems, PlayerSystems, PlayerUse};

pub use hold::{spawn_held_grenade, HeldGrenade, HoldingGrenade};
pub use throw::{GrenadePhase, GrenadeThrow, GrenadeUser, GrenadeUserSettings};

/// Held-side 1:1 target of [`GrenadeUser`].
#[derive(Component, Debug)]
#[relationship_target(relationship = GrenadeUser)]
pub struct HeldByGrenade(Entity);

pub struct GrenadeUserPlugin;

impl Plugin for GrenadeUserPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<grenades::GrenadesPlugin>() {
			app.add_plugins(grenades::GrenadesPlugin);
		}
		app.add_systems(Update, throw::apply_throw_intents.in_set(PlayerSystems::Intent))
			.add_systems(
				Update,
				(
					hold::stamp_holding_grenade,
					hold::pose_held_grenade,
					hold::apply_grenade_swap_pose,
					hold::sync_held_visibility,
				)
					.chain()
					.in_set(PlayerPoseSystems::Item),
			)
			.add_systems(
				Update,
				hold::sync_throw_arm
					.in_set(PlayerPoseSystems::Overlay)
					.after(CharacterMotionSystems::Anim),
			)
			.add_systems(Update, throw::advance_throw.after(PlayerPoseSystems::Overlay))
			.add_systems(
				Update,
				(
					hold::realize_held_grenade_visuals,
					tick_owned_recharge,
					despawn_orphaned_held_grenades,
				),
			);
	}
}

pub fn teardown_held_grenade(commands: &mut Commands, user: Entity, grenade: &GrenadeUser) {
	commands.entity(grenade.held).try_despawn();
	commands.entity(user).remove::<(GrenadeUser, GrenadeThrow, PlayerUse)>();
}

fn tick_owned_recharge(time: Res<Time>, mut bags: Query<&mut Inventory>) {
	let dt = time.delta_secs();
	for mut bag in &mut bags {
		for item in &mut bag.items {
			if let InventoryItem::Grenade { recharge, .. } = item {
				recharge.tick(dt);
			}
		}
	}
}

fn despawn_orphaned_held_grenades(
	mut commands: Commands,
	held: Query<Entity, (With<HeldGrenade>, Without<HeldByGrenade>)>,
) {
	for entity in &held {
		commands.entity(entity).try_despawn();
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use super::*;
	use character_items::{GrenadeRecharge, GrenadeSpec, GrenadeStats};
	use std::time::Duration;

	#[test]
	fn owned_recharge_ticks_while_unequipped() {
		let mut world = World::new();
		let mut time = Time::<()>::default();
		time.advance_by(Duration::from_millis(500));
		world.insert_resource(time);
		world.spawn(Inventory {
			items: vec![InventoryItem::Grenade {
				spec: GrenadeSpec::standard(),
				stats: GrenadeStats::standard(),
				recharge: GrenadeRecharge { remaining: 1.0 },
			}],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
		});
		world.run_system_once(tick_owned_recharge).expect("tick");
		let bag = world.query::<&Inventory>().single(&world).expect("bag");
		let InventoryItem::Grenade { recharge, .. } = &bag.items[0] else {
			panic!("grenade");
		};
		assert!(recharge.remaining < 1.0);
	}

	#[test]
	fn orphaned_held_grenade_despawns_when_the_user_is_gone() {
		let mut world = World::new();
		let held = world.spawn(HeldGrenade { scale: 1.0 }).id();
		world.run_system_once(despawn_orphaned_held_grenades).expect("orphan");
		assert!(!world.entities().contains(held));
	}

	#[test]
	fn held_grenade_stays_while_linked() {
		let mut world = World::new();
		let held = world.spawn(HeldGrenade { scale: 1.0 }).id();
		world.spawn(GrenadeUser::holding(held));
		world.flush();
		world.run_system_once(despawn_orphaned_held_grenades).expect("linked");
		assert!(world.entities().contains(held));
	}
}
