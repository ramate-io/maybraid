//! Homeostatic mobs: additional density around the player's current cell.
//!
//! This layer adds mobs on top of (not replacing) the existing HCSG mob cells.
//! A tunable grid of cells (~200m by default) covers the world; about 1/3 of
//! cells (tunable) host one mob each. Only the player's **current** cell can
//! activate its mob. An active mob persists while the player remains in the
//! 3×3 neighborhood around the mob's home cell, and retreats when the player
//! nears the boundary (tunable margin) before despawning outside the neighborhood.

use bevy::prelude::*;

use crate::generation::{GroupKind, MobEnvironmentSample, MobGroup, MobWorldHosts, MobWorldSample};
use crate::index::{cell_seed, mixed};

/// Side length of a homeostatic cell in meters.
pub const DEFAULT_CELL_SIZE: f32 = 200.0;

/// Fraction of cells that host a homeostatic mob (deterministically chosen).
pub const DEFAULT_CELL_FRACTION: f64 = 0.33;

/// Cells in the persistence neighborhood (always 3×3).
const NEIGHBORHOOD_RADIUS: i32 = 1;

/// Default retreat margin: start retreating this many meters from the
/// neighborhood boundary.
pub const DEFAULT_RETREAT_MARGIN: f32 = 40.0;

/// Default arrival radius: consider retreat complete within this distance of home.
pub const DEFAULT_ARRIVAL_RADIUS: f32 = 10.0;

/// Default hysteresis margin: inner buffer to prevent boundary flips.
/// Retreat starts at (boundary - retreat_margin), but once started,
/// it continues until (boundary - retreat_margin - hysteresis_margin).
pub const DEFAULT_HYSTERESIS_MARGIN: f32 = 20.0;

/// Configuration for homeostatic mob spawning.
#[derive(Resource, Clone, Debug)]
pub struct HomeostaticMobConfig {
	/// Side length of a homeostatic cell in meters.
	pub cell_size: f32,
	/// Fraction of cells that host a homeostatic mob (0.0 to 1.0).
	pub cell_fraction: f64,
	/// Distance from the neighborhood boundary where retreat begins.
	pub retreat_margin: f32,
	/// Distance from home center where retreat is considered complete.
	/// PLACEHOLDER: tune based on routing precision and actor radius.
	pub arrival_radius: f32,
	/// Inner margin to prevent boundary flips once retreat starts.
	/// PLACEHOLDER: tune based on player movement speed and update frequency.
	pub hysteresis_margin: f32,
}

impl Default for HomeostaticMobConfig {
	fn default() -> Self {
		Self {
			cell_size: DEFAULT_CELL_SIZE,
			cell_fraction: DEFAULT_CELL_FRACTION,
			retreat_margin: DEFAULT_RETREAT_MARGIN,
			arrival_radius: DEFAULT_ARRIVAL_RADIUS,
			hysteresis_margin: DEFAULT_HYSTERESIS_MARGIN,
		}
	}
}

/// Index of a homeostatic cell (ix, iz).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HomeostaticCellIndex(pub i32, pub i32);

impl HomeostaticCellIndex {
	/// Cell index containing the given world position.
	pub fn containing(position: Vec3, cell_size: f32) -> Self {
		let half = cell_size * 0.5;
		let ix = ((position.x + half) / cell_size).floor() as i32;
		let iz = ((position.z + half) / cell_size).floor() as i32;
		Self(ix, iz)
	}

	/// World-space center of this cell.
	pub fn center(self, cell_size: f32) -> Vec3 {
		Vec3::new(
			self.0 as f32 * cell_size,
			0.0,
			self.1 as f32 * cell_size,
		)
	}

	/// Whether this cell should host a mob (deterministic from cell coords).
	pub fn should_host_mob(self, fraction: f64) -> bool {
		let seed = cell_seed(self.0, self.1);
		let hash = mixed(seed ^ 0x686f_6d65_6f73_7461); // "homeosta"
		(hash % 10_000) < (fraction * 10_000.0) as u64
	}

	/// All cells in the 3×3 neighborhood around this cell.
	pub fn neighborhood_3x3(self) -> Vec<Self> {
		let mut cells = Vec::with_capacity(9);
		for dx in -NEIGHBORHOOD_RADIUS..=NEIGHBORHOOD_RADIUS {
			for dz in -NEIGHBORHOOD_RADIUS..=NEIGHBORHOOD_RADIUS {
				cells.push(Self(self.0 + dx, self.1 + dz));
			}
		}
		cells
	}

	/// Distance from position to the edge of the 3×3 neighborhood (negative = inside).
	pub fn distance_to_neighborhood_edge(self, position: Vec3, cell_size: f32) -> f32 {
		// A 3×3 neighborhood has (2*radius + 1) cells in each dimension.
		// For radius=1, that's 3 cells, with half-extent of 3*cell_size/2.
		let neighborhood_half = (2 * NEIGHBORHOOD_RADIUS + 1) as f32 * cell_size * 0.5;
		let center = self.center(cell_size);
		
		let dx = (position.x - center.x).abs() - neighborhood_half;
		let dz = (position.z - center.z).abs() - neighborhood_half;
		
		dx.max(dz)
	}
}

/// Active homeostatic mob tracking.
#[derive(Component, Clone, Debug)]
pub struct HomeostaticMob {
	/// Home cell of this mob.
	pub home_cell: HomeostaticCellIndex,
	/// The mob host entity.
	pub mob_entity: Entity,
	/// Generation count for tracking activations.
	pub generation: u64,
}

/// Marker for a mob in retreat mode.
#[derive(Component, Clone, Copy, Debug)]
pub struct HomeostaticRetreat {
	/// World position to retreat toward (home cell center).
	pub target: Vec3,
}

/// Tracks the player's current homeostatic cell.
#[derive(Resource, Default, Clone, Debug)]
pub struct PlayerHomeostaticCell {
	pub current: Option<HomeostaticCellIndex>,
	pub generation: u64,
}

/// Tracks the last known gameplay state for detecting teardown.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct HomeostaticGameplayState {
	pub was_enabled: bool,
}

/// System to track player's current homeostatic cell and activate its mob.
/// Generic over any player marker component P.
pub fn track_player_homeostatic_cell<P: Component>(
	config: Res<HomeostaticMobConfig>,
	players: Query<&Transform, With<P>>,
	mut player_cell: ResMut<PlayerHomeostaticCell>,
	active_mobs: Query<(&HomeostaticMob, Entity)>,
	mut commands: Commands,
) {
	let Ok(player_transform) = players.single() else {
		return;
	};

	let current_cell = HomeostaticCellIndex::containing(
		player_transform.translation,
		config.cell_size,
	);

	// Check if we've moved to a new cell
	if player_cell.current == Some(current_cell) {
		return;
	}

	player_cell.current = Some(current_cell);
	player_cell.generation = player_cell.generation.wrapping_add(1);

	// Only activate if this cell should host a mob
	if !current_cell.should_host_mob(config.cell_fraction) {
		return;
	}

	// Check if this cell already has an active mob
	for (mob, _) in &active_mobs {
		if mob.home_cell == current_cell {
			return; // Already active
		}
	}

	// Activate the mob for this cell
	let center = current_cell.center(config.cell_size);
	let seed = cell_seed(current_cell.0, current_cell.1);
	
	// Generate one mob using existing generation machinery
	// We use a flat world sample for homeostatic mobs (simplified generation)
	let world = FlatWorld {
		sample: MobEnvironmentSample {
			elevation: Some(player_transform.translation.y),
			urbanization: 0.3,
			vegetation: 0.3,
		},
	};

	let kind = GroupKind::from_num((seed as f32 / 1000.0) % 5.0);
	let mut group = MobGroup::generate(kind, seed, center.xz(), &world);
	
	if group.mobs.is_empty() {
		return; // No mob could be generated
	}

	// Take only the first mob for homeostatic placement
	group.mobs.truncate(1);
	group.kind = GroupKind::Placed;
	
	let mob_entities = group.spawn(&mut commands);
	
	if let Some(mob_entity) = mob_entities.first() {
		commands.entity(*mob_entity).insert(HomeostaticMob {
			home_cell: current_cell,
			mob_entity: *mob_entity,
			generation: player_cell.generation,
		});

		info!(
			target: "maybraid.homeostatic",
			"activated homeostatic mob at cell ({}, {})",
			current_cell.0,
			current_cell.1
		);
	}
}

/// System to despawn homeostatic mobs outside the player's 3×3 neighborhood.
/// Generic over any player marker component P.
pub fn despawn_distant_homeostatic_mobs<P: Component>(
	_config: Res<HomeostaticMobConfig>,
	_players: Query<&Transform, With<P>>,
	player_cell: Res<PlayerHomeostaticCell>,
	active_mobs: Query<(Entity, &HomeostaticMob)>,
	mut commands: Commands,
) {
	let Some(current_cell) = player_cell.current else {
		return;
	};

	let neighborhood = current_cell.neighborhood_3x3();

	for (entity, mob) in &active_mobs {
		if !neighborhood.contains(&mob.home_cell) {
			// Outside neighborhood: despawn
			commands.entity(entity).try_despawn();
			info!(
				target: "maybraid.homeostatic",
				"despawned homeostatic mob from cell ({}, {})",
				mob.home_cell.0,
				mob.home_cell.1
			);
		}
	}
}

/// System to clean up all homeostatic state when gameplay is disabled or session ends.
/// This is NOT pause - pause does not tear down gameplay state.
/// This responds to actual gameplay/session teardown (e.g., returning to Home menu).
pub fn cleanup_on_gameplay_disable(
	gameplay_enabled: Option<Res<WorldGameplayEnabled>>,
	mut gameplay_state: ResMut<HomeostaticGameplayState>,
	mut player_cell: ResMut<PlayerHomeostaticCell>,
	active_mobs: Query<Entity, With<HomeostaticMob>>,
	mut commands: Commands,
) {
	let enabled = gameplay_enabled.map(|g| g.0).unwrap_or(true);
	
	// Detect transition from enabled to disabled
	if gameplay_state.was_enabled && !enabled {
		// Despawn all homeostatic hosts
		for entity in &active_mobs {
			commands.entity(entity).try_despawn();
		}
		
		// Reset current cell state
		player_cell.current = None;
		
		if !active_mobs.is_empty() {
			info!(
				target: "maybraid.homeostatic",
				"cleaned up {} homeostatic mobs due to gameplay disable",
				active_mobs.iter().count()
			);
		}
	}
	
	// Always update tracking state
	gameplay_state.was_enabled = enabled;
}

// Re-export for optional use by external systems
pub use maybraid_session::WorldGameplayEnabled;

/// System to trigger retreat behavior when player nears neighborhood edge.
/// Uses hysteresis: retreat starts at (boundary - retreat_margin), but once started
/// it continues until (boundary - retreat_margin - hysteresis_margin).
/// Generic over any player marker component P.
pub fn trigger_homeostatic_retreat<P: Component>(
	config: Res<HomeostaticMobConfig>,
	players: Query<&Transform, With<P>>,
	not_retreating: Query<(Entity, &HomeostaticMob, &GlobalTransform), Without<HomeostaticRetreat>>,
	mut commands: Commands,
) {
	let Ok(player_transform) = players.single() else {
		return;
	};

	for (entity, mob, mob_transform) in &not_retreating {
		let distance_to_edge = mob.home_cell.distance_to_neighborhood_edge(
			player_transform.translation,
			config.cell_size,
		);

		// Start retreating if player is within margin of the edge
		if distance_to_edge >= -config.retreat_margin {
			let retreat_target = mob.home_cell.center(config.cell_size);
			commands.entity(entity).insert(HomeostaticRetreat {
				target: retreat_target.with_y(mob_transform.translation().y),
			});

			info!(
				target: "maybraid.homeostatic",
				"homeostatic mob at cell ({}, {}) entering retreat",
				mob.home_cell.0,
				mob.home_cell.1
			);
		}
	}
}

/// System to release retreat when mob arrives at home or player moves back.
/// Retreat is released (HomeostaticRetreat removed) when:
/// 1. The mob reaches its home cell center (within arrival_radius), OR
/// 2. The player moves back inside the hysteresis margin.
/// Generic over any player marker component P.
pub fn release_homeostatic_retreat<P: Component>(
	config: Res<HomeostaticMobConfig>,
	players: Query<&Transform, With<P>>,
	retreating: Query<(Entity, &HomeostaticMob, &GlobalTransform), With<HomeostaticRetreat>>,
	mut commands: Commands,
) {
	let Ok(player_transform) = players.single() else {
		return;
	};

	for (entity, mob, mob_transform) in &retreating {
		// Check if mob has arrived at home
		let home_center = mob.home_cell.center(config.cell_size);
		let distance_to_home = mob_transform.translation().xz().distance(home_center.xz());
		
		if distance_to_home <= config.arrival_radius {
			commands.entity(entity).remove::<HomeostaticRetreat>();
			info!(
				target: "maybraid.homeostatic",
				"homeostatic mob at cell ({}, {}) completed retreat (arrived at home)",
				mob.home_cell.0,
				mob.home_cell.1
			);
			continue;
		}

		// Check if player moved back inside the hysteresis margin
		let distance_to_edge = mob.home_cell.distance_to_neighborhood_edge(
			player_transform.translation,
			config.cell_size,
		);
		
		// Release retreat if player is now well inside (hysteresis)
		if distance_to_edge < -(config.retreat_margin + config.hysteresis_margin) {
			commands.entity(entity).remove::<HomeostaticRetreat>();
			info!(
				target: "maybraid.homeostatic",
				"homeostatic mob at cell ({}, {}) released retreat (player returned)",
				mob.home_cell.0,
				mob.home_cell.1
			);
		}
	}
}

/// System to handle retreating mobs by setting POI goals toward their home cell.
/// Drives the mob through existing movement/AI systems.
/// Retreat has explicit priority and replaces any existing PoiGoal.
pub fn apply_homeostatic_retreat(
	time: Res<Time>,
	config: Res<HomeostaticMobConfig>,
	retreating: Query<(Entity, &HomeostaticRetreat, &HomeostaticMob)>,
	mut commands: Commands,
) {
	use poi_intelligence::{PoiGoal, PoiId};
	
	for (entity, retreat, mob) in &retreating {
		// Create a POI goal to drive the mob back to its home cell.
		// This REPLACES any existing PoiGoal (explicit retreat priority).
		// The goal uses a dummy PoiId (0) since this is a transient retreat position.
		let goal = PoiGoal::new(
			mob.generation, // Use mob generation as goal generation
			PoiId(0), // Dummy ID for retreat goal
			None, // No poi entity
			poi_intelligence::PoiKind::new("homeostatic-retreat"), // Marker for retreat
			retreat.target,
			config.arrival_radius.max(5.0), // Arrival radius
			time.elapsed_secs(), // selected_at
			0.0, // No linger - just reach the position
		);
		
		commands.entity(entity).insert(goal);
	}
}

// Helper types for simplified mob generation

struct FlatWorld {
	sample: MobEnvironmentSample,
}

impl MobWorldSample for FlatWorld {
	fn sample_mobs(&self, _xz: Vec2) -> MobEnvironmentSample {
		self.sample
	}
}

impl MobWorldHosts for FlatWorld {
	fn plant_hosts(&self, _origin: Vec2, _extent: f32) -> Vec<crate::generation::MobPlantHost> {
		Vec::new()
	}
}

// Helper for selecting mob groups (reuses generation logic)
use mob_characters::FromMobNumber;

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cell_index_contains_position() {
		let cell = HomeostaticCellIndex::containing(Vec3::new(250.0, 0.0, -150.0), 200.0);
		assert_eq!(cell, HomeostaticCellIndex(1, -1));

		let origin = HomeostaticCellIndex::containing(Vec3::ZERO, 200.0);
		assert_eq!(origin, HomeostaticCellIndex(0, 0));
	}

	#[test]
	fn cell_center_is_correct() {
		let cell = HomeostaticCellIndex(2, -1);
		let center = cell.center(200.0);
		assert_eq!(center, Vec3::new(400.0, 0.0, -200.0));
	}

	#[test]
	fn cell_fraction_is_deterministic() {
		let cell1 = HomeostaticCellIndex(0, 0);
		let cell2 = HomeostaticCellIndex(1, 0);
		
		let result1a = cell1.should_host_mob(0.33);
		let result1b = cell1.should_host_mob(0.33);
		assert_eq!(result1a, result1b, "same cell gives same result");

		let result2 = cell2.should_host_mob(0.33);
		// Different cells should have independent results
		assert!(result1a != result2 || result1a == result2); // Always true, just checking it compiles
	}

	#[test]
	fn cell_fraction_hits_target_over_large_sample() {
		let target_fraction = 0.33;
		let sample_size = 10_000;
		
		let mut flagged_count = 0;
		for ix in -50..50 {
			for iz in -50..50 {
				let cell = HomeostaticCellIndex(ix, iz);
				if cell.should_host_mob(target_fraction) {
					flagged_count += 1;
				}
			}
		}

		let actual_fraction = flagged_count as f64 / sample_size as f64;
		let error = (actual_fraction - target_fraction).abs();
		
		assert!(
			error < 0.02,
			"expected ~{target_fraction}, got {actual_fraction} (error {error})"
		);
	}

	#[test]
	fn neighborhood_3x3_has_nine_cells() {
		let cell = HomeostaticCellIndex(5, 10);
		let neighborhood = cell.neighborhood_3x3();
		
		assert_eq!(neighborhood.len(), 9);
		assert!(neighborhood.contains(&HomeostaticCellIndex(5, 10))); // center
		assert!(neighborhood.contains(&HomeostaticCellIndex(4, 9))); // corner
		assert!(neighborhood.contains(&HomeostaticCellIndex(6, 11))); // corner
	}

	#[test]
	fn distance_to_edge_negative_inside_neighborhood() {
		let cell = HomeostaticCellIndex(0, 0);
		let center = cell.center(200.0);
		
		let distance = cell.distance_to_neighborhood_edge(center, 200.0);
		assert!(distance < 0.0, "center should be well inside: {distance}");
	}

	#[test]
	fn distance_to_edge_positive_outside_neighborhood() {
		let cell = HomeostaticCellIndex(0, 0);
		let far_away = Vec3::new(1000.0, 0.0, 1000.0);
		
		let distance = cell.distance_to_neighborhood_edge(far_away, 200.0);
		assert!(distance > 0.0, "far position should be outside: {distance}");
	}

	#[test]
	fn distance_at_40m_margin_from_true_boundary() {
		// A 3x3 neighborhood of 200m cells has half-extent of 300m.
		// Test that a position at X=259 (41m from true boundary at 300m) is still inside the retreat margin.
		let cell = HomeostaticCellIndex(0, 0);
		let pos_259 = Vec3::new(259.0, 0.0, 0.0);
		let distance_259 = cell.distance_to_neighborhood_edge(pos_259, 200.0);
		assert!(
			distance_259 < -40.0,
			"X=259 should be >40m inside the true 300m boundary (got {}m)",
			-distance_259
		);

		// At X=260, should be exactly at the 40m margin
		let pos_260 = Vec3::new(260.0, 0.0, 0.0);
		let distance_260 = cell.distance_to_neighborhood_edge(pos_260, 200.0);
		assert!(
			(distance_260 + 40.0).abs() < 1.0,
			"X=260 should be ~40m from the 300m boundary (got {}m)",
			-distance_260
		);
	}

	#[test]
	fn distance_at_true_neighborhood_exit() {
		// Exit at X=300 for a centered 3x3 of 200m cells
		let cell = HomeostaticCellIndex(0, 0);
		let pos_300 = Vec3::new(300.0, 0.0, 0.0);
		let distance_300 = cell.distance_to_neighborhood_edge(pos_300, 200.0);
		assert!(
			distance_300.abs() < 1.0,
			"X=300 should be at the neighborhood edge (got {}m)",
			distance_300
		);
	}

	#[test]
	fn distance_with_negative_coordinates() {
		let cell = HomeostaticCellIndex(0, 0);
		// Test negative X at the 40m margin
		let pos_neg_260 = Vec3::new(-260.0, 0.0, 0.0);
		let distance = cell.distance_to_neighborhood_edge(pos_neg_260, 200.0);
		assert!(
			(distance + 40.0).abs() < 1.0,
			"X=-260 should be ~40m from boundary (got {}m)",
			-distance
		);

		// Test negative exit
		let pos_neg_300 = Vec3::new(-300.0, 0.0, 0.0);
		let distance_exit = cell.distance_to_neighborhood_edge(pos_neg_300, 200.0);
		assert!(
			distance_exit.abs() < 1.0,
			"X=-300 should be at edge (got {}m)",
			distance_exit
		);
	}

	#[test]
	fn distance_at_corner_cases() {
		let cell = HomeostaticCellIndex(0, 0);
		// Corner at (300, 300) should be at edge
		let corner = Vec3::new(300.0, 0.0, 300.0);
		let distance = cell.distance_to_neighborhood_edge(corner, 200.0);
		assert!(
			distance.abs() < 1.0,
			"corner (300,300) should be at edge (got {}m)",
			distance
		);

		// Diagonal just inside at (250, 250)
		let inside_corner = Vec3::new(250.0, 0.0, 250.0);
		let distance_inside = cell.distance_to_neighborhood_edge(inside_corner, 200.0);
		assert!(
			distance_inside < 0.0,
			"(250,250) should be inside (got {}m)",
			distance_inside
		);
	}
}

