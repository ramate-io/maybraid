//! Terrain-placed Shepherds Village and Shepherds Commune developments.

use richmond_developments::{ShepherdsCommune, ShepherdsVillage};

#[derive(Debug, Clone)]
pub struct ShepherdsVillageDevelopment {
	pub village: ShepherdsVillage,
}

#[derive(Debug, Clone)]
pub struct ShepherdsCommuneDevelopment {
	pub commune: ShepherdsCommune,
}
