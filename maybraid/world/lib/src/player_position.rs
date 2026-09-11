//! Player position log and retained Discovery waypoints keyed by [`CharacterId`].

use std::fs;
use std::time::Duration;

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use crozon_character_persist::{CharacterId, PersistError, SaveRoot};
use durham_terrain_models::terrain_streaming_enabled;
use serde::{Deserialize, Serialize};

use chico_vegetation_on_terrain_playground::Player;

use crate::WorldPlayerLoadout;

const LOG_INTERVAL: Duration = Duration::from_secs(10);
const WAYPOINT_GAP_M: f32 = 500.0;
const WAYPOINT_CAP: usize = 5;
const FILE_VERSION: u32 = 1;

/// Last [`WAYPOINT_CAP`] player positions that differ by more than [`WAYPOINT_GAP_M`].
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct PlayerPositionWaypoints {
	pub owner: Option<CharacterId>,
	pub positions: Vec<Vec3>,
}

#[derive(Serialize, Deserialize)]
struct WaypointFile {
	version: u32,
	id: CharacterId,
	waypoints: Vec<[f32; 3]>,
}

pub struct PlayerPositionPlugin;

impl Plugin for PlayerPositionPlugin {
	fn build(&self, app: &mut App) {
		if !app.world().contains_resource::<SaveRoot>() {
			app.insert_resource(SaveRoot::workspace());
		}
		app.init_resource::<PlayerPositionWaypoints>().add_systems(
			Update,
			(
				sync_waypoints_for_character,
				retain_player_waypoints,
				log_player_position
					.run_if(terrain_streaming_enabled)
					.run_if(on_timer(LOG_INTERVAL)),
			)
				.chain(),
		);
	}
}

fn current_character_id(loadout: Option<&WorldPlayerLoadout>) -> Option<CharacterId> {
	loadout.and_then(|loadout| CharacterId::from_hex(&loadout.key))
}

fn sync_waypoints_for_character(
	save_root: Res<SaveRoot>,
	loadout: Option<Res<WorldPlayerLoadout>>,
	mut waypoints: ResMut<PlayerPositionWaypoints>,
) {
	let Some(id) = current_character_id(loadout.as_deref()) else {
		return;
	};
	if waypoints.owner == Some(id) {
		return;
	}
	*waypoints = load_waypoints(&save_root, id);
	if !waypoints.positions.is_empty() {
		info!(
			target: "world.player",
			"retained {} waypoint(s) for {}: {}",
			waypoints.positions.len(),
			id.to_hex(),
			format_waypoints(&waypoints.positions)
		);
	}
}

fn log_player_position(players: Query<&Transform, With<Player>>) {
	let Ok(transform) = players.single() else {
		return;
	};
	let p = transform.translation;
	info!(target: "world.player", "position ({:.1}, {:.1}, {:.1})", p.x, p.y, p.z);
}

fn retain_player_waypoints(
	streaming: Res<durham_terrain_models::TerrainStreamingEnabled>,
	save_root: Res<SaveRoot>,
	loadout: Option<Res<WorldPlayerLoadout>>,
	players: Query<&Transform, With<Player>>,
	mut waypoints: ResMut<PlayerPositionWaypoints>,
) {
	if !streaming.0 {
		return;
	}
	let Ok(transform) = players.single() else {
		return;
	};
	if !record_waypoint(&mut waypoints.positions, transform.translation) {
		return;
	}
	let Some(id) = current_character_id(loadout.as_deref()) else {
		return;
	};
	waypoints.owner = Some(id);
	if let Err(error) = save_waypoints(&save_root, id, &waypoints.positions) {
		warn!(target: "world.player", "failed to persist player waypoints: {error}");
		return;
	}
	info!(
		target: "world.player",
		"retained waypoint ({:.1}, {:.1}, {:.1}) ({}/{})",
		transform.translation.x,
		transform.translation.y,
		transform.translation.z,
		waypoints.positions.len(),
		WAYPOINT_CAP
	);
}

/// Push `at` when it is the first sample or more than [`WAYPOINT_GAP_M`] from the last.
/// Returns whether the list changed.
pub fn record_waypoint(positions: &mut Vec<Vec3>, at: Vec3) -> bool {
	if let Some(last) = positions.last() {
		if last.xz().distance(at.xz()) <= WAYPOINT_GAP_M {
			return false;
		}
	}
	positions.push(at);
	while positions.len() > WAYPOINT_CAP {
		positions.remove(0);
	}
	true
}

fn format_waypoints(positions: &[Vec3]) -> String {
	positions
		.iter()
		.map(|p| format!("({:.1},{:.1},{:.1})", p.x, p.y, p.z))
		.collect::<Vec<_>>()
		.join(" ")
}

fn load_waypoints(root: &SaveRoot, id: CharacterId) -> PlayerPositionWaypoints {
	let path = root.position_path(id);
	let Ok(json) = fs::read_to_string(&path) else {
		return PlayerPositionWaypoints { owner: Some(id), positions: Vec::new() };
	};
	let Ok(file) = serde_json::from_str::<WaypointFile>(&json) else {
		warn!(target: "world.player", "ignoring unreadable {}", path.display());
		return PlayerPositionWaypoints { owner: Some(id), positions: Vec::new() };
	};
	if file.version != FILE_VERSION {
		return PlayerPositionWaypoints { owner: Some(id), positions: Vec::new() };
	}
	PlayerPositionWaypoints {
		owner: Some(id),
		positions: file.waypoints.into_iter().map(|[x, y, z]| Vec3::new(x, y, z)).collect(),
	}
}

fn save_waypoints(
	root: &SaveRoot,
	id: CharacterId,
	positions: &[Vec3],
) -> Result<(), PersistError> {
	root.ensure_dirs()?;
	let file = WaypointFile {
		version: FILE_VERSION,
		id,
		waypoints: positions.iter().map(|p| [p.x, p.y, p.z]).collect(),
	};
	fs::write(root.position_path(id), serde_json::to_string_pretty(&file)?)?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn waypoints_keep_five_far_samples() {
		let mut positions = Vec::new();
		assert!(record_waypoint(&mut positions, Vec3::ZERO));
		assert!(!record_waypoint(&mut positions, Vec3::new(100.0, 0.0, 0.0)));
		assert!(record_waypoint(&mut positions, Vec3::new(600.0, 1.0, 0.0)));
		for i in 2..6 {
			assert!(record_waypoint(&mut positions, Vec3::new(600.0 * i as f32, 0.0, 0.0)));
		}
		assert_eq!(positions.len(), 5);
		assert_eq!(positions[0].x, 600.0);
		assert_eq!(positions[4].x, 3000.0);
	}

	#[test]
	fn waypoint_file_round_trips_beside_the_character() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(42);
		let positions = vec![Vec3::new(1.0, 2.0, 3.0), Vec3::new(800.0, 4.0, 9.0)];
		save_waypoints(&root, id, &positions)?;
		let loaded = load_waypoints(&root, id);
		assert_eq!(loaded.owner, Some(id));
		assert_eq!(loaded.positions, positions);
		assert_eq!(
			root.position_path(id),
			dir.path().join("positions").join(format!("{}.json", id.to_hex()))
		);
		Ok(())
	}
}
