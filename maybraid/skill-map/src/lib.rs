//! Discover skill maps: a 2D paradimension a [`SkillMapUser`] steers while both
//! bumpers are down. Walks claim authored tiles that dispatch world effects.
//!
//! Stamp [`SkillMapUser`] on the live character (same install as
//! [`firearm_user::FirearmUser`]). Pause / text-entry should clear
//! [`SkillMapEnabled`].

mod cursor;
mod effects;
mod fireball_embers;
mod fireball_material;
mod fireball_trail;
mod map;
mod tile_material;
mod tiles;
mod user;
mod viewport;

use bevy::prelude::*;
use fireball_material::FireballMaterialPlugin;
use tile_material::SkillMapTileMaterialPlugin;

use maybraid_character_controller::CharacterControlSystems;
use projectiles::ProjectilesPlugin;
use threat_management_intelligence::ThreatManagementSystems;

pub use effects::{
	forget_chance, FIREBALL_COLOR, FIREBALL_GRAVITY, FIREBALL_RADIUS, FIREBALL_SPEED,
};
pub use map::{authored_map, authored_map_from_spec, authored_maps, SkillKind, SkillMapId};
pub use tiles::{classify_noise, TileKind};
pub use user::{
	spawn_skill_maps, spawn_skill_maps_with, MappedBy, SkillMapEquip, SkillMapHeld, SkillMapMember,
	SkillMapSession, SkillMapSteerLock, SkillMapUser, SkillMapUserSettings,
};
pub use viewport::{Debraid, SkillMapViewport};

/// When `false`, the map is hidden and claims / steering are ignored.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillMapEnabled(pub bool);

impl Default for SkillMapEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// One claimed power tile or a water fail, attributed to the mapping user.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillMapEvent {
	Claim { user: Entity, kind: SkillKind },
	Fail { user: Entity, map: SkillMapId },
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkillMapSystems {
	Spawn,
	Steer,
	Collide,
	Dispatch,
}

pub struct SkillMapPlugin;

impl Plugin for SkillMapPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ProjectilesPlugin>() {
			app.add_plugins(ProjectilesPlugin);
		}
		if !app.is_plugin_added::<bevy_hanabi::HanabiPlugin>() {
			app.add_plugins(bevy_hanabi::HanabiPlugin);
		}
		app.add_plugins(FireballMaterialPlugin)
			.add_plugins(SkillMapTileMaterialPlugin)
			.add_systems(Startup, fireball_embers::setup_fireball_effects)
			.init_resource::<SkillMapEnabled>()
			.add_message::<SkillMapEvent>()
			.configure_sets(
				Update,
				(
					SkillMapSystems::Spawn.after(CharacterControlSystems),
					SkillMapSystems::Steer.after(SkillMapSystems::Spawn),
					SkillMapSystems::Collide.after(SkillMapSystems::Steer),
					SkillMapSystems::Dispatch
						.after(SkillMapSystems::Collide)
						.before(ThreatManagementSystems::Select),
				),
			)
			.add_systems(Update, viewport::present_skill_maps.in_set(SkillMapSystems::Spawn))
			.add_systems(
				Update,
				(
					cursor::apply_skill_map_intents,
					cursor::tick_steer_lock,
					cursor::steer_cursors,
					viewport::sync_viewport_chrome,
					viewport::track_cursors,
					viewport::tick_debraid,
				)
					.chain()
					.in_set(SkillMapSystems::Steer),
			)
			.add_systems(Update, tiles::collide_tiles.in_set(SkillMapSystems::Collide))
			.add_systems(
				Update,
				(
					tiles::restore_spent_tiles,
					effects::dispatch_fireballs,
					effects::dispatch_dumbwaves,
					effects::tick_pulses,
					user::despawn_orphaned_skill_maps,
				)
					.chain()
					.in_set(SkillMapSystems::Dispatch),
			)
			.add_systems(
				PostUpdate,
				(
					fireball_trail::drop_fireball_beads,
					fireball_trail::tick_fireball_beads,
					fireball_trail::contact_fireball_beads,
				)
					.chain()
					.after(TransformSystems::Propagate),
			);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn gameplay_off_is_an_explicit_gate() {
		assert!(SkillMapEnabled::default().0);
		assert!(!SkillMapEnabled(false).0);
	}

	#[test]
	fn authored_maps_are_fireball_and_dumbwave() {
		let maps = authored_maps();
		assert_eq!(maps.len(), 2);
		assert_eq!(maps[0].kind, SkillKind::Fireball);
		assert_eq!(maps[1].kind, SkillKind::Dumbwave);
		let seeded = authored_map(SkillKind::Fireball, 99);
		assert_eq!(seeded.seed, 99);
		assert_eq!(seeded.id, SkillMapId(0));
	}
}
