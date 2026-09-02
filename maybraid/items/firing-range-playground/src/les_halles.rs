//! Fit and spawn a small mixed-use Les Halles development for multi-storey pathing.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use movement_intelligence_richmond::{
	circulation_from_stairwell, circulation_from_storey, CirculationStairwell,
};
use procedural_common::NoiseParams;
use richmond_building_components::{building_bounds, spawn_building_components};
use richmond_building_physics::{spawn_building_walk_colliders, BUILDING_FRICTION};
use richmond_buildings::{Confines, Fit, Openings};
use richmond_developments::MixedUseLesHallesDevelopment;


/// Footprint large enough for a monotower (`≥ 28 m`) and two storeys (`height ≥ 10 m`).
const TOWER_EXTENT: Vec3 = Vec3::new(36.0, 10.0, 36.0);
const TOWER_SEED: i32 = 1337;

/// Capsule spawn poses next to a ground-floor stair mouth.
#[derive(Resource, Clone, Copy, Debug)]
pub struct LesHallesSpawn {
	pub player: Vec3,
	pub npc: Vec3,
	pub look_yaw: f32,
}

impl Default for LesHallesSpawn {
	fn default() -> Self {
		let h = player::capsule_spawn_height();
		Self {
			player: Vec3::new(0.0, h, 0.0),
			npc: Vec3::new(-6.5, h, -8.0),
			look_yaw: -std::f32::consts::FRAC_PI_2,
		}
	}
}

pub(crate) fn setup_les_halles(mut commands: Commands) {
	let bounds = confines_bounds(TOWER_EXTENT);
	let confines = Confines::new(bounds, 0.0, Openings::new());
	let noise = NoiseParams { seed: TOWER_SEED, ..NoiseParams::default() };
	match MixedUseLesHallesDevelopment::fit_to_confines(&confines, noise) {
		Ok((dev, _)) => spawn_development(&mut commands, dev, Transform::IDENTITY),
		Err(err) => {
			bevy::log::error!("Les Halles development fit failed: {err}");
			commands.insert_resource(LesHallesSpawn::default());
		}
	}
}

fn confines_bounds(extent: Vec3) -> Aabb3d {
	let hx = extent.x.max(1e-4) * 0.5;
	let hz = extent.z.max(1e-4) * 0.5;
	let h = extent.y.max(1e-4);
	Aabb3d::from_min_max(Vec3::new(-hx, 0.0, -hz), Vec3::new(hx, h, hz))
}

fn spawn_development(
	commands: &mut Commands,
	dev: MixedUseLesHallesDevelopment,
	transform: Transform,
) {
	let mut spawn = LesHallesSpawn::default();
	let n_floors = dev.tower.floors.len();
	for (id, floor) in dev.tower.floors.iter().enumerate() {
		let bounds = building_bounds(floor);
		for entity in spawn_building_components(commands, floor, transform, bounds) {
			spawn_building_walk_colliders(commands, entity, floor, BUILDING_FRICTION);
			commands.entity(entity).insert(circulation_from_storey(id as u32, floor, transform));
		}
	}

	let mut stair_k = 0usize;
	let mut placed_near_stair = false;
	if n_floors >= 2 {
		let last_well_i = n_floors - 2;
		for floor_i in 0..=last_well_i {
		let n_shafts =
			dev.tower.floors.get(floor_i).map(|f| f.floor_plan().shaft_bounds.len()).unwrap_or(0);
		for _ in 0..n_shafts {
			let Some(stairwell) = dev.stairwells.get(stair_k) else {
				break;
			};
			let bounds = building_bounds(stairwell);
			let from_id = floor_i as u32;
			let to_id = from_id + 1;
			let link = circulation_from_stairwell(from_id, to_id, stairwell, transform);
			if !placed_near_stair {
				spawn = spawn_from_link(&link);
				placed_near_stair = true;
			}
			for entity in spawn_building_components(commands, stairwell, transform, bounds) {
				spawn_building_walk_colliders(commands, entity, stairwell, BUILDING_FRICTION);
				commands.entity(entity).insert(link.clone());
			}
			stair_k += 1;
		}
		}
	}

	let bounds = building_bounds(&dev.roof);
	for entity in spawn_building_components(commands, &dev.roof, transform, bounds) {
		spawn_building_walk_colliders(commands, entity, &dev.roof, BUILDING_FRICTION);
	}

	if n_floors < 2 || !placed_near_stair {
		bevy::log::warn!(
			"Les Halles stack has {n_floors} storey(s); NPC multi-level follow needs at least two"
		);
	}
	commands.insert_resource(spawn);
}

fn spawn_from_link(link: &CirculationStairwell) -> LesHallesSpawn {
	let h = player::capsule_spawn_height();
	let mouth = Vec3::new(link.mouth.x, h.max(link.mouth.y + 0.15), link.mouth.z);
	let away = {
		let d = Vec3::new(mouth.x, 0.0, mouth.z);
		if d.length_squared() < 1e-4 {
			Vec3::X
		} else {
			d.normalize()
		}
	};
	// Stand in the courtyard, looking toward the well.
	let player = mouth - away * 2.4;
	let npc = mouth - away * 4.2 + Vec3::new(-away.z, 0.0, away.x) * 1.6;
	let look_yaw = (-away.z).atan2(-away.x);
	LesHallesSpawn {
		player: Vec3::new(player.x, h, player.z),
		npc: Vec3::new(npc.x, h, npc.z),
		look_yaw,
	}
}

pub(crate) fn draw_circulation_gizmos(mut gizmos: Gizmos, links: Query<&CirculationStairwell>) {
	for link in &links {
		let mut prev: Option<Vec3> = None;
		for p in &link.polyline {
			let lifted = *p + Vec3::Y * 0.15;
			gizmos.sphere(Isometry3d::from_translation(lifted), 0.12, Color::srgb(0.95, 0.85, 0.2));
			if let Some(a) = prev {
				gizmos.line(a, lifted, Color::srgb(0.95, 0.85, 0.2));
			}
			prev = Some(lifted);
		}
		gizmos.sphere(
			Isometry3d::from_translation(link.mouth + Vec3::Y * 0.2),
			0.18,
			Color::srgb(0.2, 0.9, 0.35),
		);
		gizmos.sphere(
			Isometry3d::from_translation(link.landing + Vec3::Y * 0.2),
			0.18,
			Color::srgb(0.9, 0.25, 0.3),
		);
	}
}
