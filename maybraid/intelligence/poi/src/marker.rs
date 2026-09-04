use bevy::prelude::*;

use crate::{PoiId, PoiKind};

pub const MAX_POI_ARRIVAL_RADIUS: f32 = 256.0;

/// Semantic identity and arrival region for an entity-backed point of interest.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Poi {
	pub id: PoiId,
	pub kind: PoiKind,
	pub arrival_radius: f32,
	pub salience: f32,
}

impl Poi {
	pub fn new(id: PoiId, kind: PoiKind) -> Self {
		Self { id, kind, arrival_radius: 1.0, salience: 1.0 }
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
