//! Drive the encounter actor through the shared NPC movement stack.
//!
//! The pillar stays a static [`BossColumn`]. The child physics actor uses
//! [`player::apply_character_controller`], a [`tether_intelligence`] leash around
//! that pillar, meandering patrol pins, and firearm movement toward spotted
//! combat contacts. Gait comes from [`player::drive_player_locomotion`].

use std::collections::HashMap;
use std::f32::consts::TAU;

use avian3d::prelude::{CollisionLayers, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bikanwarda_character::{
	encounter_meters, hit_complex_collider, BikanwardaActorRecipe, BikanwardaAnchors,
	BikanwardaChainAnchor, BikanwardaRoot, BikanwardaSpec,
};
use characters::{CharacterHeading, CharacterRecipe, CharacterRoot, LocomotionCapsule};
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
	apply_character_controller, apply_character_mobility, bind_hurtbox_host, HitVolumeCollider,
	Npc, PlayerLook, PlayerPlugin, PlayerSystems, PlayerYawOwner,
};
use poi_intelligence::{
	PoiId, PoiIntelligencePlugin, PoiIntelligenceUser, PoiInterests, PoiKind, PoiKnowledge,
	PoiObservation, PoiSource, PoiSystems, PoiVisitState,
};
use tether_intelligence::{
	install_tether, Tether, TetherIntelligenceUser, TetherObjective, TetherPlugin,
};

use crate::presentation::{
	BossActor, BossColumn, BossEncounterRoot, BossPedestal, PEDESTAL_RADIUS,
};

/// Horizontal leash around the pillar. Patrol pins sit on the authored 8 m ring,
/// scaled with the shared encounter form.
pub const BOSS_TETHER_RADIUS: f32 = encounter_meters(24.0);

const PATROL_KIND: PoiKind = PoiKind::new("bikanwarda/patrol");
pub(crate) const PATROL_RING_M: f32 = encounter_meters(8.0);
pub(crate) const PATROL_PIN_COUNT: u32 = 4;
const PATROL_ARRIVAL_RADIUS: f32 = encounter_meters(5.0);
/// Retry cadence once SpatialQuery is present but some pins still lack Fixed support.
pub(crate) const PATROL_SNAP_RETRY_SECS: f32 = 0.25;

/// World-space patrol pin on the scaled ring around the live pillar.
///
/// Half-step rotation keeps the spawn pose (column offset along +X) off the
/// pins so `not_already_there` does not skip the first reachable goal.
/// Production observes snapped ground, not these geometric seeds; tests still
/// place pads and assert the live ring from this helper.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn patrol_pin_world(column_world: Vec3, actor_y: f32, index: u32) -> Vec3 {
	let angle = TAU * (index as f32 + 0.5) / PATROL_PIN_COUNT as f32;
	Vec3::new(column_world.x, actor_y, column_world.z)
		+ Vec3::new(angle.cos(), 0.0, angle.sin()) * PATROL_RING_M
}

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
				orient_boss_toward_spotted_target
					.after(combat_targeting::CombatTargetingSystems::Rank)
					.after(FirearmIntelligenceSystems::Orient)
					.before(PlayerSystems::Locomotion),
				align_boss_chain_anchors.after(PlayerSystems::Locomotion),
			),
		);
		// Plant pins from the live pillar's propagated world pose. `Added<BossActor>`
		// is written at the end of Update (assembly commands); reading column
		// GlobalTransform in that same Update sees the child's local offset and
		// plants goals around the origin instead of Area 1746.
		app.add_systems(PostUpdate, initialize_boss_locomotion.after(TransformSystems::Propagate));
		app.add_systems(Update, snap_boss_patrol_pins_to_fixed_ground.before(PoiSystems::Select));
	}
}

/// Marks patrol knowledge that has been snapped onto Fixed ground around the
/// live pillar. Geometric ring seeds are not walkable until this runs.
#[derive(Component, Clone, Copy, Debug, Default)]
struct PatrolPinsSnapped;

/// Pending Fixed-ground snaps. Selection only sees pins this state has observed.
#[derive(Component, Clone, Debug)]
pub(crate) struct PatrolPinSnapState {
	pending: u8,
	next_attempt_at: f32,
	pub(crate) attempts: u32,
}

impl PatrolPinSnapState {
	fn new() -> Self {
		Self { pending: (1u8 << PATROL_PIN_COUNT) - 1, next_attempt_at: 0.0, attempts: 0 }
	}

	fn is_pending(&self, index: u32) -> bool {
		index < PATROL_PIN_COUNT && self.pending & (1 << index) != 0
	}

	fn mark_snapped(&mut self, index: u32) {
		if index < PATROL_PIN_COUNT {
			self.pending &= !(1 << index);
		}
	}

	pub(crate) fn pending_count(&self) -> u32 {
		u32::from(self.pending.count_ones())
	}
}

pub(crate) fn patrol_pin_id(index: u32) -> PoiId {
	PoiId(0xB1A1_0001 + u64::from(index))
}

fn xz_distance(a: Vec3, b: Vec3) -> f32 {
	Vec2::new(a.x, a.z).distance(Vec2::new(b.x, b.z))
}

fn snap_xz_to_fixed(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	xz: Vec2,
	actor_y: f32,
	feet_below_origin: f32,
	max_fall: f32,
) -> Option<(Vec3, Entity)> {
	let lift = feet_below_origin.max(0.4) + 2.0;
	let origin = Vec3::new(xz.x, actor_y + lift, xz.y);
	let hit = spatial.cast_ray(origin, Dir3::NEG_Y, lift + max_fall + 2.0, true, filter)?;
	if hit.distance < 0.02 {
		return None;
	}
	Some((Vec3::new(xz.x, origin.y - hit.distance + feet_below_origin, xz.y), hit.entity))
}

fn reachable_patrol_pin(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	column: Vec3,
	actor_y: f32,
	hull: &LocomotionCapsule,
	index: u32,
	blocked: &[Entity],
) -> Option<Vec3> {
	let min_radial = PEDESTAL_RADIUS + hull.radius.max(0.4) + 0.15;
	let max_radial = (BOSS_TETHER_RADIUS - hull.radius.max(0.4)).max(min_radial + 0.35);
	let rings = [PATROL_RING_M, (PATROL_RING_M * 1.5).min(max_radial), max_radial];
	let mut angles = vec![TAU * (index as f32 + 0.5) / PATROL_PIN_COUNT as f32];
	for extra in 0..4u32 {
		angles.push(angles[0] + TAU * extra as f32 / 8.0);
	}
	for radius in rings {
		if radius < min_radial || radius > max_radial {
			continue;
		}
		for angle in &angles {
			let xz = Vec2::new(column.x + angle.cos() * radius, column.z + angle.sin() * radius);
			let Some((snapped, hit)) = snap_xz_to_fixed(
				spatial,
				filter,
				xz,
				actor_y,
				hull.half_height(),
				hull.radius.max(0.4) + encounter_meters(1.2),
			) else {
				continue;
			};
			if blocked.contains(&hit) {
				continue;
			}
			if xz_distance(snapped, column) < min_radial {
				continue;
			}
			return Some(snapped);
		}
	}
	None
}

fn snap_boss_patrol_pins_to_fixed_ground(
	time: Res<Time>,
	spatial: Option<SpatialQuery>,
	mut bosses: Query<
		(Entity, &GlobalTransform, &mut PoiKnowledge, &LocomotionCapsule, &mut PatrolPinSnapState),
		(With<BikanwardaRoot>, Without<PatrolPinsSnapped>),
	>,
	hosts: Query<(&BossActor, &Children), With<BossEncounterRoot>>,
	columns: Query<(Entity, &GlobalTransform), With<BossColumn>>,
	pedestals: Query<Entity, With<BossPedestal>>,
	mut commands: Commands,
) {
	let Some(spatial) = spatial else {
		return;
	};
	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);
	let now = time.elapsed_secs();
	for (entity, actor_gt, mut knowledge, hull, mut snap) in &mut bosses {
		if now < snap.next_attempt_at {
			continue;
		}
		let Some((_, children)) = hosts.iter().find(|(actor, _)| actor.0 == entity) else {
			continue;
		};
		let Some((column_entity, column_gt)) =
			children.iter().find_map(|child| columns.get(child).ok())
		else {
			continue;
		};
		let pedestal = children.iter().find(|&child| pedestals.contains(child));
		let mut blocked = vec![column_entity, entity];
		if let Some(pedestal) = pedestal {
			blocked.push(pedestal);
		}
		let column = column_gt.translation();
		let actor_y = actor_gt.translation().y;
		snap.attempts = snap.attempts.saturating_add(1);
		snap.next_attempt_at = now + PATROL_SNAP_RETRY_SECS;
		for i in 0..PATROL_PIN_COUNT {
			if !snap.is_pending(i) {
				continue;
			}
			let Some(position) =
				reachable_patrol_pin(&spatial, &filter, column, actor_y, hull, i, &blocked)
			else {
				continue;
			};
			let mut source = PoiSource::OBJECTIVE;
			source.insert(PoiSource::EXTERNAL);
			knowledge.observe(
				PoiObservation {
					user: entity,
					id: patrol_pin_id(i),
					entity: None,
					kind: PATROL_KIND,
					position,
					arrival_radius: PATROL_ARRIVAL_RADIUS,
					salience: 1.0,
					confidence: 1.0,
					source,
				},
				now,
			);
			snap.mark_snapped(i);
		}
		if snap.pending_count() == 0 {
			commands.entity(entity).insert(PatrolPinsSnapped);
		}
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
	visuals: Query<(Entity, &ChildOf), With<CharacterRoot>>,
) {
	for (host, actor_ref, host_global, children) in &new_bindings {
		if already_motor.contains(actor_ref.0) {
			continue;
		}
		let Some((column_entity, _column_global)) =
			children.iter().find_map(|child| columns.get(child).ok())
		else {
			warn!("Could not find column child for locomotion host {:?}", host);
			continue;
		};

		let spec = specs.get(actor_ref.0).cloned().unwrap_or_else(|_| BikanwardaSpec::encounter());
		let hull = BikanwardaActorRecipe::from_spec(spec).components().scaled_locomotion_capsule();
		apply_character_controller(&mut commands, actor_ref.0, hull);
		apply_character_mobility(&mut commands, actor_ref.0, 1.0, 1.0);
		bind_hurtbox_host(&mut commands, actor_ref.0, host);
		commands.entity(actor_ref.0).insert((
			HitVolumeCollider(hit_complex_collider(hull)),
			Npc,
			PlayerLook::default(),
			CollisionLayers::new(
				[PhysicsInteractionLayer::Fixed, PhysicsInteractionLayer::Animated],
				[PhysicsInteractionLayer::Fixed, PhysicsInteractionLayer::Animated],
			),
			ReplanMovement,
		));

		commands.entity(column_entity).insert(Tether);

		let objective = TetherObjective::Tether(column_entity, BOSS_TETHER_RADIUS);
		// Default added_radius/stuck_timeout let the motor walk ~2 m past the
		// leash before the brain writes a pull. Zero both so pursuit at the rim
		// cannot outrun the scaled objective.
		let mut tether = TetherIntelligenceUser::new(objective)
			.with_added_radius(0.0)
			.with_stuck_timeout(0.0)
			.with_clearance(hull.radius.max(0.4));
		tether.enabled = true;
		install_tether(&mut commands, actor_ref.0, tether);

		let mut movement = MovementIntelligence::new(MovementObjective::Reach(
			MovementLocation::new(host_global.translation(), hull.radius.max(0.4)),
		));
		movement.ability.agent_radius = hull.radius.max(0.4);
		movement.ability.feet_below_origin = hull.half_height();
		movement.ability.eye_height = (hull.length + 2.0 * hull.radius) * 0.8;
		movement.ability.path_segment = (hull.radius * 2.0).max(encounter_meters(4.0));
		movement.ability.max_step = hull.radius.max(0.4);
		movement.ability.max_fall = hull.half_height().max(encounter_meters(1.2));
		movement.ability.candidate_budget.max_candidates = 8;
		movement.ability.candidate_budget.horizon = encounter_meters(30.0);
		movement.ability.vantage_standoffs = VantageStandoffs::from_radii(&[
			encounter_meters(8.0),
			encounter_meters(12.0),
			encounter_meters(16.0),
		]);
		movement.ability.vantage_azimuths = 4;

		let knowledge = PoiKnowledge::default();

		let mut meandering = MeanderingIntelligenceUser::new(BOSS_TETHER_RADIUS);
		meandering.linger_secs = 1.5;
		meandering.selection_interval = 0.25;
		meandering.enabled = true;

		let mut firearm_movement = FirearmMovementIntelligence::new();
		firearm_movement.settings = FirearmMovementIntelligenceSettings {
			range: (encounter_meters(14.0), encounter_meters(1.0)),
			cover: 0.0,
			flee: (0.0, encounter_meters(8.0)),
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
			PatrolPinSnapState::new(),
			PoiVisitState::default(),
			FirearmIntelligence::new(),
			firearm_movement,
			FirearmTargeting::default(),
			FirearmEngagement::hold(),
		));

		// Spotted-facing and wish-facing both read [`PlayerYawOwner`] on the visual.
		for (visual, child_of) in &visuals {
			if child_of.parent() == actor_ref.0 {
				commands.entity(visual).insert(PlayerYawOwner::Wish);
			}
		}
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

/// Body rotation is locked; facing lives on the visual. Keep rear chain
/// sockets on that yaw so the tether anchor stays behind the turned boss.
fn align_boss_chain_anchors(
	visuals: Query<(&Transform, &ChildOf), With<CharacterRoot>>,
	hulls: Query<&LocomotionCapsule, With<BikanwardaRoot>>,
	mut anchors: Query<(&BikanwardaChainAnchor, &ChildOf, &mut Transform), Without<CharacterRoot>>,
) {
	let mut facing: HashMap<Entity, Quat> = HashMap::new();
	for (transform, child) in &visuals {
		facing.entry(child.parent()).or_insert(transform.rotation);
	}
	for (anchor, child, mut transform) in &mut anchors {
		let actor = child.parent();
		let Ok(hull) = hulls.get(actor) else {
			continue;
		};
		let Some(rest) = BikanwardaAnchors::for_capsule(*hull).get(anchor.name).copied() else {
			continue;
		};
		let yaw = facing.get(&actor).copied().unwrap_or(Quat::IDENTITY);
		transform.rotation = yaw * rest.rotation;
		transform.translation = yaw * rest.translation;
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
