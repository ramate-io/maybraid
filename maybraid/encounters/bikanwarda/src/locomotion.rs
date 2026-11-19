//! Drive the encounter actor through the shared NPC movement stack.
//!
//! The pillar stays a static [`BossColumn`]. The child physics actor uses
//! [`player::apply_character_controller`], a [`tether_intelligence`] leash around
//! that pillar, meandering patrol pins, and firearm movement toward spotted
//! combat contacts. Gait comes from [`player::drive_player_locomotion`].

use std::f32::consts::TAU;

use avian3d::prelude::CollisionLayers;
use bevy::prelude::*;
use bikanwarda_character::{BikanwardaActorRecipe, BikanwardaRoot, BikanwardaSpec};
use characters::{CharacterHeading, CharacterRecipe, CharacterRoot};
use combat_targeting::CombatTargeting;
use firearm_intelligence::{
	FirearmEngagement, FirearmIntelligence, FirearmIntelligencePlugin, FirearmIntelligenceSystems,
	FirearmMovementIntelligence, FirearmMovementIntelligenceSettings, FirearmTargeting,
};
use lod_avian::PhysicsInteractionLayer;
use meandering_intelligence::{MeanderingIntelligencePlugin, MeanderingIntelligenceUser};
use movement_intelligence::{
	MovementIntelligence, MovementIntelligencePlugin, MovementLocation, MovementObjective,
	ReplanMovement, VantageStandoffs,
};
use movement_intelligence_avian::AvianMovementSurface;
use movement_realization::MovementRealizationPlugin;
use npc_intelligence::{NpcIntelligence, NpcIntelligencePlugin};
use player::{
	Npc, PlayerLook, PlayerPlugin, PlayerYawOwner, apply_character_controller,
	apply_character_mobility,
};
use poi_intelligence::{
	PoiId, PoiIntelligencePlugin, PoiIntelligenceUser, PoiInterests, PoiKind, PoiKnowledge,
	PoiObservation, PoiSource, PoiVisitState,
};
use tether_intelligence::{
	Tether, TetherIntelligenceUser, TetherObjective, TetherPlugin, install_tether,
};

use crate::presentation::{BossActor, BossColumn, BossEncounterRoot};

/// Horizontal leash around the pillar. Patrol pins sit on the authored 8 m ring.
pub const BOSS_TETHER_RADIUS: f32 = 24.0;

const PATROL_KIND: PoiKind = PoiKind::new("bikanwarda/patrol");
const PATROL_RING_M: f32 = 8.0;
const PATROL_PIN_COUNT: u32 = 4;
const PATROL_ARRIVAL_RADIUS: f32 = 5.0;

pub struct BossEncounterLocomotionPlugin;

impl Plugin for BossEncounterLocomotionPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<TetherPlugin>() {
			app.add_plugins(TetherPlugin);
		}
		if !app.is_plugin_added::<PoiIntelligencePlugin>() {
			app.add_plugins(PoiIntelligencePlugin);
		}
		if !app.is_plugin_added::<MeanderingIntelligencePlugin>() {
			app.add_plugins(MeanderingIntelligencePlugin);
		}
		if !app.is_plugin_added::<NpcIntelligencePlugin>() {
			app.add_plugins(NpcIntelligencePlugin);
		}
		app.add_systems(
			Update,
			(
				initialize_boss_locomotion.after(crate::combat::initialize_boss_combat),
				orient_boss_toward_spotted_target
					.after(combat_targeting::CombatTargetingSystems::Rank)
					.after(FirearmIntelligenceSystems::Orient),
			),
		);
	}
}

pub fn initialize_boss_locomotion(
	mut commands: Commands,
	new_bindings: Query<
		(Entity, &BossActor, &GlobalTransform, &Children),
		(Added<BossActor>, With<BossEncounterRoot>),
	>,
	columns: Query<(Entity, &GlobalTransform), With<BossColumn>>,
	specs: Query<&BikanwardaSpec, With<BikanwardaRoot>>,
	already_motor: Query<(), With<player::CharacterController>>,
) {
	for (host, actor_ref, host_global, children) in &new_bindings {
		if already_motor.contains(actor_ref.0) {
			continue;
		}
		let Some((column_entity, column_global)) =
			children.iter().find_map(|child| columns.get(child).ok())
		else {
			warn!("Could not find column child for locomotion host {:?}", host);
			continue;
		};

		let spec = specs.get(actor_ref.0).cloned().unwrap_or_else(|_| BikanwardaSpec::encounter());
		let hull = BikanwardaActorRecipe::from_spec(spec).components().scaled_locomotion_capsule();
		apply_character_controller(&mut commands, actor_ref.0, hull);
		apply_character_mobility(&mut commands, actor_ref.0, 1.0, 1.0);
		commands.entity(actor_ref.0).insert((
			Npc,
			PlayerLook::default(),
			PlayerYawOwner::Wish,
			CollisionLayers::new(
				[PhysicsInteractionLayer::Fixed, PhysicsInteractionLayer::Animated],
				[PhysicsInteractionLayer::Fixed, PhysicsInteractionLayer::Animated],
			),
			ReplanMovement,
		));

		let actor_y = host_global.translation().y;
		let column_xz =
			Vec3::new(column_global.translation().x, actor_y, column_global.translation().z);
		commands.entity(column_entity).insert(Tether);

		let objective = TetherObjective::Tether(column_entity, BOSS_TETHER_RADIUS);
		let mut tether = TetherIntelligenceUser::new(objective).with_added_radius(2.0);
		tether.enabled = true;
		install_tether(&mut commands, actor_ref.0, tether);

		let mut movement = MovementIntelligence::new(MovementObjective::Reach(
			MovementLocation::new(host_global.translation(), hull.radius.max(0.4)),
		));
		movement.ability.agent_radius = hull.radius.max(0.4);
		movement.ability.feet_below_origin = hull.half_height();
		movement.ability.eye_height = (hull.length + 2.0 * hull.radius) * 0.8;
		movement.ability.path_segment = (hull.radius * 2.0).max(4.0);
		movement.ability.max_step = hull.radius.max(0.4);
		movement.ability.max_fall = hull.half_height().max(1.2);
		movement.ability.candidate_budget.max_candidates = 8;
		movement.ability.candidate_budget.horizon = 30.0;
		movement.ability.vantage_standoffs = VantageStandoffs::from_radii(&[8.0, 12.0, 16.0]);
		movement.ability.vantage_azimuths = 4;

		let mut knowledge = PoiKnowledge::default();
		for i in 0..PATROL_PIN_COUNT {
			let angle = TAU * i as f32 / PATROL_PIN_COUNT as f32;
			let position = column_xz + Vec3::new(angle.cos(), 0.0, angle.sin()) * PATROL_RING_M;
			let mut source = PoiSource::OBJECTIVE;
			source.insert(PoiSource::EXTERNAL);
			let observation = PoiObservation {
				user: actor_ref.0,
				id: PoiId(0xB1A1_0001 + u64::from(i)),
				entity: None,
				kind: PATROL_KIND,
				position,
				arrival_radius: PATROL_ARRIVAL_RADIUS,
				salience: 1.0,
				confidence: 1.0,
				source,
			};
			knowledge.observe(observation, 0.0);
		}

		let mut meandering = MeanderingIntelligenceUser::new(BOSS_TETHER_RADIUS);
		meandering.linger_secs = 1.5;
		meandering.selection_interval = 0.25;
		meandering.enabled = true;

		let mut firearm_movement = FirearmMovementIntelligence::new();
		firearm_movement.settings = FirearmMovementIntelligenceSettings {
			range: (14.0, 1.0),
			cover: 0.0,
			flee: (0.0, 8.0),
			vantage: (0.0, 1.0),
		};

		commands.entity(actor_ref.0).insert((
			NpcIntelligence {
				idle_tether: Some(objective),
				engaged_tether: Some(objective),
				keep_tether_in_combat: true,
			},
			movement,
			meandering,
			PoiIntelligenceUser::new(PoiInterests::one(PATROL_KIND)),
			knowledge,
			PoiVisitState::default(),
			FirearmIntelligence::new(),
			firearm_movement,
			FirearmTargeting::default(),
			FirearmEngagement::hold(),
		));
	}
}

/// Face the spotted combat contact. Uses [`CombatTargeting`] filled by the
/// existing spotting bridge — no new visibility.
fn orient_boss_toward_spotted_target(
	time: Res<Time>,
	actors: Query<(&CombatTargeting, &GlobalTransform), With<BikanwardaRoot>>,
	targets: Query<&GlobalTransform>,
	mut visuals: Query<
		(&ChildOf, &mut Transform, &mut CharacterHeading, Option<&mut PlayerYawOwner>),
		With<CharacterRoot>,
	>,
) {
	let amount = (time.delta_secs() * 5.0).clamp(0.0, 1.0);
	for (child_of, mut visual, mut heading, yaw_owner) in &mut visuals {
		let Ok((targeting, actor_gt)) = actors.get(child_of.parent()) else {
			continue;
		};
		let Some(contact) = targeting.best_contact() else {
			if let Some(mut yaw_owner) = yaw_owner {
				if *yaw_owner != PlayerYawOwner::Wish {
					*yaw_owner = PlayerYawOwner::Wish;
				}
			}
			continue;
		};
		let Ok(target_gt) = targets.get(contact.subject) else {
			continue;
		};
		if let Some(mut yaw_owner) = yaw_owner {
			if *yaw_owner != PlayerYawOwner::Look {
				*yaw_owner = PlayerYawOwner::Look;
			}
		}
		let to = target_gt.translation() - actor_gt.translation();
		let forward = Vec3::new(to.x, 0.0, to.z);
		if forward.length_squared() < 1e-4 {
			continue;
		}
		if !heading.would_turn_toward(&visual, forward) {
			continue;
		}
		let current = heading.facing(&visual);
		heading.set(&mut visual, current.slerp(forward.normalize(), amount));
	}
}

/// Test helper: install the shared motor / movement stack used by world NPCs.
///
/// Production installs these via the world intelligence plugin and [`PlayerPlugin`].
/// Outcome-only apps must not pull Avian spatial-query systems.
pub fn add_player_motor_plugins(app: &mut App) {
	if !app.is_plugin_added::<bevy::input::InputPlugin>() {
		app.add_plugins(bevy::input::InputPlugin);
	}
	if !app.is_plugin_added::<PlayerPlugin>() {
		app.add_plugins(PlayerPlugin);
	}
	if !app.is_plugin_added::<MovementIntelligencePlugin<AvianMovementSurface<'_, '_>>>() {
		app.add_plugins(MovementIntelligencePlugin::<AvianMovementSurface<'_, '_>>::default());
	}
	if !app.is_plugin_added::<FirearmIntelligencePlugin>() {
		app.add_plugins(FirearmIntelligencePlugin);
	}
	if !app.is_plugin_added::<MovementRealizationPlugin>() {
		app.add_plugins(MovementRealizationPlugin);
	}
}
