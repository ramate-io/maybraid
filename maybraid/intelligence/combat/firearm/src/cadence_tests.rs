//! Exercise the scheduled contact adapter and actual muzzle/fire runtime together.

use std::time::Duration;

use avian3d::prelude::PhysicsPlugins;
use bevy::prelude::*;
use bevy::time::{TimeUpdateStrategy, common_conditions::on_timer};
use combat_targeting::{CombatTargeting, TargetSource};
use firearm_user::FirearmUser;
use firearms::{
	BoneMap, FireControl, FireOnTrigger, MemberOf, ProjectileSource, RigRoot, Weapon, WeaponFired,
	WeaponLaunchPose, WeaponTrigger, register_firearm_runtime,
};
use movement_intelligence::{MovementIntelligence, MovementLocation, MovementObjective};
use player::PlayerLook;
use spotting_intelligence::{
	InterestLayers, SpotBounds, SpotDirective, SpotSubject, SpottedContact, SpottingObserveLimits,
	SpottingSettings, SpottingSystems, SpottingUser,
};

use crate::{FirearmIntelligence, FirearmIntelligencePlugin, FirearmTargeting};

fn app(retained_on_timer: bool) -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		PhysicsPlugins::default(),
		bevy::asset::AssetPlugin::default(),
		bevy::mesh::MeshPlugin,
		FirearmIntelligencePlugin,
	))
	.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(16)))
	.insert_resource(SpottingObserveLimits {
		max_observers_per_tick: 0,
		max_retained_observers_per_tick: 1,
		max_discovery_samples_per_tick: 80,
		max_retained_samples_per_tick: 8,
	})
	.configure_sets(Update, SpottingSystems::Discover.run_if(on_timer(Duration::from_millis(125))));
	if retained_on_timer {
		// Reproduce the previous application cadence without changing library code.
		app.configure_sets(
			Update,
			SpottingSystems::Refresh.run_if(on_timer(Duration::from_millis(125))),
		);
	}
	register_firearm_runtime(&mut app);
	app.finish();
	app
}

fn combatant(app: &mut App, target_at: Vec3) -> (Entity, Entity, Entity) {
	let target = app
		.world_mut()
		.spawn((
			Transform::from_translation(target_at),
			SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 0.9)),
			// Peers remain valid subjects; their Transform is intentionally excluded
			// from the firearm aim system's noncombatant-body query.
			FirearmIntelligence::new(),
		))
		.id();
	let pivot = Vec3::Y * 1.6;
	let gun = app
		.world_mut()
		.spawn((
			Transform::from_translation(pivot),
			Weapon::bullet(),
			FireOnTrigger,
			FireControl::auto(),
			WeaponTrigger(false),
			WeaponLaunchPose::pending(),
		))
		.id();
	let barrel = app
		.world_mut()
		.spawn(
			Transform::from_translation(pivot)
				.with_rotation(Quat::from_rotation_arc(Vec3::Y, (target_at - pivot).normalize())),
		)
		.id();
	app.world_mut().spawn((
		MemberOf(gun),
		RigRoot::default(),
		BoneMap { by_name: [("barrel".to_string(), barrel)].into_iter().collect() },
	));
	let directive = SpotDirective {
		discovery_interval_secs: 0.125,
		respot_interval_secs: 0.125,
		..SpotDirective::new(InterestLayers::CHARACTER, 40.0)
	};
	let mut spotting =
		SpottingUser::new(pivot, [directive]).with_settings(SpottingSettings::new(8, 8, 2.5));
	spotting.next_discovery_at = 10_000.0;
	spotting.contacts.insert(
		target,
		SpottedContact::new(target, target_at, Vec3::ZERO, target_at, None, -1.0, 0.125),
	);
	let mut targeting = CombatTargeting::default();
	targeting.include(target, TargetSource::ENEMYSHIP);
	let mut brain = FirearmIntelligence::new();
	brain.settings.accuracy = 1.0;
	brain.settings.headshots = 0.0;
	// Acquisition is slightly longer than the 200 ms sight freshness window.
	// Continuous valid refreshes can acquire; isolated 500 ms observations cannot.
	brain.settings.trigger_happiness = 0.55;
	let actor = app
		.world_mut()
		.spawn((
			Transform::default(),
			MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(
				Vec3::ZERO,
				0.5,
			))),
			FirearmUser::holding(gun),
			brain,
			targeting,
			FirearmTargeting::default(),
			PlayerLook::default(),
			spotting,
		))
		.id();
	app.world_mut().entity_mut(gun).insert(ProjectileSource(actor));
	(actor, target, gun)
}

fn run(retained_on_timer: bool) -> anyhow::Result<Vec<usize>> {
	let mut app = app(retained_on_timer);
	// Close and raised/lowered targets cover the reported 5–30 m distances.
	// Static barrel bones start aligned: this fixture isolates sight continuity,
	// actual muzzle validation, acquisition, and emission from the pose renderer.
	let fixtures: Vec<_> = [(5.0, 0.0), (10.0, 3.0), (20.0, -3.0), (30.0, 1.0)]
		.into_iter()
		.map(|(distance, height)| combatant(&mut app, Vec3::new(0.0, height, -distance)))
		.collect();
	let mut shots = vec![0; fixtures.len()];
	let mut first_admitted = vec![None; fixtures.len()];
	for _ in 0..180 {
		app.update();
		let now = app.world().resource::<Time>().elapsed_secs();
		for (index, &(actor, target, _)) in fixtures.iter().enumerate() {
			let spotting = app
				.world()
				.get::<SpottingUser>(actor)
				.ok_or_else(|| anyhow::anyhow!("missing spotting user"))?;
			let sight = spotting
				.contacts
				.get(&target)
				.ok_or_else(|| anyhow::anyhow!("missing sight contact"))?;
			let combat = app
				.world()
				.get::<CombatTargeting>(actor)
				.ok_or_else(|| anyhow::anyhow!("missing combat targeting"))?;
			// No additional timer or Update separates the generic contact and memory.
			assert_eq!(
				combat.contact(target).map(|contact| contact.last_spotted_at),
				Some(sight.last_success_at)
			);
			if sight.last_success_at == now {
				first_admitted[index].get_or_insert(now);
				assert!(
					app.world()
						.get::<FirearmTargeting>(actor)
						.ok_or_else(|| anyhow::anyhow!("missing firearm targeting"))?
						.for_target(target)
						.any(|trajectory| trajectory.clear)
				);
			}
		}
		let emitted: Vec<_> =
			app.world_mut().resource_mut::<Messages<WeaponFired>>().drain().collect();
		for event in emitted {
			if let Some(index) = fixtures.iter().position(|(actor, _, _)| *actor == event.shooter) {
				shots[index] += 1;
				let admitted = first_admitted[index]
					.ok_or_else(|| anyhow::anyhow!("shot before any fresh clear admission"))?;
				assert!(
					now - admitted >= 0.2,
					"acquisition delay was skipped for actor {index}: sight={admitted}, shot={now}"
				);
			}
		}
	}
	Ok(shots)
}

#[test]
fn retained_updates_keep_close_and_vertical_targets_fresh_through_acquisition() -> anyhow::Result<()>
{
	assert!(run(false)?.iter().all(|count| *count > 0));
	Ok(())
}

#[test]
fn overloaded_125_ms_refresh_round_does_not_fire_on_stale_sight() -> anyhow::Result<()> {
	assert_eq!(run(true)?, vec![0; 4]);
	Ok(())
}

#[test]
fn newly_discovered_contact_reaches_combat_memory_in_the_same_update() -> anyhow::Result<()> {
	let mut app = app(false);
	app.world_mut().resource_mut::<SpottingObserveLimits>().max_observers_per_tick = 1;
	let at = Vec3::new(0.0, 2.0, -5.0);
	let target = app
		.world_mut()
		.spawn((
			Transform::from_translation(at),
			SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 0.9)),
		))
		.id();
	let mut user =
		SpottingUser::new(Vec3::Y * 1.6, [SpotDirective::new(InterestLayers::CHARACTER, 20.0)]);
	user.hint(target, spotting_intelligence::SpottingHint::new(1));
	let mut targeting = CombatTargeting::default();
	targeting.include(target, TargetSource::ENEMYSHIP);
	let actor = app
		.world_mut()
		.spawn((Transform::default(), user, FirearmIntelligence::new(), targeting))
		.id();
	for _ in 0..12 {
		app.update();
		let Some(contact) = app
			.world()
			.get::<SpottingUser>(actor)
			.and_then(|user| user.contacts.get(&target))
		else {
			continue;
		};
		let now = app.world().resource::<Time>().elapsed_secs();
		assert_eq!(contact.last_success_at, now);
		assert_eq!(
			app.world()
				.get::<CombatTargeting>(actor)
				.and_then(|targeting| targeting.contact(target))
				.map(|contact| contact.last_spotted_at),
			Some(now)
		);
		return Ok(());
	}
	anyhow::bail!("discovery never admitted the clear explicit-hint subject");
}
