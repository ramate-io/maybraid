//! Bundled dual-band controller layouts (one Bevy resource for SystemParam limits).

use crate::terrain::stamps::canyon::{
	CanyonHighPassControllerLayout, CanyonLowPassControllerLayout,
};
use crate::terrain::stamps::massif::{
	MassifHighPassControllerLayout, MassifLowPassControllerLayout,
};
use crate::terrain::stamps::plateau::{
	PlateauHighPassControllerLayout, PlateauLowPassControllerLayout,
};
use crate::terrain::stamps::pocket_water::{
	PocketWaterHighPassControllerLayout, PocketWaterLowPassControllerLayout,
};
use crate::terrain::stamps::rolling::{
	RollingHighPassControllerLayout, RollingLowPassControllerLayout,
};
use crate::terrain::stamps::valley::{
	ValleyHighPassControllerLayout, ValleyLowPassControllerLayout,
};
use bevy::prelude::*;

/// All jersey controller-grid layouts, kept as one resource so
/// [`crate::terrain::index::AvianTerrainIndex`] stays within SystemParam limits.
#[derive(Resource, Debug, Clone, Default)]
pub struct StampControllerLayouts {
	pub plateau_low_pass: PlateauLowPassControllerLayout,
	pub plateau_high_pass: PlateauHighPassControllerLayout,
	pub massif_low_pass: MassifLowPassControllerLayout,
	pub massif_high_pass: MassifHighPassControllerLayout,
	pub canyon_low_pass: CanyonLowPassControllerLayout,
	pub canyon_high_pass: CanyonHighPassControllerLayout,
	pub pocket_water_low_pass: PocketWaterLowPassControllerLayout,
	pub pocket_water_high_pass: PocketWaterHighPassControllerLayout,
	pub rolling_low_pass: RollingLowPassControllerLayout,
	pub rolling_high_pass: RollingHighPassControllerLayout,
	pub valley_low_pass: ValleyLowPassControllerLayout,
	pub valley_high_pass: ValleyHighPassControllerLayout,
}
