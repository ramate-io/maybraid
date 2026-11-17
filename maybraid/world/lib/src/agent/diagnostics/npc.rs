//! One-shot bounded NPC evidence. Reads cached state without probing or changing actors.

use avian3d::prelude::{GravityScale, LinearVelocity, ShapeHits};
use bevy::{ecs::system::SystemParam, prelude::*};
use combat_targeting::CombatTargeting;
use firearm_intelligence::FirearmTargeting;
use firearm_user::FirearmUser;
use firearms::{WeaponLaunchPose, WeaponTrigger};
use maybraid_agent::{
	AgentDiagnosticActor, AgentEvent, AgentEventBody, AgentNpcCombat, AgentNpcGroundHit,
	AgentNpcGroundHits, AgentNpcSelector, AgentNpcSnapshot, AgentNpcSpotContact, AgentNpcSupport,
	AgentNpcTrajectory, AgentNpcWeapon,
};
use maybraid_spawns_support::PendingActorSupport;
use mob_characters::CharacterSceneRecipe;
use player::{CharacterController, Grounded, Player as MotorPlayer, WalkableGround};
use spotting_intelligence::SpottingUser;
use world_player::Player as VegetationPlayer;

use super::{AgentDiagnostics, observe_shots};
use crate::agent::{AgentRuntime, emit};

const MAX_ACTORS: usize = 8;
const MAX_TRAJECTORIES: usize = 18;

type Npcs<'w, 's> = Query<
	'w,
	's,
	(Entity, Option<&'static Name>, Option<&'static Transform>),
	(With<CharacterSceneRecipe>, Without<MotorPlayer>, Without<VegetationPlayer>),
>;

#[derive(SystemParam)]
struct NpcSnapshotQueries<'w, 's> {
	npcs: Npcs<'w, 's>,
	positions: Query<'w, 's, &'static Transform>,
	physics: Query<
		'w,
		's,
		(
			Option<&'static LinearVelocity>,
			Option<&'static GravityScale>,
			Has<CharacterController>,
			Has<Grounded>,
			Option<&'static ShapeHits>,
			Option<&'static WalkableGround>,
			Option<&'static PendingActorSupport>,
		),
	>,
	combat: Query<
		'w,
		's,
		(
			Option<&'static CombatTargeting>,
			Option<&'static SpottingUser>,
			Option<&'static FirearmTargeting>,
			Option<&'static FirearmUser>,
		),
	>,
	weapons: Query<'w, 's, (Option<&'static WeaponTrigger>, Option<&'static WeaponLaunchPose>)>,
}

impl NpcSnapshotQueries<'_, '_> {
	fn capture(
		&self,
		selector: &AgentNpcSelector,
		player: Option<Entity>,
		now: f32,
	) -> Result<Vec<AgentNpcSnapshot>, String> {
		let selected = match selector {
			AgentNpcSelector::Nearest { radius } => {
				let player = player.ok_or("no selected player")?;
				let origin = self
					.positions
					.get(player)
					.map_err(|_| "selected player is no longer live")?
					.translation
					.xz();
				// Keep only eight candidates, including vertically displaced actors.
				let mut nearest: Vec<(f32, u64, Entity)> = Vec::with_capacity(MAX_ACTORS + 1);
				for (entity, _, transform) in &self.npcs {
					let Some(transform) = transform else { continue };
					let distance = transform.translation.xz().distance_squared(origin);
					if !distance.is_finite() || distance > radius * radius {
						continue;
					}
					nearest.push((distance, entity.to_bits(), entity));
					nearest.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
					nearest.truncate(MAX_ACTORS);
				}
				nearest.into_iter().map(|(_, _, entity)| entity).collect()
			}
			AgentNpcSelector::Entity { entity } => {
				let entity = entity
					.parse::<u64>()
					.ok()
					.and_then(Entity::try_from_bits)
					.ok_or("invalid entity bits")?;
				self.npcs
					.get(entity)
					.map_err(|_| "NPC is no longer live or is not a mob character")?;
				vec![entity]
			}
		};
		selected.into_iter().map(|entity| self.actor(entity, player, now)).collect()
	}

	fn actor(
		&self,
		entity: Entity,
		player: Option<Entity>,
		now: f32,
	) -> Result<AgentNpcSnapshot, String> {
		let (_, name, transform) =
			self.npcs.get(entity).map_err(|_| "NPC retired during snapshot")?;
		let (velocity, gravity, motor, grounded, hits, plane, pending) =
			self.physics.get(entity).map_err(|_| "NPC physics unavailable")?;
		let (targeting, spotting, trajectories, user) =
			self.combat.get(entity).map_err(|_| "NPC state unavailable")?;
		let engaged = targeting.and_then(|targeting| targeting.engaged);
		let mut targets = Vec::with_capacity(2);
		for target in [engaged, player].into_iter().flatten() {
			if !targets.contains(&target) {
				targets.push(target);
			}
		}
		let combat =
			(targeting.is_some() || spotting.is_some() || trajectories.is_some()).then(|| {
				AgentNpcCombat {
					enabled: targeting.map(|targeting| targeting.enabled),
					engaged: engaged.map(|target| target.to_bits().to_string()),
					engaged_contact_age: engaged
						.and_then(|target| targeting?.contact(target))
						.map(|contact| now - contact.last_spotted_at),
					player_contact_age: player
						.and_then(|target| targeting?.contact(target))
						.map(|contact| now - contact.last_spotted_at),
					spotting: targets
						.iter()
						.filter_map(|target| spotting?.contacts.get(target))
						.map(|contact| AgentNpcSpotContact {
							subject: contact.subject.to_bits().to_string(),
							success_age: now - contact.last_success_at,
							attempt_age: now - contact.last_attempt_at,
							next_respot_in: contact.next_respot_at - now,
							failures: contact.consecutive_failures,
						})
						.collect(),
					trajectories: trajectories
						.into_iter()
						.flat_map(|cache| &cache.trajectories)
						.filter(|trajectory| targets.contains(&trajectory.target))
						.take(MAX_TRAJECTORIES)
						.map(|trajectory| AgentNpcTrajectory {
							target: trajectory.target.to_bits().to_string(),
							clear: trajectory.clear,
							checked_age: now - trajectory.checked_at,
						})
						.collect(),
				}
			});
		Ok(AgentNpcSnapshot {
			actor: AgentDiagnosticActor {
				entity: entity.to_bits().to_string(),
				player: false,
				name: name.map(|name| name.as_str().to_owned()),
				position: transform.map(|transform| transform.translation.to_array()),
			},
			velocity: velocity.map(|velocity| velocity.0.to_array()),
			gravity_scale: gravity.map(|gravity| gravity.0),
			grounded: motor.then_some(grounded),
			shape_hits: hits.map(|hits| AgentNpcGroundHits {
				count: hits.len(),
				nearest: hits.iter().min_by(|a, b| a.distance.total_cmp(&b.distance)).map(|hit| {
					AgentNpcGroundHit {
						entity: hit.entity.to_bits().to_string(),
						distance: hit.distance,
						normal: (-hit.normal2).to_array(),
					}
				}),
			}),
			cached_ground_normal: plane.map(|plane| plane.normal.to_array()),
			pending_support: pending.map(|support| AgentNpcSupport {
				failure: support.failure.map(|failure| format!("{failure:?}")),
				held: support.held_position().to_array(),
				search_xz: support.search_xz().to_array(),
				search_surface_y: support
					.search()
					.map(|search| [search.min_surface_y, search.max_surface_y]),
				exhausted: support.exhausted(),
			}),
			combat,
			weapon: user.map(|user| {
				let state = self.weapons.get(user.held).ok();
				AgentNpcWeapon {
					held: user.held.to_bits().to_string(),
					trigger: state.and_then(|(trigger, _)| trigger).map(|trigger| trigger.0),
					launch_ready: state.and_then(|(_, pose)| pose).map(|pose| pose.ready),
				}
			}),
		})
	}
}

pub(super) fn install(app: &mut App) {
	app.add_systems(Last, snapshot_npcs.after(observe_shots));
}

fn snapshot_npcs(
	runtime: Res<AgentRuntime>,
	mut diagnostics: ResMut<AgentDiagnostics>,
	time: Res<Time>,
	queries: NpcSnapshotQueries,
) {
	let Some((request_id, selector)) = diagnostics.pending_npc.take() else { return };
	if !diagnostics.enabled {
		return;
	}
	let player = diagnostics.player;
	let result = queries.capture(&selector, player, time.elapsed_secs());
	let (actors, error) = match result {
		Ok(actors) => (actors, None),
		Err(error) => (Vec::new(), Some(error)),
	};
	emit(
		&runtime.sink,
		AgentEvent {
			t: time.elapsed_secs(),
			frame: runtime.frame,
			body: AgentEventBody::NpcSnapshot {
				request_id,
				selector,
				player: player.map(|player| player.to_bits().to_string()),
				actors,
				error,
			},
		},
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::agent::tests::idle_runtime;
	use bevy::ecs::system::RunSystemOnce;
	use maybraid_agent::EventSink;

	fn capture(
		world: &mut World,
		selector: AgentNpcSelector,
		player: Option<Entity>,
	) -> anyhow::Result<Vec<AgentNpcSnapshot>> {
		world
			.run_system_once(move |queries: NpcSnapshotQueries| {
				queries.capture(&selector, player, 4.0)
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.map_err(anyhow::Error::msg)
	}

	#[test]
	fn nearest_is_capped_stably_and_uses_xz_for_floating_actors() -> anyhow::Result<()> {
		let mut world = World::new();
		let player = world
			.spawn((MotorPlayer, CharacterSceneRecipe::default(), Transform::default()))
			.id();
		let floating = world
			.spawn((CharacterSceneRecipe::default(), Transform::from_xyz(1.0, 500.0, 0.0)))
			.id();
		let mut ties = Vec::new();
		for _ in 0..10 {
			ties.push(
				world
					.spawn((CharacterSceneRecipe::default(), Transform::from_xyz(2.0, 0.0, 0.0)))
					.id(),
			);
		}
		let far = world
			.spawn((CharacterSceneRecipe::default(), Transform::from_xyz(31.0, 0.0, 0.0)))
			.id();
		let snapshots =
			capture(&mut world, AgentNpcSelector::Nearest { radius: 30.0 }, Some(player))?;
		assert_eq!(snapshots.len(), 8);
		assert_eq!(snapshots[0].actor.entity, floating.to_bits().to_string());
		ties.sort_by_key(|entity| entity.to_bits());
		let expected: Vec<_> =
			ties[..7].iter().map(|entity| entity.to_bits().to_string()).collect();
		assert_eq!(
			snapshots[1..]
				.iter()
				.map(|actor| actor.actor.entity.clone())
				.collect::<Vec<_>>(),
			expected
		);
		assert!(snapshots.iter().all(|actor| actor.actor.entity != far.to_bits().to_string()
			&& actor.actor.entity != player.to_bits().to_string()));
		// Direct selection needs neither a live player nor a radius/position scan.
		let direct = capture(
			&mut world,
			AgentNpcSelector::Entity { entity: far.to_bits().to_string() },
			None,
		)?;
		assert_eq!(direct.len(), 1);
		assert_eq!(direct[0].actor.entity, far.to_bits().to_string());
		world.entity_mut(far).despawn();
		assert!(
			capture(
				&mut world,
				AgentNpcSelector::Entity { entity: far.to_bits().to_string() },
				None
			)
			.is_err()
		);
		Ok(())
	}

	#[test]
	fn cached_support_ground_and_combat_evidence_is_read_only() -> anyhow::Result<()> {
		use combat_targeting::CombatContact;
		use spotting_intelligence::{SpotFeature, SpottedContact};
		let mut world = World::new();
		let player = world.spawn_empty().id();
		let weapon = world.spawn((WeaponTrigger(false), WeaponLaunchPose::pending())).id();
		let position = Vec3::new(5.0, 80.0, 0.0);
		let mut support =
			PendingActorSupport::new(position, characters::LocomotionCapsule::HUMANOID, None);
		support.failure = Some(maybraid_spawns_support::SupportProbeFailure::Clearance);
		let mut targeting = CombatTargeting::default();
		targeting.upsert_contact(CombatContact {
			subject: player,
			position: Vec3::ZERO,
			movement_vector: Vec3::ZERO,
			visible_point: Vec3::ZERO,
			visible_head: None,
			last_spotted_at: 3.5,
		});
		targeting.engage(player);
		let mut spotting = SpottingUser::default();
		spotting.contacts.insert(
			player,
			SpottedContact::new(player, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, None, 3.0, 0.125),
		);
		let mut trajectories = FirearmTargeting::default();
		trajectories.trajectories.push(firearm_intelligence::AimTrajectory {
			target: player,
			feature: SpotFeature::CenterMass,
			aim_point: Vec3::ZERO,
			center: Vec3::ZERO,
			distance: 5.0,
			clear: false,
			checked_at: 3.75,
		});
		let actor = world
			.spawn((
				CharacterSceneRecipe::default(),
				Transform::from_translation(position),
				LinearVelocity(Vec3::ZERO),
				GravityScale(0.0),
				CharacterController,
				Grounded,
				ShapeHits::default(),
				WalkableGround { normal: Vec3::Y },
				support,
				targeting,
				spotting.clone(),
				trajectories,
				FirearmUser::holding(weapon),
			))
			.id();
		let entities_before = world.entities().len();
		let snapshots = capture(
			&mut world,
			AgentNpcSelector::Entity { entity: actor.to_bits().to_string() },
			Some(player),
		)?;
		let snapshot = snapshots.first().ok_or_else(|| anyhow::anyhow!("missing snapshot"))?;
		assert_eq!(snapshot.grounded, Some(true));
		assert_eq!(snapshot.shape_hits.as_ref().map(|hits| hits.count), Some(0));
		assert_eq!(snapshot.gravity_scale, Some(0.0));
		assert_eq!(
			snapshot.pending_support.as_ref().map(|support| support.held),
			Some(position.to_array())
		);
		let combat = snapshot
			.combat
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("missing combat evidence"))?;
		assert_eq!(combat.player_contact_age, Some(0.5));
		assert_eq!(combat.engaged_contact_age, Some(0.5));
		assert_eq!(combat.spotting.len(), 1);
		assert_eq!(combat.spotting[0].success_age, 1.0);
		assert_eq!(combat.trajectories[0].checked_age, 0.25);
		assert!(!combat.trajectories[0].clear);
		assert_eq!(snapshot.weapon.as_ref().and_then(|weapon| weapon.trigger), Some(false));
		assert_eq!(world.get::<Transform>(actor).map(|pose| pose.translation), Some(position));
		assert_eq!(world.get::<GravityScale>(actor).map(|gravity| gravity.0), Some(0.0));
		assert_eq!(world.get::<SpottingUser>(actor), Some(&spotting));
		assert_eq!(world.get::<WeaponTrigger>(weapon).map(|trigger| trigger.0), Some(false));
		assert_eq!(world.entities().len(), entities_before);
		Ok(())
	}

	#[test]
	fn nearest_cached_shape_hit_is_reported_without_running_physics() -> anyhow::Result<()> {
		use avian3d::prelude::ShapeHitData;
		let mut world = World::new();
		let far = world.spawn_empty().id();
		let near = world.spawn_empty().id();
		let hits = ShapeHits(vec![
			ShapeHitData {
				entity: far,
				distance: 0.4,
				point1: Vec3::ZERO,
				point2: Vec3::ZERO,
				normal1: Vec3::X,
				normal2: -Vec3::X,
			},
			ShapeHitData {
				entity: near,
				distance: 0.1,
				point1: Vec3::ZERO,
				point2: Vec3::ZERO,
				normal1: Vec3::Y,
				normal2: -Vec3::Y,
			},
		]);
		let actor = world.spawn((CharacterSceneRecipe::default(), hits.clone())).id();
		let snapshots = capture(
			&mut world,
			AgentNpcSelector::Entity { entity: actor.to_bits().to_string() },
			None,
		)?;
		let hit = snapshots
			.first()
			.and_then(|actor| actor.shape_hits.as_ref())
			.and_then(|hits| hits.nearest.as_ref())
			.ok_or_else(|| anyhow::anyhow!("missing nearest cached hit"))?;
		assert_eq!(hit.entity, near.to_bits().to_string());
		assert_eq!(hit.distance, 0.1);
		assert_eq!(hit.normal, Vec3::Y.to_array());
		assert_eq!(world.get::<ShapeHits>(actor), Some(&hits));
		Ok(())
	}

	#[test]
	fn snapshots_require_enabled_diagnostics_and_are_not_replayed() -> anyhow::Result<()> {
		let directory = tempfile::tempdir()?;
		let path = directory.path().join("snapshots.jsonl");
		let mut world = World::new();
		assert!(!world.contains_resource::<AgentDiagnostics>());
		let mut runtime = idle_runtime();
		runtime.sink = EventSink::open(&path).map_err(anyhow::Error::msg)?;
		world.insert_resource(runtime);
		world.init_resource::<Time>();
		let actor = world.spawn(CharacterSceneRecipe::default()).id();
		let selector = AgentNpcSelector::Entity { entity: actor.to_bits().to_string() };
		world.insert_resource(AgentDiagnostics { enabled: false, player: None, pending_npc: None });
		assert!(
			world
				.resource_mut::<AgentDiagnostics>()
				.request_npc_snapshot(1, selector.clone())
				.is_err()
		);
		world.resource_mut::<AgentDiagnostics>().enabled = true;
		world
			.resource_mut::<AgentDiagnostics>()
			.request_npc_snapshot(2, selector.clone())
			.map_err(anyhow::Error::msg)?;
		assert!(
			world
				.resource_mut::<AgentDiagnostics>()
				.request_npc_snapshot(3, selector.clone())
				.is_err()
		);
		world
			.run_system_once(snapshot_npcs)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(snapshot_npcs)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let lines = std::fs::read_to_string(&path)?;
		assert_eq!(lines.lines().count(), 1);
		let event = AgentEvent::parse_line(lines.trim()).map_err(anyhow::Error::msg)?;
		let AgentEventBody::NpcSnapshot { request_id, actors, error, .. } = event.body else {
			return Err(anyhow::anyhow!("wrong event"));
		};
		assert_eq!(request_id, 2);
		assert!(error.is_none());
		let actor = actors.first().ok_or_else(|| anyhow::anyhow!("missing actor"))?;
		assert!(
			actor.grounded.is_none() && actor.pending_support.is_none() && actor.weapon.is_none()
		);
		world
			.resource_mut::<AgentDiagnostics>()
			.request_npc_snapshot(4, selector)
			.map_err(anyhow::Error::msg)?;
		world.resource_mut::<AgentDiagnostics>().enabled = false;
		world
			.run_system_once(snapshot_npcs)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.resource_mut::<AgentDiagnostics>().enabled = true;
		world
			.run_system_once(snapshot_npcs)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(std::fs::read_to_string(&path)?.lines().count(), 1);
		Ok(())
	}
}
