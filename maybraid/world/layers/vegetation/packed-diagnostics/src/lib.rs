//! Bounded structured trace metrics, identical observer plumbing in both modes.
//! Buffer counters report submitted payload/allocation bytes, never bus traffic.
use bevy::core_pipeline::core_3d::Opaque3d;
use bevy::prelude::*;
use bevy::render::render_phase::ViewBinnedRenderPhases;
use bevy::render::{Render, RenderApp, RenderSystems};
use chico::packed::{PackedBatchInstance, PackedCounters, PackedSnapshot};
use std::{
	sync::atomic::Ordering,
	time::{Duration, Instant},
};

pub struct PackedGroveDiagnosticsPlugin;
impl Plugin for PackedGroveDiagnosticsPlugin {
	fn build(&self, app: &mut App) {
		if !matches!(
			std::env::var("MAYBRAID_PACKED_GROVE_METRICS").as_deref(),
			Ok("1" | "true" | "on")
		) {
			return;
		}
		app.add_systems(PostUpdate, record_main_metrics);
		if let Some(render) = app.get_sub_app_mut(RenderApp) {
			render
				.init_resource::<PackedCounters>()
				.add_systems(Render, record_render_metrics.in_set(RenderSystems::Cleanup));
		}
	}
}
fn record_main_metrics(world: &mut World, mut previous: Local<Option<Instant>>) {
	let now = Instant::now();
	if previous.is_some_and(|last| now.duration_since(last) < Duration::from_secs(1)) {
		return;
	}
	*previous = Some(now);
	let snapshot = PackedSnapshot::read(world);
	let count = world.query_filtered::<Entity, With<Mesh3d>>().iter(world).count();
	bevy::log::tracing::trace!(target:"maybraid.packed.metrics",  global_mesh_entities=count,resident_packed_tiles=snapshot.resident_tiles,active_packed_tiles=snapshot.active_tiles,
  resident_packed_batches=snapshot.resident_batches, resident_packed_instances_all_lods=snapshot.resident_instances_all_lods,
  packed_source_ir_bytes=snapshot.source_ir_bytes,packed_encoded_cpu_bytes=snapshot.encoded_cpu_bytes,packed_resident_gpu_payload_bytes=snapshot.resident_gpu_payload_bytes,
  packed_cache_builds=snapshot.cache_builds,packed_budget_denials=snapshot.budget_denials,"packed_main_census");
}

pub fn record_render_metrics(
	phases: Res<ViewBinnedRenderPhases<Opaque3d>>,
	counters: Res<PackedCounters>,
	instances: Query<&PackedBatchInstance>,
	mut previous: Local<Option<Instant>>,
) {
	let now = Instant::now();
	if previous.is_some_and(|last| now.duration_since(last) < Duration::from_secs(1)) {
		return;
	}
	*previous = Some(now);
	// A multidraw batch-set is not one draw. Keep these separate from issued
	// custom draw-command counters; gpucapture inspects ordinary actual draws.
	let sets = phases.0.values().map(|phase| phase.multidrawable_meshes.len()).sum::<usize>();
	let bins = phases.0.values().map(|phase| phase.batchable_meshes.len()).sum::<usize>();
	let unbatched = phases.0.values().map(|phase| phase.unbatchable_meshes.len()).sum::<usize>();
	bevy::log::tracing::trace!(target:"maybraid.packed.metrics",  opaque_multidraw_sets=sets,opaque_batchable_bins=bins,opaque_unbatchable_bins=unbatched,
  extracted_proxy_batches=instances.iter().count(),
  submitted_payload_bytes=counters.0.submitted_payload_bytes.load(Ordering::Relaxed),
  allocated_payload_bytes=counters.0.allocated_payload_bytes.load(Ordering::Relaxed),
  upload_calls=counters.0.upload_calls.load(Ordering::Relaxed),
  issued_main_draws=counters.0.issued_main_draws.load(Ordering::Relaxed),
  issued_prepass_draws=counters.0.issued_prepass_draws.load(Ordering::Relaxed),
  issued_shadow_draws=counters.0.issued_shadow_draws.load(Ordering::Relaxed),
  issued_main_instances=counters.0.issued_main_instances.load(Ordering::Relaxed),"packed_render_census");
}
