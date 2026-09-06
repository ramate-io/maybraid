//! Generate and present LOD plugins for [`SelectedUrbanization`].
//!
//! Host spawn and pad bake stay off these plugins. Pass a spec to stamp
//! bullseyes in `Plugin::build`; use [`UrbanizationGenerationPlugin::plugins_only`]
//! / [`UrbanizationPresentationPlugin::plugins_only`] when a later system
//! enables them ([#720](https://github.com/ramate-io/maybraid/issues/720)).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::{
	LodGenerateKeepRegion, LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems,
	LodPresentKeepRegion, LodPresentRegionPlugin, LodPresentSystems, LodViewer,
};

use crate::{
	install_urbanization_generate_stream, install_urbanization_present_stream,
	SelectedUrbanization, UrbanizationGenerateBullseye, UrbanizationIndex, UrbanizationLodChan,
	UrbanizationPresentBullseye, UrbanizationStreamSpec,
};

/// Registers [`SelectedUrbanization`] generate.
///
/// [`Default`] stamps [`UrbanizationStreamSpec::default`] and enables the
/// generate bullseye. [`Self::plugins_only`] leaves bullseyes disabled.
pub struct UrbanizationGenerationPlugin {
	pub spec: Option<UrbanizationStreamSpec>,
}

impl Default for UrbanizationGenerationPlugin {
	fn default() -> Self {
		Self { spec: Some(UrbanizationStreamSpec::default()) }
	}
}

impl UrbanizationGenerationPlugin {
	/// LOD generate plugins without enabling the bullseye.
	pub fn plugins_only() -> Self {
		Self { spec: None }
	}
}

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
		if let Some(spec) = self.spec {
			install_urbanization_generate_stream(app, spec);
		}
	}
}

/// Present-keep for urbanization. Does not spawn building hosts or bake pads.
pub struct UrbanizationPresentationPlugin {
	pub spec: Option<UrbanizationStreamSpec>,
}

impl Default for UrbanizationPresentationPlugin {
	fn default() -> Self {
		Self { spec: Some(UrbanizationStreamSpec::default()) }
	}
}

impl UrbanizationPresentationPlugin {
	/// Present-keep plugins without enabling the bullseye.
	pub fn plugins_only() -> Self {
		Self { spec: None }
	}
}

impl Plugin for UrbanizationPresentationPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(LodPresentRegionPlugin::<
			UrbanizationPresentBullseye,
			With<LodViewer>,
			UrbanizationLodChan,
		>::default())
			.add_systems(
				Update,
				log_first_urbanization_present_keep.after(LodPresentSystems::Produce),
			);
		if let Some(spec) = self.spec {
			install_urbanization_present_stream(app, spec);
		}
	}
}

fn log_first_urbanization_generate_keep(
	keep: Res<LodGenerateKeepRegion<UrbanizationLodChan>>,
	mut logged: Local<bool>,
) {
	log_keep_arm_once("urbanization generate", keep.region, &mut logged);
}

fn log_first_urbanization_present_keep(
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	mut logged: Local<bool>,
) {
	log_keep_arm_once("urbanization present", keep.region, &mut logged);
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
