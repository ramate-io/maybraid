//! Spawn High roster plants off the host transform tree; cull them with the stub.

use std::collections::HashMap;

use bevy::prelude::*;
use lod::{LodChunkCullSystems, LodSceneLevel};
use mob_characters::CharacterSceneRecipe;
use mob_intelligence::{ancestor_mob, MemberOf, Mob, MobId, MobSlot, RosterBinding, RosterRef};
use poi_intelligence::{NearbyPlace, PoiSupport, AGENT_SEPARATION};

use crate::plugin::MobSceneSystems;
use crate::MobScene;

pub type MemberRosterRef = RosterRef<CharacterSceneRecipe>;

#[derive(Resource, Default)]
struct RosterStubBodies(HashMap<Entity, RosterBinding>);

pub(crate) fn configure_roster_ref_systems(app: &mut App) {
	app.init_resource::<RosterStubBodies>()
		.add_systems(
			Update,
			(fulfill_roster_refs, record_roster_bindings)
				.chain()
				.in_set(MobSceneSystems::Fulfill),
		)
		.add_systems(
			Update,
			(despawn_roster_bodies_for_removed_stubs, cull_members_when_high_drops)
				.chain()
				.after(LodChunkCullSystems::Drain),
		);
}

fn fulfill_roster_refs(
	mut commands: Commands,
	stubs: Query<(Entity, &MemberRosterRef), Without<RosterBinding>>,
	child_of: Query<&ChildOf>,
	mobs: Query<(), With<Mob>>,
	hosts: Query<
		(Entity, &MobId, &Transform, Option<&GlobalTransform>, Option<&MobScene>),
		With<Mob>,
	>,
	live: Query<(&MemberOf, &Transform)>,
) {
	let mut placed: Vec<(Entity, Vec3)> = Vec::new();
	for (stub, roster) in &stubs {
		let Some(host) = ancestor_mob(stub, &child_of, &mobs) else {
			continue;
		};
		let Ok((host, id, transform, global, scene)) = hosts.get(host) else {
			continue;
		};
		let preferred = global
			.map(|global| global.transform_point(roster.offset))
			.unwrap_or_else(|| transform.transform_point(roster.offset));
		let occupied: Vec<Vec3> = live
			.iter()
			.filter(|(member, _)| member.mob == host)
			.map(|(_, transform)| transform.translation)
			.chain(placed.iter().filter(|(placed_host, _)| *placed_host == host).map(|(_, at)| *at))
			.collect();
		let cleared = NearbyPlace { position: preferred, poi: None, support: PoiSupport::Terrain }
			.clear_of(
				transform.translation,
				&occupied,
				AGENT_SEPARATION,
				u64::from(roster.slot).wrapping_add(1),
			)
			.position;
		let at = Vec3::new(cleared.x, preferred.y, cleared.z);
		let body = roster.recipe.spawn(&mut commands, Transform::from_translation(at));
		commands.entity(body).insert((MobSlot(roster.slot), *id));
		if let Some(kind) = scene.map(|scene| scene.mob.kind) {
			commands.entity(body).insert(kind);
		}
		commands.entity(stub).insert(RosterBinding { body, host, slot: roster.slot });
		placed.push((host, at));
	}
}

fn record_roster_bindings(
	added: Query<(Entity, &RosterBinding), Added<RosterBinding>>,
	mut live: ResMut<RosterStubBodies>,
) {
	for (stub, binding) in &added {
		live.0.insert(stub, *binding);
	}
}

fn despawn_roster_bodies_for_removed_stubs(
	mut removed: RemovedComponents<RosterBinding>,
	mut live: ResMut<RosterStubBodies>,
	members: Query<&MemberOf>,
	mut commands: Commands,
) {
	for stub in removed.read() {
		let Some(binding) = live.0.remove(&stub) else {
			continue;
		};
		let Ok(member) = members.get(binding.body) else {
			continue;
		};
		if member.mob == binding.host && member.slot == binding.slot {
			commands.entity(binding.body).try_despawn();
		}
	}
}

fn cull_members_when_high_drops(
	hosts: Query<&LodSceneLevel, With<Mob>>,
	members: Query<(Entity, &MemberOf)>,
	mut commands: Commands,
) {
	// Preserve the all-High path without looking up every member's host.
	if !hosts.iter().any(|level| *level != LodSceneLevel::High) {
		return;
	}
	for (entity, membership) in &members {
		if hosts.get(membership.mob).is_ok_and(|level| *level != LodSceneLevel::High) {
			commands.entity(entity).try_despawn();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use std::sync::Arc;

	use crate::MobKind;

	fn host_and_stub(world: &mut World, offset: Vec3) -> (Entity, Entity) {
		let recipe = Arc::new(CharacterSceneRecipe::default());
		let mut commands = world.commands();
		let host = commands
			.spawn((
				Transform::from_xyz(10.0, 2.0, 4.0),
				GlobalTransform::from_translation(Vec3::new(10.0, 2.0, 4.0)),
				Mob::new(24.0),
				MobId(9),
			))
			.id();
		let stub = commands
			.spawn((MemberRosterRef::new(Arc::clone(&recipe), 0, offset), ChildOf(host)))
			.id();
		world.flush();
		(host, stub)
	}

	#[test]
	fn fulfill_spawns_a_world_body_off_the_host_tree() {
		let mut world = World::new();
		world.init_resource::<RosterStubBodies>();
		let offset = Vec3::new(5.0, 1.0, 0.0);
		let (host, stub) = host_and_stub(&mut world, offset);
		world.run_system_once(fulfill_roster_refs).expect("fulfill");
		world.flush();
		let binding = world.get::<RosterBinding>(stub).expect("binding");
		assert_eq!(binding.host, host);
		assert!(world.get::<ChildOf>(binding.body).is_none());
		let at = world.get::<Transform>(binding.body).expect("pose").translation;
		assert!((at - Vec3::new(15.0, 3.0, 4.0)).length() < 1e-3);
		assert_eq!(world.get::<MobSlot>(binding.body), Some(&MobSlot(0)));
		assert_eq!(world.get::<MobId>(binding.body), Some(&MobId(9)));
		assert!(world.get::<CharacterSceneRecipe>(stub).is_none());
	}

	#[test]
	fn host_translation_does_not_tow_the_body() {
		let mut world = World::new();
		world.init_resource::<RosterStubBodies>();
		let (host, stub) = host_and_stub(&mut world, Vec3::new(5.0, 0.0, 0.0));
		world.run_system_once(fulfill_roster_refs).expect("fulfill");
		world.flush();
		let body = world.get::<RosterBinding>(stub).expect("binding").body;
		let before = world.get::<Transform>(body).expect("pose").translation;
		world.entity_mut(host).insert(Transform::from_xyz(100.0, 2.0, 4.0));
		world.flush();
		let after = world.get::<Transform>(body).expect("pose").translation;
		assert!((after - before).length() < 1e-4);
	}

	#[test]
	fn removing_the_stub_despawns_a_still_bound_body() {
		let mut world = World::new();
		world.init_resource::<RosterStubBodies>();
		let (host, stub) = host_and_stub(&mut world, Vec3::X);
		world.run_system_once(fulfill_roster_refs).expect("fulfill");
		world.flush();
		let body = world.get::<RosterBinding>(stub).expect("binding").body;
		world.entity_mut(body).insert(MemberOf { mob: host, slot: 0 });
		world.run_system_once(record_roster_bindings).expect("record");
		world.flush();
		world.entity_mut(stub).despawn();
		world.flush();
		world.run_system_once(despawn_roster_bodies_for_removed_stubs).expect("cull");
		world.flush();
		assert!(world.get_entity(body).is_err());
	}

	#[test]
	fn removing_the_stub_does_not_despawn_a_recycled_body() {
		let mut world = World::new();
		world.init_resource::<RosterStubBodies>();
		let (_host, stub) = host_and_stub(&mut world, Vec3::X);
		world.run_system_once(fulfill_roster_refs).expect("fulfill");
		world.flush();
		let body = world.get::<RosterBinding>(stub).expect("binding").body;
		world.run_system_once(record_roster_bindings).expect("record");
		world.flush();
		world.entity_mut(stub).despawn();
		world.flush();
		world.run_system_once(despawn_roster_bodies_for_removed_stubs).expect("cull");
		world.flush();
		assert!(world.get_entity(body).is_ok());
		assert_eq!(world.get::<MobId>(body), Some(&MobId(9)));
	}

	#[test]
	fn fulfill_stamps_the_host_mob_kind_on_the_body() {
		let mut world = World::new();
		world.init_resource::<RosterStubBodies>();
		let recipe = Arc::new(CharacterSceneRecipe::default());
		let mut commands = world.commands();
		let host = commands
			.spawn((
				Transform::from_xyz(10.0, 2.0, 4.0),
				GlobalTransform::from_translation(Vec3::new(10.0, 2.0, 4.0)),
				Mob::new(24.0),
				MobId(9),
				MobScene::of_kind(MobKind::Brawler, 1.0),
			))
			.id();
		let stub = commands
			.spawn((MemberRosterRef::new(Arc::clone(&recipe), 0, Vec3::X), ChildOf(host)))
			.id();
		world.flush();
		world.run_system_once(fulfill_roster_refs).expect("fulfill");
		world.flush();
		let body = world.get::<RosterBinding>(stub).expect("binding").body;
		assert_eq!(world.get::<MobKind>(body), Some(&MobKind::Brawler));
	}

	#[test]
	fn leaving_high_despawns_unparented_members() {
		let mut world = World::new();
		let host = world.spawn((Mob::new(12.0), LodSceneLevel::High)).id();
		let body = world.spawn(MemberOf { mob: host, slot: 0 }).id();
		world.entity_mut(host).insert(LodSceneLevel::UltraLow);
		world.flush();
		world.run_system_once(cull_members_when_high_drops).expect("cull");
		world.flush();
		assert!(world.get_entity(body).is_err());
	}
	#[test]
	fn hysteresis_keeps_the_same_damaged_body_until_the_outer_boundary() -> anyhow::Result<()> {
		use damage::Health;
		use lod::lod_ref::LodRef;
		use lod::scene::LodScene;
		use mob_intelligence::{MobRoster, RosterMember};
		use npc_intelligence::Personality;

		let mut world = World::new();
		world.init_resource::<RosterStubBodies>();
		let (host, stub) = host_and_stub(&mut world, Vec3::X);
		world.entity_mut(host).insert((MobScene::default(), LodSceneLevel::High));
		world
			.run_system_once(fulfill_roster_refs)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		let body = world
			.get::<RosterBinding>(stub)
			.ok_or_else(|| anyhow::anyhow!("missing binding"))?
			.body;
		let health = Health { current: 37.0, max: 100.0 };
		world.entity_mut(body).insert((MemberOf { mob: host, slot: 0 }, health));
		let mut member = RosterMember::new(Personality::Predator, Vec3::ZERO);
		member.entity = Some(body);
		world.entity_mut(host).insert(MobRoster::new(vec![member]));
		for distance in [299.0, 301.0, 399.0, 300.0, 401.0] {
			world
				.run_system_once(crate::plugin::snapshot_retained_mob_members)
				.map_err(|error| anyhow::anyhow!("{error:?}"))?;
			let viewer = Transform::from_xyz(distance, 0.0, 0.0);
			let bounds = bevy::shape::Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
			let reference = LodRef {
				entity: Entity::PLACEHOLDER,
				previous_transform: &viewer,
				current_transform: &viewer,
				bounds: &bounds,
			};
			let desired = world
				.get::<MobScene>(host)
				.ok_or_else(|| anyhow::anyhow!("missing scene"))?
				.scene_lod_level(&reference);
			world.entity_mut(host).insert(desired);
			world
				.run_system_once(cull_members_when_high_drops)
				.map_err(|error| anyhow::anyhow!("{error:?}"))?;
			world.flush();
			if distance <= 400.0 {
				assert_eq!(world.get::<Health>(body), Some(&health));
				assert_eq!(
					world.get::<RosterBinding>(stub).map(|binding| binding.body),
					Some(body)
				);
			} else {
				assert!(world.get_entity(body).is_err());
			}
		}
		Ok(())
	}

	#[test]
	fn high_drop_cull_preserves_bodies_without_an_eligible_low_host() -> anyhow::Result<()> {
		let mut world = World::new();
		let low = world.spawn((Mob::new(12.0), LodSceneLevel::UltraLow)).id();
		let high = world.spawn((Mob::new(12.0), LodSceneLevel::High)).id();
		let unrelated = world.spawn(LodSceneLevel::UltraLow).id();
		let unclassified = world.spawn(Mob::new(12.0)).id();
		let dropped = world.spawn(MemberOf { mob: low, slot: 0 }).id();
		let kept: Vec<_> = [high, unrelated, unclassified, Entity::PLACEHOLDER]
			.into_iter()
			.map(|mob| world.spawn(MemberOf { mob, slot: 0 }).id())
			.collect();
		world
			.run_system_once(cull_members_when_high_drops)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		assert!(world.get_entity(dropped).is_err());
		for entity in kept {
			assert!(world.get_entity(entity).is_ok());
		}
		Ok(())
	}
}
