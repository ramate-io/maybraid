//! Opt-in resident compatible woody instances in the ordinary material render phases.
mod cache;
mod draw;
mod gpu;
mod instances;
mod kit;
pub(crate) mod lifecycle;
mod material;
mod telemetry;
mod view_policy;
pub use gpu::{GpuPackedInstances, PackedBatchInstance, PackedCounters};
pub use select::PackedViewSelection;
pub use telemetry::PackedSnapshot;
#[cfg(feature = "packed-diagnostics")]
pub use telemetry::{PackedAdmissionCount, PackedAdmissionSnapshot};
pub use view_policy::PackedViewPolicy;
mod mode;
mod render;
mod select;

use bevy::asset::embedded_asset;
use bevy::core_pipeline::{core_3d::Opaque3d, prepass::Opaque3dPrepass};
use bevy::pbr::Shadow;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponentPlugin;
use bevy::render::render_phase::AddRenderCommand;
use bevy::render::{Render, RenderApp, RenderSystems};
pub use mode::PackMode;

pub struct PackedGrovesPlugin;
impl Plugin for PackedGrovesPlugin {
	fn build(&self, app: &mut App) {
		if PackMode::current().is_off() {
			return;
		}
		embedded_asset!(app, "packed/packed_vertex.wesl");
		embedded_asset!(app, "packed/packed_prepass.wesl");
		app.insert_resource(PackMode::current())
			.insert_resource(
				cache::PackedGroveCache::with_budget(PackMode::budget_bytes_from_env()),
			)
			.init_resource::<kit::PackedKitMeshes>()
			.add_observer(lifecycle::hide_restoring_bag)
			.init_resource::<gpu::PackedCounters>()
			.add_plugins((
				MaterialPlugin::<material::PackedLeafMaterial>::default(),
				MaterialPlugin::<material::PackedStickMaterial>::default(),
				MaterialPlugin::<material::PackedShadowMaterial>::default(),
				ExtractComponentPlugin::<gpu::PackedBatchInstance>::default(),
			))
			.add_systems(
				Update,
				(
					kit::invalidate_changed_kits,
					kit::resolve_kit_meshes,
					lifecycle::sync_packed_tiles,
				)
					.chain(),
			);
		let counters = app.world().resource::<gpu::PackedCounters>().clone();
		if let Some(render) = app.get_sub_app_mut(RenderApp) {
			render
				.insert_resource(counters)
				.init_resource::<select::PackedViewSelection>()
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
					(gpu::prepare_packed_gpu, select::select_packed_views)
						.in_set(RenderSystems::PrepareResources),
				);
		}
	}
}
