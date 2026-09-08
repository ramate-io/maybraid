//! Application-owned budgets and cadence for world NPC intelligence.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use chico_vegetation_on_terrain_playground::Player as VegetationPlayer;
use evasion_intelligence::{EvasionPlugin, EvasionSystems};
use firearm_intelligence::{FirearmIntelligencePlugin, FirearmIntelligenceSystems};
use firearm_user::FirearmUserPlugin;
use firearms::{FirearmWeaponSystems, FirearmWeaponsPlugin};
use fleeing_intelligence::{FleeingPlugin, FleeingSystems};
use hiding_intelligence::{HidingPlugin, HidingSystems};
use intelligence_lod::{IntelligenceBand, IntelligenceLod, IntelligencePriority};
use maybraid_mobs::player_affiliations;
use meandering_intelligence::MeanderingIntelligencePlugin;
use movement_intelligence::{
	CandidateBudget, MovementIntelligenceLimits, MovementIntelligencePlugin,
};
use movement_intelligence_avian::AvianMovementSurface;
use movement_realization::MovementRealizationPlugin;
use player::LocomotionCapsule;
use poi_intelligence::PoiSystems;
use routing_intelligence::{RoutingPlugin, RoutingSystems};
use spotting_intelligence::{InterestLayers, SpotBounds, SpotSubject, SpottingSystems};
use tether_intelligence::TetherSystems;
use threat_intelligence::{
	Affiliations, ThreatId, ThreatIntelligencePlugin, ThreatSubject, ThreatSystems,
};
use threat_intelligence_damage::ThreatIntelligenceDamagePlugin;
use threat_management_intelligence::{
	ThreatManagementIntelligence, ThreatManagementPlugin, ThreatTactic,
};

const INTELLIGENCE_LOD_REFRESH_INTERVAL: Duration = Duration::from_millis(250);

const WORLD_MOVEMENT_LIMITS: MovementIntelligenceLimits = MovementIntelligenceLimits {
	max_budget: CandidateBudget { max_candidates: 8, max_steps: 3, horizon: 28.0 },
	max_replans_per_frame: 4,
	max_walk_probes_per_frame: 8,
};

type WorldPlayers<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		Option<&'static SpotSubject>,
		Option<&'static ThreatSubject>,
		Option<&'static Affiliations>,
	),
	With<VegetationPlayer>,
>;

pub struct WorldIntelligencePlugin;

impl Plugin for WorldIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(WORLD_MOVEMENT_LIMITS)
			.init_resource::<IntelligencePriority>();
		if !app.is_plugin_added::<FirearmWeaponsPlugin>() {
			app.add_plugins(FirearmWeaponsPlugin);
		}
		if !app.is_plugin_added::<FirearmUserPlugin>() {
			app.add_plugins(FirearmUserPlugin);
		}
		if !app.is_plugin_added::<MovementIntelligencePlugin<AvianMovementSurface<'_, '_>>>() {
			app.add_plugins(MovementIntelligencePlugin::<AvianMovementSurface<'_, '_>>::default());
		}
		if !app.is_plugin_added::<FirearmIntelligencePlugin>() {
			app.add_plugins(FirearmIntelligencePlugin);
		}
		if !app.is_plugin_added::<ThreatIntelligencePlugin>() {
			app.add_plugins(ThreatIntelligencePlugin);
		}
		if !app.is_plugin_added::<ThreatIntelligenceDamagePlugin>() {
			app.add_plugins(ThreatIntelligenceDamagePlugin);
		}
		if !app.is_plugin_added::<ThreatManagementPlugin>() {
			app.add_plugins(ThreatManagementPlugin);
		}
		if !app.is_plugin_added::<EvasionPlugin>() {
			app.add_plugins(EvasionPlugin);
		}
		if !app.is_plugin_added::<FleeingPlugin>() {
			app.add_plugins(FleeingPlugin);
		}
		if !app.is_plugin_added::<HidingPlugin>() {
			app.add_plugins(HidingPlugin);
		}
		if !app.is_plugin_added::<MeanderingIntelligencePlugin>() {
			app.add_plugins(MeanderingIntelligencePlugin);
		}
		if !app.is_plugin_added::<RoutingPlugin>() {
			app.add_plugins(RoutingPlugin);
		}
		if !app.is_plugin_added::<MovementRealizationPlugin>() {
			app.add_plugins(MovementRealizationPlugin);
		}
		app.add_systems(Update, sync_world_player_threat_actor.in_set(ThreatSystems::Prepare))
			.configure_sets(
				Update,
				SpottingSystems::Observe.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				FirearmIntelligenceSystems::Spotting.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				FirearmIntelligenceSystems::Movement.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				(EvasionSystems::Ingest, EvasionSystems::Rank)
					.chain()
					.after(SpottingSystems::Observe)
					.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				(FleeingSystems::Write, HidingSystems::Write)
					.chain()
					.after(EvasionSystems::Rank)
					.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(Update, PoiSystems::Select.run_if(on_timer(Duration::from_millis(200))))
			.configure_sets(
				Update,
				(
					TetherSystems::Write.run_if(on_timer(Duration::from_millis(250))),
					RoutingSystems::Plan.run_if(on_timer(Duration::from_millis(250))),
				),
			)
			.add_systems(
				Update,
				bake_intelligence_lod.run_if(on_timer(INTELLIGENCE_LOD_REFRESH_INTERVAL)),
			)
			.configure_sets(
				PostUpdate,
				FirearmIntelligenceSystems::ValidateAim.run_if(on_timer(Duration::from_millis(33))),
			)
			.configure_sets(
				PostUpdate,
				FirearmIntelligenceSystems::Fire.run_if(on_timer(Duration::from_millis(33))),
			)
			.configure_sets(
				PostUpdate,
				FirearmWeaponSystems::Fire.run_if(on_timer(Duration::from_millis(33))),
			);
	}
}

fn bake_intelligence_lod(
	player: Query<&GlobalTransform, With<VegetationPlayer>>,
	mut plants: Query<(
		Entity,
		&GlobalTransform,
		&mut IntelligenceLod,
		&ThreatManagementIntelligence,
	)>,
	mut priority: ResMut<IntelligencePriority>,
) {
	let Ok(player) = player.single() else {
		return;
	};
	let player_xz = player.translation().xz();
	let mut rows: Vec<(Entity, IntelligenceLod)> = Vec::new();
	for (entity, transform, mut lod, management) in &mut plants {
		let dist = transform.translation().xz().distance(player_xz);
		lod.band = IntelligenceBand::from_viewer(dist, management.tactic != ThreatTactic::Ignore);
		lod.skips = lod.skips.saturating_add(1);
		rows.push((entity, *lod));
	}
	rows.sort_by_key(|&(entity, lod)| lod.bake_key(entity));
	priority.rank.clear();
	for (i, (entity, _)) in rows.into_iter().enumerate() {
		priority.rank.insert(entity, i as u32);
	}
}

fn sync_world_player_threat_actor(mut commands: Commands, players: WorldPlayers) {
	let hull = LocomotionCapsule::HUMANOID;
	let spot = SpotSubject::new(
		InterestLayers::CHARACTER,
		SpotBounds::capsule(hull.radius, hull.half_height()),
	);
	for (entity, current_spot, current_subject, current_affiliations) in &players {
		let id = ThreatId(entity.to_bits());
		let subject = ThreatSubject::new(id);
		let mut entity_commands = commands.entity(entity);
		if current_spot != Some(&spot) {
			entity_commands.insert(spot);
		}
		if current_subject != Some(&subject) {
			entity_commands.insert(subject);
		}
		if current_affiliations.is_none() {
			entity_commands.insert(player_affiliations(id));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use damage::DamageApplied;
	use maybraid_mobs::{FFA_GROUP, MobBrain, MobKind, PLAYER_GROUP};
	use threat_intelligence::{ThreatIntelligenceUser, ThreatKnowledge};

	#[test]
	fn bake_orders_near_before_far_and_promotes_high_skips() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<IntelligencePriority>()
			.add_systems(Update, bake_intelligence_lod);
		app.world_mut()
			.spawn((VegetationPlayer, Transform::default(), GlobalTransform::default()));
		let near = spawn_lod_plant(&mut app, Vec3::X * 40.0, ThreatTactic::Ignore, 0);
		let mid = spawn_lod_plant(&mut app, Vec3::X * 120.0, ThreatTactic::Ignore, 0);
		let far_fresh = spawn_lod_plant(&mut app, Vec3::X * 300.0, ThreatTactic::Ignore, 0);
		let far_waiting = spawn_lod_plant(
			&mut app,
			Vec3::X * 320.0,
			ThreatTactic::Ignore,
			IntelligenceLod::FAIRNESS_CAP,
		);
		let combat = spawn_lod_plant(&mut app, Vec3::X * 300.0, ThreatTactic::Combat, 0);

		app.update();

		let world = app.world();
		assert_eq!(
			world.get::<IntelligenceLod>(near).map(|lod| lod.band),
			Some(IntelligenceBand::Near)
		);
		assert_eq!(
			world.get::<IntelligenceLod>(mid).map(|lod| lod.band),
			Some(IntelligenceBand::Mid)
		);
		assert_eq!(
			world.get::<IntelligenceLod>(far_fresh).map(|lod| lod.band),
			Some(IntelligenceBand::Far)
		);
		assert_eq!(
			world.get::<IntelligenceLod>(combat).map(|lod| lod.band),
			Some(IntelligenceBand::Near)
		);
		assert_eq!(world.get::<IntelligenceLod>(near).map(|lod| lod.skips), Some(1));
		let rank = &world.resource::<IntelligencePriority>().rank;
		assert!(rank[&near] < rank[&mid]);
		assert!(rank[&mid] < rank[&far_waiting]);
		assert!(rank[&far_waiting] < rank[&far_fresh]);
		assert!(rank[&combat] < rank[&mid]);
	}

	#[test]
	fn missing_lod_is_treated_as_near() {
		assert_eq!(IntelligenceLod::band_or_near(None), IntelligenceBand::Near);
	}

	fn spawn_lod_plant(app: &mut App, at: Vec3, tactic: ThreatTactic, skips: u8) -> Entity {
		let mut management = ThreatManagementIntelligence::default();
		management.tactic = tactic;
		app.world_mut()
			.spawn((
				GlobalTransform::from_translation(at),
				IntelligenceLod { band: IntelligenceBand::Near, skips },
				management,
			))
			.id()
	}

	#[test]
	fn world_replans_drain_instead_of_resolving_every_marker() {
		assert_eq!(WORLD_MOVEMENT_LIMITS.max_replans_per_frame, 4);
		assert_eq!(WORLD_MOVEMENT_LIMITS.max_walk_probes_per_frame, 8);
		assert!(
			WORLD_MOVEMENT_LIMITS.max_replans_per_frame
				< MovementIntelligenceLimits::default().max_replans_per_frame
		);
	}

	#[test]
	fn world_player_is_registered_for_spotting_and_threats() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_systems(Update, sync_world_player_threat_actor);
		let player = app.world_mut().spawn((VegetationPlayer, Transform::default())).id();

		app.update();

		let entity = app.world().entity(player);
		assert!(entity.get::<SpotSubject>().is_some());
		assert_eq!(
			entity.get::<ThreatSubject>().map(|subject| subject.id),
			Some(ThreatId(player.to_bits()))
		);
		let Some(affiliations) = entity.get::<Affiliations>() else {
			panic!("player affiliations were not installed");
		};
		assert!(affiliations.memberships.contains_key(&PLAYER_GROUP));
		assert!(affiliations.memberships.contains_key(&FFA_GROUP));
		assert!(affiliations.known_antagonists.contains_key(&FFA_GROUP));
	}

	#[test]
	fn generated_mob_discovers_nearby_world_player() {
		let mut app = threat_app();
		let player = spawn_world_player(&mut app);
		let mob_id = ThreatId(11);
		let affiliations = MobBrain::for_kind(MobKind::Herd).affiliations.for_member(mob_id);
		let mob = app
			.world_mut()
			.spawn((
				ThreatSubject::new(mob_id),
				affiliations,
				ThreatIntelligenceUser::default(),
				ThreatKnowledge::default(),
				Transform::from_xyz(4.0, 0.0, 0.0),
				GlobalTransform::from_xyz(4.0, 0.0, 0.0),
			))
			.id();

		app.update();

		let player_id = ThreatId(player.to_bits());
		assert!(
			app.world()
				.get::<ThreatKnowledge>(mob)
				.is_some_and(|knowledge| { knowledge.get(player_id).is_some() })
		);
	}

	#[test]
	fn damage_from_world_player_enters_victim_threat_knowledge() {
		let mut app = threat_app();
		let player = spawn_world_player(&mut app);
		let victim_id = ThreatId(13);
		let victim = app
			.world_mut()
			.spawn((
				ThreatSubject::new(victim_id),
				Affiliations::with_self(victim_id),
				ThreatIntelligenceUser::default(),
				ThreatKnowledge::default(),
				Transform::from_xyz(4.0, 0.0, 0.0),
				GlobalTransform::from_xyz(4.0, 0.0, 0.0),
			))
			.id();
		app.update();
		assert!(
			app.world()
				.get::<ThreatKnowledge>(victim)
				.is_some_and(ThreatKnowledge::is_empty)
		);

		app.world_mut().write_message(DamageApplied {
			target: victim,
			source: Some(player),
			amount: 10.0,
			remaining: 90.0,
			point: Vec3::ZERO,
		});
		app.update();
		app.update();

		let player_id = ThreatId(player.to_bits());
		assert!(
			app.world()
				.get::<ThreatKnowledge>(victim)
				.is_some_and(|knowledge| { knowledge.get(player_id).is_some() })
		);
	}

	fn threat_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TransformPlugin, ThreatIntelligenceDamagePlugin))
			.add_message::<DamageApplied>()
			.add_systems(Update, sync_world_player_threat_actor.in_set(ThreatSystems::Prepare));
		app
	}

	fn spawn_world_player(app: &mut App) -> Entity {
		app.world_mut()
			.spawn((VegetationPlayer, Transform::default(), GlobalTransform::default()))
			.id()
	}
}
