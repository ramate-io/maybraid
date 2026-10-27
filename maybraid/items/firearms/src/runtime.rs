//! Firearm execution contract: armed flag, cadence, and the fire clock.
//!
//! Shot spawn / muzzle / impact stay in [`crate::presentation`]. Headless
//! tests and production run [`crate::projectiles::decide_weapon_fire`].

use bevy::prelude::*;

use crate::cadence::{WeaponFired, WeaponShotRequest};
use crate::projectiles::{
	decide_weapon_fire, sample_weapon_launch_poses, FirearmWeaponSystems, WeaponsArmed,
};

#[derive(Resource, Default)]
struct FirearmRuntime;

/// Messages, armed flag, and the authoritative fire clock.
/// Does not spawn meshes or register `fire_weapons`.
pub fn register_firearm_runtime(app: &mut App) {
	if app.world().contains_resource::<FirearmRuntime>() {
		return;
	}
	maybraid_spawns_projectiles::register_projectile_spawn_runtime(app);
	app.init_resource::<FirearmRuntime>()
		.init_resource::<WeaponsArmed>()
		.add_message::<WeaponFired>()
		.add_message::<WeaponShotRequest>()
		.add_systems(
			PostUpdate,
			(
				sample_weapon_launch_poses,
				decide_weapon_fire,
				maybraid_spawns_projectiles::realize_projectiles,
			)
				.chain()
				.in_set(FirearmWeaponSystems::Fire),
		);
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cadence::{FireControl, WeaponShotRequest};
	use crate::projectiles::{FireOnTrigger, Weapon, WeaponTrigger};
	use crate::WeaponRecoil;
	use bevy::ecs::system::RunSystemOnce;

	fn tick_fire(app: &mut App) {
		app.world_mut()
			.resource_mut::<Time>()
			.advance_by(std::time::Duration::from_secs_f32(1.0 / 30.0));
		app.world_mut().run_system_once(decide_weapon_fire).expect("decide weapon fire");
	}

	#[test]
	fn decide_weapon_fire_emits_one_shot_request() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_firearm_runtime(&mut app);
		let gun = app
			.world_mut()
			.spawn((
				Weapon::bolt(),
				WeaponTrigger(true),
				FireOnTrigger,
				FireControl::auto(),
				WeaponRecoil(0.08),
				crate::projectiles::WeaponLaunchPose::ready_at(Vec3::ZERO, Vec3::Y),
			))
			.id();
		tick_fire(&mut app);
		let shots: Vec<WeaponShotRequest> =
			app.world_mut().resource_mut::<Messages<WeaponShotRequest>>().drain().collect();
		assert_eq!(shots.len(), 1);
		assert_eq!(shots[0].weapon, gun);
		assert!((shots[0].recoil - 0.08).abs() < 1e-5);
		let fired: Vec<WeaponFired> =
			app.world_mut().resource_mut::<Messages<WeaponFired>>().drain().collect();
		assert_eq!(fired.len(), 1);
		assert_eq!(fired[0].shooter, gun);
	}

	#[test]
	fn a_missing_launch_pose_does_not_consume_a_shot() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_firearm_runtime(&mut app);
		app.world_mut().spawn((
			Weapon::bolt(),
			WeaponTrigger(true),
			FireOnTrigger,
			FireControl::auto(),
			WeaponRecoil(0.08),
		));
		tick_fire(&mut app);
		assert!(app.world_mut().resource_mut::<Messages<WeaponShotRequest>>().drain().count() == 0);
		assert!(app.world_mut().resource_mut::<Messages<WeaponFired>>().drain().count() == 0);
		assert!(
			app.world_mut()
				.resource_mut::<Messages<maybraid_spawns_projectiles::SpawnProjectile>>()
				.drain()
				.count() == 0
		);
	}

	#[test]
	fn fire_decision_realizes_projectiles_in_the_same_frame() {
		use projectiles::Flight;

		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_firearm_runtime(&mut app);
		app.world_mut().spawn((
			Weapon::bolt(),
			WeaponTrigger(true),
			FireOnTrigger,
			FireControl::auto(),
			crate::projectiles::WeaponLaunchPose::ready_at(Vec3::ZERO, Vec3::Y),
		));
		app.world_mut()
			.resource_mut::<Time>()
			.advance_by(std::time::Duration::from_secs_f32(1.0 / 30.0));
		app.update();
		let flights = app
			.world_mut()
			.run_system_once(|flights: Query<&Flight>| flights.iter().count())
			.expect("count flights");
		assert_eq!(flights, 1);
	}

	#[test]
	fn production_registration_attaches_visuals_after_sim() {
		use bevy::prelude::Mesh3d;
		use projectiles::Flight;

		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_firearm_runtime(&mut app);
		maybraid_spawns_projectiles::register_projectile_presentation(&mut app);
		app.init_resource::<Assets<Mesh>>();
		app.init_resource::<Assets<StandardMaterial>>();
		app.world_mut().spawn((
			Weapon::bolt(),
			WeaponTrigger(true),
			FireOnTrigger,
			FireControl::auto(),
			crate::projectiles::WeaponLaunchPose::ready_at(Vec3::ZERO, Vec3::Y),
		));
		app.world_mut()
			.resource_mut::<Time>()
			.advance_by(std::time::Duration::from_secs_f32(1.0 / 30.0));
		app.update();
		let (flights, meshes) = app
			.world_mut()
			.run_system_once(|flights: Query<(Entity, Option<&Mesh3d>), With<Flight>>| {
				let count = flights.iter().count();
				let meshed = flights.iter().filter(|(_, mesh)| mesh.is_some()).count();
				(count, meshed)
			})
			.expect("count flights");
		assert_eq!(flights, 1);
		assert_eq!(meshes, 1);
	}

	#[test]
	fn register_firearm_runtime_is_idempotent() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_firearm_runtime(&mut app);
		register_firearm_runtime(&mut app);
		app.world_mut().spawn((
			Weapon::bolt(),
			WeaponTrigger(true),
			FireOnTrigger,
			crate::projectiles::WeaponLaunchPose::ready_at(Vec3::ZERO, Vec3::Y),
		));
		tick_fire(&mut app);
		assert_eq!(
			app.world_mut().resource_mut::<Messages<WeaponShotRequest>>().drain().count(),
			1
		);
	}
}
