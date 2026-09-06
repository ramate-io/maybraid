//! Generate-only LOD for [`SelectedUrbanization`].
//!
//! Present, pad bake, and host spawn stay off this plugin
//! ([#720](https://github.com/ramate-io/maybraid/issues/720) step 3).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::{
	LodGenerateKeepRegion, LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems,
	LodViewer,
};

use crate::{
	SelectedUrbanization, UrbanizationGenerateBullseye, UrbanizationIndex, UrbanizationLodChan,
};

/// Registers [`SelectedUrbanization`] generate. Bullseyes stay disabled until
/// [`crate::install_urbanization_generate_stream`].
#[derive(Default)]
pub struct UrbanizationGenerationPlugin;

impl Plugin for UrbanizationGenerationPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<UrbanizationIndex>()
			.add_plugins(LodGenerateRegionPlugin::<
				UrbanizationGenerateBullseye,
				With<LodViewer>,
				UrbanizationLodChan,
			>::default())
			.add_plugins(LodGeneratePlugin::<
				SelectedUrbanization,
				UrbanizationIndex,
				UrbanizationLodChan,
				With<LodViewer>,
			>::default())
			.add_systems(
				Update,
				log_first_urbanization_generate_keep.after(LodGenerateSystems::Produce),
			);
	}
}

fn log_first_urbanization_generate_keep(
	keep: Res<LodGenerateKeepRegion<UrbanizationLodChan>>,
	mut logged: Local<bool>,
) {
	log_keep_arm_once("urbanization generate", keep.region, &mut logged);
}

fn log_keep_arm_once(label: &'static str, region: Option<Aabb3d>, logged: &mut bool) {
	let Some(region) = region else {
		return;
	};
	if *logged {
		return;
	}
	*logged = true;
	let edge_x = region.max.x - region.min.x;
	let edge_z = region.max.z - region.min.z;
	let half = edge_x.max(edge_z) * 0.5;
	info!(
		"{label} keep first arm min.xz=({:.1},{:.1}) max.xz=({:.1},{:.1}) edge_xz=({:.1},{:.1}) half={:.1}",
		region.min.x, region.min.z, region.max.x, region.max.z, edge_x, edge_z, half
	);
	if half > 7500.0 {
		warn!(
			"{label} keep half-extent {half:.0} m looks like terrain stream-edge, not the 1 km / 3 km rings"
		);
	}
}
