//! Independent appearance recipes; all principal motion remains in the shared controller.
use crate::{
	composition::{ChoreographyOptions, CombinedChoreographyPart, EffectDefinition, EffectRecipe},
	layers::combined_choreography::CombinedChoreographyLayer,
	parts::cartoon_landing::motion::Puff,
};
use bevy::prelude::*;
#[derive(Component, Clone, Debug)]
pub struct CombinedLanding {
	pub layer: CombinedChoreographyLayer,
}
#[derive(Component, Clone, Debug)]
pub struct CombinedWaveContact {
	pub layer: CombinedChoreographyLayer,
}
impl CombinedChoreographyLayer {
	pub fn load(server: &AssetServer, landing: bool, contact: bool) -> Self {
		Self {
			part: CombinedChoreographyPart {
				assets: server.load("vfx/dust/combined.combined.json"),
				meshes: crate::composition::CartoonLandingPart::load(server).meshes,
				landing,
				contact,
				options: ChoreographyOptions::default(),
			},
		}
	}
}
impl CombinedLanding {
	pub fn load(server: &AssetServer) -> Self {
		Self { layer: CombinedChoreographyLayer::load(server, true, false) }
	}
	pub fn contact(server: &AssetServer) -> Self {
		Self { layer: CombinedChoreographyLayer::load(server, true, true) }
	}
	pub fn main_duration(&self) -> f64 {
		Puff::DURATION
	}
}
impl CombinedWaveContact {
	pub fn load(server: &AssetServer) -> Self {
		Self { layer: CombinedChoreographyLayer::load(server, false, true) }
	}
}
impl EffectRecipe for CombinedLanding {
	fn effect_definition(&self) -> EffectDefinition {
		self.layer.effect_definition()
	}
}
impl EffectRecipe for CombinedWaveContact {
	fn effect_definition(&self) -> EffectDefinition {
		self.layer.effect_definition()
	}
}
