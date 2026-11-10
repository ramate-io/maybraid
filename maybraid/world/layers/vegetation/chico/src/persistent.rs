//! Default-off, one-cluster selected-band persistent instancing experiment.
mod draw;
mod fixture;
mod gpu;
mod kit;
mod material;
mod payload;
mod render;
mod spawn;
#[cfg(test)]
mod tests;

pub use fixture::{FixtureMode, VegetationInstanceFixture};
#[cfg(feature = "persistent-vegetation-diagnostics")]
pub use gpu::PersistentCounters;
pub use spawn::{FixtureState, FixtureStatus};

use bevy::asset::embedded_asset;
use bevy::core_pipeline::{core_3d::Opaque3d, prepass::Opaque3dPrepass};
use bevy::pbr::Shadow;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponentPlugin;
use bevy::render::render_phase::AddRenderCommand;
use bevy::render::{Render, RenderApp, RenderSystems};
use lod::{lod_ref::LodRef, LodSceneLevel, SceneChunk};
use vegetation_components::{scene_children, CanopyShadowMode};
use vegetation_groves::grove::nest_flattened_plant_source;

use crate::ForestGroveTile;

pub struct PersistentVegetationPlugin;
impl Plugin for PersistentVegetationPlugin {
	fn build(&self, app: &mut App) {
		let fixture = VegetationInstanceFixture::from_env();
		app.insert_resource(fixture);
		if fixture.tile_min.is_none() {
			return;
		}
		embedded_asset!(app, "persistent/packed_vertex.wesl");
		embedded_asset!(app, "persistent/packed_prepass.wesl");
		app.init_resource::<kit::PackedKitMeshes>()
			.init_resource::<spawn::SharedMaterials>()
			.add_plugins((
				MaterialPlugin::<material::PackedLeafMaterial>::default(),
				MaterialPlugin::<material::PackedStickMaterial>::default(),
				MaterialPlugin::<material::PackedShadowMaterial>::default(),
				ExtractComponentPlugin::<gpu::PackedBatchInstance>::default(),
			))
			.add_systems(
				Update,
				(kit::resolve_kit_meshes, spawn::load_selected_band, spawn::fail_invalid_fixture)
					.chain(),
			);
		#[cfg(feature = "persistent-vegetation-diagnostics")]
		app.init_resource::<gpu::PersistentCounters>();
		#[cfg(feature = "persistent-vegetation-diagnostics")]
		let counters = app.world().resource::<gpu::PersistentCounters>().clone();
		if let Some(render) = app.get_sub_app_mut(RenderApp) {
			#[cfg(feature = "persistent-vegetation-diagnostics")]
			render.insert_resource(counters);
			render
				.init_resource::<gpu::PackedDrawEntities>()
				.add_render_command::<Opaque3d, draw::DrawPackedMaterial>()
				.add_render_command::<Opaque3dPrepass, draw::DrawPackedPrepass>()
				.add_render_command::<Opaque3dPrepass, draw::DrawPackedDepthOnly>()
				.add_render_command::<Shadow, draw::DrawPackedPrepass>()
				.add_render_command::<Shadow, draw::DrawPackedDepthOnly>()
				.add_systems(
					Render,
					render::install_packed_draw_functions
						.after(RenderSystems::PrepareAssets)
						.before(RenderSystems::Specialize),
				)
				.add_systems(
					Render,
					gpu::prepare_packed_gpu.in_set(RenderSystems::PrepareResources),
				);
		}
	}
}

/// A single root owns the selected High band and exactly one status marker.
/// Ordinary visuals are constructed directly; persistent visuals never construct them.
pub(crate) fn fixture_chunks(
	tile: &ForestGroveTile,
	lod_ref: &LodRef,
	mode: FixtureMode,
	shadows: CanopyShadowMode,
) -> SceneChunk {
	use bevy::ecs::template::template;
	use bevy::scene::prelude::bsn;
	let ForestGroveTile::Orchard(orchard) = tile else {
		let status =
			FixtureStatus::failed(mode, tile.scene_bounds(), "selected fixture is not Orchard");
		return SceneChunk::primitive(bsn! { ~{status} });
	};
	let band = match payload::SelectedBand::from_orchard(orchard, shadows) {
		Ok(band) => band,
		Err(reason) => {
			let status = FixtureStatus::failed(mode, tile.scene_bounds(), reason);
			return SceneChunk::primitive(bsn! { ~{status} });
		}
	};
	let status = FixtureStatus::pending(mode, tile.scene_bounds(), &band);
	let mut children: Vec<Box<dyn bevy::scene::prelude::Scene>> = Vec::new();
	match mode {
		FixtureMode::Ordinary => children.extend(
			tile.scene_chunks_with_level(lod_ref, LodSceneLevel::High)
				.into_primitives()
				.into_iter()
				.map(|(_, scene)| scene),
		),
		FixtureMode::Persistent => children.extend(orchard.placed_plants().map(|plant| {
			Box::new(nest_flattened_plant_source(plant)) as Box<dyn bevy::scene::prelude::Scene>
		})),
	}
	SceneChunk::weighted(
		band.source_plants as u32 * vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT,
		(
			scene_children(children),
			bsn! {
				~{status}
				LodSceneLevel::High
				template(move |_ctx| Ok(spawn::PendingBand(band.clone())))
			},
		),
	)
}
