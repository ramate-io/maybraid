use bevy::prelude::*;

use crate::{PoiId, PoiKind};

/// Vertical support contract for an arrival pin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PoiSupport {
	/// The pin is an XZ destination; runtime may replace its height with terrain.
	#[default]
	Terrain,
	/// The pin's authored Y is its standing floor and must be preserved.
	Floor,
}

/// Position, radius, and support contract for arriving at a POI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoiArrival {
	pub position: Vec3,
	pub radius: f32,
	pub support: PoiSupport,
}

impl PoiArrival {
	pub fn terrain(position: Vec3, radius: f32) -> Self {
		Self { position, radius, support: PoiSupport::Terrain }
	}

	pub fn floor(position: Vec3, radius: f32) -> Self {
		Self { position, radius, support: PoiSupport::Floor }
	}

	pub fn surface_y(self, terrain_y: f32) -> f32 {
		match self.support {
			PoiSupport::Terrain => terrain_y,
			PoiSupport::Floor => self.position.y,
		}
	}

	pub fn stands_on_floor(self) -> bool {
		self.support == PoiSupport::Floor
	}
}

pub const MAX_POI_ARRIVAL_RADIUS: f32 = 256.0;

/// Semantic identity and arrival region for an entity-backed point of interest.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Poi {
	pub id: PoiId,
	pub kind: PoiKind,
	pub arrival_radius: f32,
	pub support: PoiSupport,
	pub salience: f32,
}

impl Poi {
	pub fn new(id: PoiId, kind: PoiKind) -> Self {
		Self { id, kind, arrival_radius: 1.0, support: PoiSupport::Terrain, salience: 1.0 }
	}

	pub fn with_support(mut self, support: PoiSupport) -> Self {
		self.support = support;
		self
	}

	pub fn with_floor_support(self) -> Self {
		self.with_support(PoiSupport::Floor)
	}

	pub fn with_arrival_radius(mut self, radius: f32) -> Self {
		if radius.is_finite() {
			self.arrival_radius = radius.max(0.0);
		}
		self
	}

	pub fn with_salience(mut self, salience: f32) -> Self {
		if salience.is_finite() {
			self.salience = salience.max(0.0);
		}
		self
	}
}

/// Include this [`Poi`] in bounded, nearby spatial scans.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalPoi;

/// Include this sparse [`Poi`] in inexpensive whole-map scans.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlobalPoi;
