use bevy::prelude::*;

use crate::{InterestLayers, SpotBounds};

/// An entity that can be discovered and observed by spotting users.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct SpotSubject {
	pub layers: InterestLayers,
	pub bounds: SpotBounds,
	pub salience: f32,
}

impl SpotSubject {
	pub fn new(layers: InterestLayers, bounds: SpotBounds) -> Self {
		Self { layers, bounds, salience: 1.0 }
	}

	pub fn with_salience(mut self, salience: f32) -> Self {
		self.salience = salience;
		self
	}
}
