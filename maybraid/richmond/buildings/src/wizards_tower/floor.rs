//! A floor of the Wizard's Tower.
//!
//! Geometry: outer crate-level [`crate::ArcWall`] with door/window portals, squared-off floor
//! with a centered spire hole, and a crate-level [`crate::ArcSpire`] tread run inside the
//! spire square that rises one storey. Each storey also carries a lantern-like point light
//! (mesh TBD).

use bevy::prelude::{Color, PointLight, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::Vec3;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;
use richmond_building_components::floors::FloorNode;
use richmond_building_components::scene_children;
use richmond_building_components::stairs::SpiralStair;

use crate::arc_spire::{uniform_storey_bindings, ArcSpire, ArcSpireParams, FitTolerance};
use crate::arc_wall::{ArcWall, ArcWallParams};
use crate::wizards_tower::floor_fill::{squared_floor_with_spire_hole, SPIRE_HALF_FRAC};
use crate::wizards_tower::must_assign_cardinal_portals;
use crate::CellConstraints;

/// One storey of the circular tower.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerFloor {
	pub constraints: CellConstraints,
	/// Storey height in meters (outer wall \(Y\) scale).
	pub storey_height: f32,
	/// Outer arc wall with portals.
	pub arc_wall: ArcWall,
	/// Four circle−inscribed-square caps that square off the circular footprint.
	pub floor_caps: [FloorNode; 4],
	/// Rectangular slabs filling the inscribed square around the spire hole.
	pub floor_rects: [FloorNode; 4],
	/// Circular tread spire inside the spire square, fitted to storey \(Y\) bindings.
	pub arc_spire: ArcSpire,
	/// Warm lantern point light hanging over the usable floor (no mesh yet).
	pub lantern: Vec3,
}

impl WizardsTowerFloor {
	/// Build from this floor's subsetted constraints, storey height, and portal noise
	/// (seeded per storey).
	pub fn new(
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
		let target_tread_height = SpiralStair::DEFAULT_TREAD_HEIGHT;

		// Hang over the floor ring, clear of the spire stairs (~chest / lantern height).
		let lantern = Vec3::new(
			center.x + 0.55 * radius,
			constraints.aabb.min.y + storey_height * 0.65,
			center.z,
		);

		let arc_wall = ArcWall::new(ArcWallParams {
			center_xz,
			radius,
			storey_height,
			arc_degrees: 360.0,
			must_assign: must_assign_cardinal_portals(),
			must_not_assign: vec![],
			portal_noise,
			optional_portals: (0, 2),
		});

		let arc_spire = ArcSpire::new(ArcSpireParams {
			center_xz,
			radius: stair_radius,
			tread_width,
			tread_depth,
			target_tread_height,
			y_bindings: uniform_storey_bindings(center_xz.y, storey_height, target_tread_height),
			fit_tolerance: FitTolerance::default(),
			turns: 1.0,
		});

		Self {
			storey_height,
			arc_wall,
			floor_caps,
			floor_rects,
			arc_spire,
			lantern,
			constraints,
		}
	}

	pub(crate) fn emit_external_features(
		&self,
		children: &mut Vec<Box<dyn Scene>>,
		lod_ref: &LodRef,
	) {
		for wall in &self.arc_wall.walls {
			children.push(Box::new(wall.scene_with_lod(lod_ref)));
		}
	}

	pub(crate) fn emit_internal_features(
		&self,
		children: &mut Vec<Box<dyn Scene>>,
		lod_ref: &LodRef,
	) {
		use richmond_building_components::{confined_scene, ParentConfines};

		// Floor-compartment ball (storey AABB), not the whole tower.
		let confines = ParentConfines::internal(
			self.storey_confine_center(),
			self.storey_confine_radius(),
		);
		for cap in &self.floor_caps {
			children.push(Box::new(
				cap.clone()
					.with_confines(confines)
					.scene_with_lod(lod_ref),
			));
		}
		for rect in &self.floor_rects {
			children.push(Box::new(
				rect.clone()
					.with_confines(confines)
					.scene_with_lod(lod_ref),
			));
		}
		// Per-storey spire run: same floor ball. A continuous multi-storey spire
		// would use ParentConfines::Capsule instead.
		children.push(Box::new(
			self.arc_spire
				.stairs
				.clone()
				.with_confines(confines)
				.scene_with_lod(lod_ref),
		));
		children.push(Box::new(confined_scene(confines, self.lantern_scene())));
	}

	fn storey_confine_center(&self) -> Vec3 {
		let aabb = &self.constraints.aabb;
		Vec3::from((aabb.min + aabb.max) * 0.5)
	}

	fn storey_confine_radius(&self) -> f32 {
		let aabb = &self.constraints.aabb;
		let extent = aabb.max - aabb.min;
		(0.5 * extent.x.min(extent.z)).max(1e-4)
	}

	fn lantern_scene(&self) -> impl Scene + 'static {
		let range = (self.storey_height * 2.5).max(4.0);
		let transform = Transform::from_translation(self.lantern);
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
}

impl LodScene for WizardsTowerFloor {
	fn scene_lod_status(
		&self,
		_lod_ref: &LodRef,
	) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

		fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		_level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		self.emit_external_features(&mut children, lod_ref);
		self.emit_internal_features(&mut children, lod_ref);
		scene_children(children)
	}
}
