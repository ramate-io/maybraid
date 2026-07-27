//! A floor of the Wizard's Tower.
//!
//! Geometry: circular outer [`RingWall`] with door/window portals, squared-off floor
//! with a centered spire hole, and a circular rough-stone tread run inside the spire
//! square that rises one storey. Each storey also carries a lantern-like point light
//! (mesh TBD).

use bevy::prelude::{Color, PointLight, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;
use richmond_building_components::floors::{rough_stone_floor, Floor};
use richmond_building_components::partitions::rough_stone_wall;
use richmond_building_components::stairs::{rough_stone_stair, Stair};
use richmond_building_components::{scene_children, Placed};

use crate::wizards_tower::floor_fill::{squared_floor_with_spire_hole, SPIRE_HALF_FRAC};
use crate::wizards_tower::ring_wall::{
	wizard_tower_must_assign, RingWall, RingWallParams,
};
use crate::CellConstraints;

/// One storey of the circular tower.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerFloor {
	pub constraints: CellConstraints,
	/// Storey height in meters (outer ring wall \(Y\) scale).
	pub storey_height: f32,
	/// Circular outer ring with portals.
	pub ring: RingWall,
	/// Four circle−inscribed-square caps that square off the circular footprint.
	pub floor_caps: [Placed<Floor>; 4],
	/// Rectangular slabs filling the inscribed square around the spire hole.
	pub floor_rects: [Placed<Floor>; 4],
	/// Circular tread run inside the spire square, from this floor up to the next storey.
	pub stairs: Placed<Stair>,
	/// Warm lantern point light hanging over the usable floor (no mesh yet).
	pub lantern: Vec3,
}

impl WizardsTowerFloor {
	/// Build from column parent constraints, this floor's subsetted constraints,
	/// storey height, and portal noise (seeded per storey).
	pub fn new(
		_parent_constraints: &CellConstraints,
		constraints: CellConstraints,
		storey_height: f32,
		portal_noise: NoiseParams,
	) -> Self {
		let storey_height = storey_height.max(1e-4);
		let center = (constraints.aabb.min + constraints.aabb.max) * 0.5;
		let center_xz = Vec3::new(center.x, constraints.aabb.min.y, center.z);
		let extent = constraints.aabb.max - constraints.aabb.min;
		let radius = 0.5 * extent.x.min(extent.z);
		let spire_half = SPIRE_HALF_FRAC * radius;
		let (floor_caps, floor_rects) =
			squared_floor_with_spire_hole(center_xz, radius, spire_half);

		// Spiral stays inside the centered spire square (outer tread edge ≤ spire_half).
		let tread_width = spire_half * 0.45;
		let stair_radius = (spire_half - 0.5 * tread_width).max(1e-4);
		let tread_depth = tread_width * 0.55;

		// Hang over the floor ring, clear of the spire stairs (~chest / lantern height).
		let lantern = Vec3::new(
			center.x + 0.55 * radius,
			constraints.aabb.min.y + storey_height * 0.65,
			center.z,
		);

		let ring = RingWall::new(RingWallParams {
			center_xz,
			radius,
			storey_height,
			must_assign: wizard_tower_must_assign(),
			must_not_assign: vec![],
			portal_noise,
			// Attempt a few extra windows in the free arcs.
			optional_portals: (0, 2),
		});

		Self {
			storey_height,
			ring,
			floor_caps,
			floor_rects,
			stairs: Placed::new(
				Stair::spiral_run(storey_height, stair_radius, tread_width, tread_depth),
				center_xz,
				0.0,
			),
			lantern,
			constraints,
		}
	}
}

fn floor_lantern(at: Vec3, storey_height: f32) -> impl Scene + 'static {
	let range = (storey_height * 2.5).max(4.0);
	let transform = Transform::from_translation(at);
	bsn! {
		PointLight {
			color: Color::srgb(1.0, 0.72, 0.42),
			intensity: 2800.0,
			range: {range},
			shadow_maps_enabled: false,
		}
		template_value(transform)
		Visibility::default()
	}
}

impl LodScene for WizardsTowerFloor {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		for wall in &self.ring.walls {
			children.push(Box::new(rough_stone_wall(wall, lod_ref)));
		}
		for cap in &self.floor_caps {
			children.push(Box::new(rough_stone_floor(cap, lod_ref)));
		}
		for rect in &self.floor_rects {
			children.push(Box::new(rough_stone_floor(rect, lod_ref)));
		}
		children.push(Box::new(rough_stone_stair(&self.stairs, lod_ref)));
		children.push(Box::new(floor_lantern(self.lantern, self.storey_height)));
		scene_children(children)
	}
}
