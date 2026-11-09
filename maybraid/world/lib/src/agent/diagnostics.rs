//! Message-driven gameplay evidence, installed only by --agent-diagnostics.
//! No entity scans, visibility probes, tracing text, or AI mutations.

use avian3d::prelude::{GravityScale, LinearVelocity, Sleeping};
use bevy::prelude::*;
use damage::{DamageApplied, DamageSystems, Died, Health};
use firearms::WeaponFired;
use maybraid_agent::{AgentDiagnosticActor, AgentEvent, AgentEventBody, AgentPlayerVitals};
use maybraid_session::SessionLifecycleSystems;
use maybraid_spawns_support::PendingActorSupport;
use player::{CharacterController, Grounded, Player as MotorPlayer};
use world_player::{Player as VegetationPlayer, player::AwaitingTerrainSurface};

use crate::agent::{AgentRuntime, emit};

#[derive(Resource)]
pub(super) struct AgentDiagnostics {
	pub enabled: bool,
	/// Keep the selected identity through same-frame retirement.
	pub player: Option<Entity>,
}

pub(super) fn install(app: &mut App) {
	app.insert_resource(AgentDiagnostics { enabled: true, player: None })
		.add_systems(
			PostUpdate,
			observe_damage
				.after(DamageSystems::Down)
				.before(SessionLifecycleSystems::Notify),
		)
		.add_systems(Last, observe_shots);
}

pub(super) type PlayerVitalsQuery<'w, 's> = Query<
	'w,
	's,
	(
		Option<&'static Health>,
		Option<&'static LinearVelocity>,
		Has<CharacterController>,
		Has<Grounded>,
		Has<PendingActorSupport>,
		Has<AwaitingTerrainSurface>,
		Option<&'static GravityScale>,
		Has<Sleeping>,
	),
>;

pub(super) fn player_vitals(
	entity: Entity,
	query: &PlayerVitalsQuery,
) -> Option<AgentPlayerVitals> {
	let (health, velocity, motor, grounded, pending_support, awaiting_surface, gravity, sleeping) =
		query.get(entity).ok()?;
	Some(AgentPlayerVitals {
		entity: entity.to_bits().to_string(),
		health: health.map(|pool| pool.current),
		max_health: health.map(|pool| pool.max),
		velocity: velocity.map(|velocity| velocity.0.to_array()),
		grounded: motor.then_some(grounded),
		pending_support,
		awaiting_surface,
		gravity_scale: gravity.map(|gravity| gravity.0),
		sleeping,
	})
}

pub(super) fn player_identity(entity: Entity, position: Vec3) -> AgentDiagnosticActor {
	AgentDiagnosticActor {
		entity: entity.to_bits().to_string(),
		player: true,
		name: None,
		position: Some(position.to_array()),
	}
}

type ActorIdentities<'w, 's> = Query<
	'w,
	's,
	(Option<&'static Name>, Option<&'static Transform>, Has<MotorPlayer>, Has<VegetationPlayer>),
>;

fn identity(
	entity: Entity,
	known_player: Option<Entity>,
	actors: &ActorIdentities,
) -> AgentDiagnosticActor {
	let (name, transform, motor, vegetation) =
		actors.get(entity).ok().unwrap_or((None, None, false, false));
	AgentDiagnosticActor {
		entity: entity.to_bits().to_string(),
		player: motor || vegetation || known_player == Some(entity),
		name: name.map(|name| name.as_str().to_owned()),
		position: transform.map(|transform| transform.translation.to_array()),
	}
}

fn observe_damage(
	runtime: Res<AgentRuntime>,
	diagnostics: Res<AgentDiagnostics>,
	time: Res<Time>,
	mut damage: MessageReader<DamageApplied>,
	mut deaths: MessageReader<Died>,
	actors: ActorIdentities,
) {
	if !diagnostics.enabled {
		damage.clear();
		deaths.clear();
		return;
	}
	let t = time.elapsed_secs();
	let frame = runtime.frame;
	for event in damage.read() {
		emit(
			&runtime.sink,
			AgentEvent {
				t,
				frame,
				body: AgentEventBody::DamageApplied {
					target: identity(event.target, diagnostics.player, &actors),
					source: event
						.source
						.map(|entity| identity(entity, diagnostics.player, &actors)),
					amount: event.amount,
					applied: event.applied,
					remaining: event.remaining,
					point: event.point.to_array(),
				},
			},
		);
	}
	for event in deaths.read() {
		emit(
			&runtime.sink,
			AgentEvent {
				t,
				frame,
				body: AgentEventBody::Died {
					actor: identity(event.entity, diagnostics.player, &actors),
					source: event
						.source
						.map(|entity| identity(entity, diagnostics.player, &actors)),
					point: event.point.to_array(),
				},
			},
		);
	}
}

fn observe_shots(
	runtime: Res<AgentRuntime>,
	diagnostics: Res<AgentDiagnostics>,
	time: Res<Time>,
	mut shots: MessageReader<WeaponFired>,
	actors: ActorIdentities,
) {
	if !diagnostics.enabled {
		shots.clear();
		return;
	}
	for shot in shots.read() {
		emit(
			&runtime.sink,
			AgentEvent {
				t: time.elapsed_secs(),
				frame: runtime.frame,
				body: AgentEventBody::WeaponFired {
					shooter: identity(shot.shooter, diagnostics.player, &actors),
					recoil: shot.recoil,
					age: shot.age,
				},
			},
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::agent::tests::idle_runtime;
	use maybraid_agent::EventSink;

	#[test]
	fn records_damage_death_and_actual_shots_read_only_and_does_not_replay_paused_messages()
	-> anyhow::Result<()> {
		let directory = tempfile::tempdir()?;
		let path = directory.path().join("diagnostics.jsonl");
		let mut runtime = idle_runtime();
		runtime.sink = EventSink::open(&path).map_err(anyhow::Error::msg)?;
		let mut app = App::new();
		app.insert_resource(runtime)
			.init_resource::<Time>()
			.add_message::<DamageApplied>()
			.add_message::<Died>()
			.add_message::<WeaponFired>();
		install(&mut app);
		let player = app
			.world_mut()
			.spawn((
				MotorPlayer,
				Name::new("subject"),
				Transform::default(),
				Health::from_max(100.0),
			))
			.id();
		let source = app.world_mut().spawn(Name::new("attacker")).id();
		app.world_mut().resource_mut::<AgentDiagnostics>().player = Some(player);
		app.world_mut().write_message(DamageApplied {
			target: player,
			source: Some(source),
			amount: 250.0,
			applied: 100.0,
			remaining: 0.0,
			point: Vec3::Y,
		});
		app.world_mut().write_message(Died {
			entity: player,
			source: Some(source),
			point: Vec3::Y,
		});
		app.world_mut()
			.write_message(WeaponFired { shooter: source, recoil: 0.0, age: 0.1 });
		let mut cursor = app.world().resource::<Messages<DamageApplied>>().get_cursor();
		app.update();
		let lines = std::fs::read_to_string(&path)?;
		let events: Vec<_> = lines
			.lines()
			.map(AgentEvent::parse_line)
			.collect::<Result<_, _>>()
			.map_err(anyhow::Error::msg)?;
		assert_eq!(events.len(), 3);
		assert!(events.iter().any(|event| matches!(&event.body, AgentEventBody::DamageApplied { target, source: Some(attacker), remaining: 0.0, .. } if target.player && attacker.entity == source.to_bits().to_string())));
		assert!(events.iter().any(
			|event| matches!(&event.body, AgentEventBody::Died { actor, .. } if actor.player)
		));
		assert!(
			events
				.iter()
				.any(|event| matches!(event.body, AgentEventBody::WeaponFired { age: 0.1, .. }))
		);
		assert_eq!(cursor.read(app.world().resource::<Messages<DamageApplied>>()).count(), 1);
		assert_eq!(app.world().get::<Health>(player).map(|health| health.current), Some(100.0));
		app.world_mut().resource_mut::<AgentDiagnostics>().enabled = false;
		app.world_mut().write_message(WeaponFired::new(source, 0.0));
		app.update();
		app.world_mut().resource_mut::<AgentDiagnostics>().enabled = true;
		app.update();
		assert_eq!(std::fs::read_to_string(&path)?.lines().count(), 3);
		Ok(())
	}
}
