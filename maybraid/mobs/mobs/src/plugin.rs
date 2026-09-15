//! Materialize mob hosts, death replacements, and player-prey targeting.

use bevy::prelude::*;
use damage::Health;
use journeying_intelligence::JourneyingIntelligencePlugin;
use lod::{
	add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for, LodChunkFulfillSystems,
	LodSceneLevel,
};
use mob_characters::{CharacterSceneSystems, MobCharacterScenesPlugin};
use mob_intelligence::{
	install_mob, install_prey_targeting, MobIdAlloc, MobInstall, MobIntelligencePlugin,
	MobMemberNeeded, MobSlot, MobSystems, RosterMember,
};
use poi_intelligence::PoiIntelligencePlugin;
use routing_intelligence::RoutingPlugin;
use tether_intelligence::TetherPlugin;

use crate::MobScene;

const RESPAWN_RETRY_SECS: f32 = 1.0;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MobSceneSystems {
	Install,
	Fulfill,
	Respawn,
	Surface,
	Center,
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MobLodRefreshMode {
	#[default]
	FullScan,
	Indexed,
}

pub struct MobScenesPlugin;

impl Plugin for MobScenesPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MobCharacterScenesPlugin>() {
			app.add_plugins(MobCharacterScenesPlugin);
		}
		if !app.is_plugin_added::<MobIntelligencePlugin>() {
			app.add_plugins(MobIntelligencePlugin);
		}
		if !app.is_plugin_added::<damage::DamagePlugin>() {
			app.add_plugins(damage::DamagePlugin);
		}
		if !app.is_plugin_added::<PoiIntelligencePlugin>() {
			app.add_plugins(PoiIntelligencePlugin);
		}
		if !app.is_plugin_added::<JourneyingIntelligencePlugin>() {
			app.add_plugins(JourneyingIntelligencePlugin);
		}
		if !app.is_plugin_added::<TetherPlugin>() {
			app.add_plugins(TetherPlugin);
		}
		if !app.is_plugin_added::<RoutingPlugin>() {
			app.add_plugins(RoutingPlugin);
		}
		match app.world().get_resource::<MobLodRefreshMode>().copied().unwrap_or_default() {
			MobLodRefreshMode::FullScan => add_lod_refresh_chunk_full_for::<MobScene>(app),
			MobLodRefreshMode::Indexed => add_lod_refresh_chunk_for::<MobScene>(app),
		}
		app.configure_sets(
			Update,
			(MobSceneSystems::Install, MobSceneSystems::Fulfill, MobSceneSystems::Respawn).chain(),
		)
		.configure_sets(
			Update,
			MobSceneSystems::Fulfill
				.after(LodChunkFulfillSystems::Drain)
				.before(CharacterSceneSystems::Materialize)
				.before(MobSystems::Bind),
		)
		.configure_sets(
			Update,
			(MobSceneSystems::Surface, MobSceneSystems::Center)
				.chain()
				.after(MobSystems::Travel),
		)
		.add_systems(
			Update,
			install_mob_scenes
				.in_set(MobSceneSystems::Install)
				.before(CharacterSceneSystems::Materialize)
				.before(MobSystems::Bind),
		)
		.add_systems(
			Update,
			spawn_needed_members.in_set(MobSceneSystems::Respawn).after(MobSystems::Respawn),
		)
		.add_systems(Update, sync_mob_scene_centers.in_set(MobSceneSystems::Center));
		crate::roster_ref::configure_roster_ref_systems(app);
	}
}

fn install_mob_scenes(
	mut commands: Commands,
	mut ids: ResMut<MobIdAlloc>,
	hosts: Query<(Entity, &MobScene, &Transform), Added<MobScene>>,
) {
	for (host, scene, transform) in &hosts {
		let id = ids.allocate();
		let members = scene
			.mob
			.roster
			.members
			.iter()
			.enumerate()
			.map(|(slot, member)| {
				let recipe = &member.character;
				let mut roster = RosterMember::new(
					recipe.brains.personality(recipe.armed()),
					transform.translation + member.offset,
				)
				.with_armed(recipe.armed())
				.with_keep_tether_in_combat(Some(recipe.brains.keep_tether_in_combat()))
				.with_interests(recipe.brains.interests_for_slot(slot));
				roster.health = Health::from_max(f32::from(recipe.sheet().health));
				roster
			})
			.collect();
		let brain = &scene.mob.intelligence;
		let mut install = MobInstall::new(id, brain.leash, members)
			.with_interests(brain.interests.clone())
			.with_affiliations(brain.affiliations.clone())
			.with_respawn(brain.respawn)
			.with_journey(brain.journey);
		if let Some(travel) = brain.travel {
			install = install.with_travel(travel);
		}
		install_mob(&mut commands, host, install);
		commands.entity(host).insert(brain.clone());
		if let Some(targeting) = brain.prey_targeting() {
			install_prey_targeting(&mut commands, host, targeting, transform.translation);
		}
	}
}

fn spawn_needed_members(
	time: Res<Time>,
	mut commands: Commands,
	mut needed: MessageReader<MobMemberNeeded>,
	mut mobs: Query<(&MobScene, Option<&LodSceneLevel>, &mut mob_intelligence::MobRoster)>,
) {
	let retry_at = time.elapsed_secs() + RESPAWN_RETRY_SECS;
	for request in needed.read() {
		let Ok((mob, level, mut roster)) = mobs.get_mut(request.mob) else {
			continue;
		};
		if level.is_some_and(|level| *level != LodSceneLevel::High) {
			if let Some(member) = roster.get_mut(request.slot) {
				member.spawn_requested = false;
				member.respawn_at = Some(retry_at);
			}
			continue;
		}
		let Some(member) = mob.mob.roster.members.get(request.slot as usize) else {
			continue;
		};
		let body = member.character.spawn(&mut commands, Transform::from_translation(request.pose));
		commands.entity(body).insert((MobSlot(request.slot), request.id));
	}
}

fn sync_mob_scene_centers(mut hosts: Query<(&Transform, &mut MobScene), Changed<Transform>>) {
	for (transform, mut scene) in &mut hosts {
		scene.center = transform.translation;
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use bevy::ecs::system::RunSystemOnce;
	use npc_intelligence::Personality;

	use super::*;
	use crate::MobKind;
	use mob_intelligence::{
		MobId, MobMemberNeeded, MobRoster, PreyTargetMemory, PreyTargetingIntelligence,
	};

	#[test]
	fn plugin_update_schedule_is_acyclic() -> Result<()> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
			.insert_resource(MobLodRefreshMode::Indexed)
			.add_plugins(MobScenesPlugin);
		app.world_mut()
			.schedule_scope(Update, |world, schedule| schedule.initialize(world))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn rejected_low_lod_respawn_is_retryable() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<Messages<MobMemberNeeded>>();
		let host = world
			.spawn((
				MobScene::of_kind(MobKind::Herd, 0.2),
				Transform::default(),
				LodSceneLevel::UltraLow,
				MobRoster::new(vec![RosterMember::new(Personality::Grazer, Vec3::Y)]),
			))
			.id();
		{
			let mut roster = world
				.get_mut::<MobRoster>(host)
				.ok_or_else(|| anyhow::anyhow!("missing roster"))?;
			let member =
				roster.get_mut(0).ok_or_else(|| anyhow::anyhow!("missing roster member"))?;
			member.spawn_requested = true;
		}
		world.resource_mut::<Messages<MobMemberNeeded>>().write(MobMemberNeeded {
			mob: host,
			id: MobId(7),
			slot: 0,
			pose: Vec3::Y,
		});

		world
			.run_system_once(spawn_needed_members)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let member = world
			.get::<MobRoster>(host)
			.and_then(|roster| roster.get(0))
			.ok_or_else(|| anyhow::anyhow!("missing roster member after retry"))?;
		assert!(!member.spawn_requested);
		assert_eq!(member.respawn_at, Some(RESPAWN_RETRY_SECS));
		Ok(())
	}

	#[test]
	fn moving_host_updates_the_mob_lod_center() -> Result<()> {
		let mut world = World::new();
		let at = Vec3::new(40.0, 12.0, -9.0);
		let host = world
			.spawn((MobScene::of_kind(MobKind::Pack, 0.4), Transform::from_translation(at)))
			.id();

		world
			.run_system_once(sync_mob_scene_centers)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(world.get::<MobScene>(host).map(|scene| scene.center), Some(at));
		Ok(())
	}

	#[test]
	fn pack_install_pins_player_prey_at_spawn() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<MobIdAlloc>();
		let home = Vec3::new(12.0, 0.0, -4.0);
		world.spawn((MobScene::of_kind(MobKind::Pack, 0.4), Transform::from_translation(home)));

		world
			.run_system_once(install_mob_scenes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let host = world
			.query::<(Entity, &PreyTargetingIntelligence, &PreyTargetMemory)>()
			.iter(&world)
			.next()
			.ok_or_else(|| anyhow::anyhow!("pack should install player targeting"))?;
		assert_eq!(host.1.kind, poi_intelligence::PoiKind::new("world/player"));
		assert_eq!(host.2.home, home);
		Ok(())
	}
}
