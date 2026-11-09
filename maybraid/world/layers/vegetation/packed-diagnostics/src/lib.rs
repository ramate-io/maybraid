//! Bounded structured trace metrics, identical observer plumbing in both modes.
//! Buffer counters report submitted payload/allocation bytes, never bus traffic.
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::core_3d::Opaque3d;
use bevy::pbr::ShadowView;
use bevy::prelude::*;
use bevy::render::render_phase::ViewBinnedRenderPhases;
use bevy::render::view::ExtractedView;
use bevy::render::{Render, RenderApp, RenderSystems};
use chico::packed::{
	GpuPackedInstances, PackedBatchInstance, PackedCounters, PackedSnapshot, PackedViewSelection,
};
use std::{
	collections::BTreeSet,
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

fn record_render_metrics(
	phases: Res<ViewBinnedRenderPhases<Opaque3d>>,
	counters: Res<PackedCounters>,
	instances: Query<(&PackedBatchInstance, Option<&GpuPackedInstances>)>,
	views: Query<(&ExtractedView, Has<ShadowView>, Option<&RenderLayers>)>,
	selection: Option<Res<PackedViewSelection>>,
	mut detail: Local<CoverageSampler>,
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
	detail.emit(&instances, &views, selection.as_deref());
}

const MAX_VIEWS: usize = 8;
const MAX_SAMPLE_BUFFERS: usize = 64;
const LOD_NAMES: [&str; 4] = ["high", "medium", "low", "ultra_low"];

/// Sampling is bounded by views/coarse batches, never by vegetation instances.
/// Once chosen, the tile stays pinned for this launch even when off-camera.
/// A missing/nonresident tile is emitted explicitly; it is not a zero count.
#[derive(Default)]
struct CoverageSampler {
	sequence: u64,
	target: Option<u64>,
	initialized: bool,
}
#[derive(Default)]
struct LodCoverage {
	batches: u64,
	instances: u64,
	buffers: Vec<(usize, u32, usize)>,
	buffer_count: usize,
	main_draws: u64,
}
impl CoverageSampler {
	fn emit(
		&mut self,
		instances: &Query<(&PackedBatchInstance, Option<&GpuPackedInstances>)>,
		views: &Query<(&ExtractedView, Has<ShadowView>, Option<&RenderLayers>)>,
		selection: Option<&PackedViewSelection>,
	) {
		self.sequence += 1;
		if !self.initialized {
			self.initialized = true;
			self.target = std::env::var("MAYBRAID_PACKED_GROVE_DIAGNOSTIC_TILE")
				.ok()
				.and_then(|s| s.parse().ok());
		}
		let mut ordered = views
			.iter()
			.filter(|(_, shadow, layers)| {
				*shadow || layers.is_none_or(|layers| layers.intersects(&RenderLayers::default()))
			})
			.map(|(view, shadow, _)| (view, shadow))
			.collect::<Vec<_>>();
		ordered.sort_by_key(|(view, _)| {
			(
				view.retained_view_entity.main_entity.id().to_bits(),
				view.retained_view_entity.auxiliary_entity.map(|entity| entity.id().to_bits()),
				view.retained_view_entity.subview_index,
			)
		});
		if self.target.is_none() {
			let mut nearest: Option<(f32, u64)> = None;
			for (view, shadow) in &ordered {
				if *shadow {
					continue;
				}
				for (batch, _) in instances.iter() {
					if !batch.is_active()
						|| batch.shadow_only
						|| batch.diagnostic_main_draws.load(Ordering::Relaxed) == 0
					{
						continue;
					}
					if !selection.is_some_and(|selection| {
						selection.levels.get(&(view.retained_view_entity, batch.cell))
							== Some(&batch.level)
					}) {
						continue;
					}
					let candidate = (
						view.world_from_view.translation().distance_squared(batch.payload.center),
						batch.cell.to_bits(),
					);
					if nearest.is_none_or(|old| {
						candidate.0.total_cmp(&old.0).then(candidate.1.cmp(&old.1)).is_lt()
					}) {
						nearest = Some(candidate);
					}
				}
			}
			self.target = nearest.map(|(_, tile)| tile);
		}
		let mut sample = std::array::from_fn::<_, 4, _>(|_| LodCoverage::default());
		let mut tile = None;
		for (batch, gpu) in
			instances.iter().filter(|(batch, _)| Some(batch.cell.to_bits()) == self.target)
		{
			tile = Some(batch);
			if batch.shadow_only {
				continue;
			}
			let lod = &mut sample[batch.lod_index()];
			lod.batches += 1;
			lod.instances += u64::from(batch.instances);
			lod.main_draws += batch.diagnostic_main_draws.load(Ordering::Relaxed);
			if let Some(gpu) = gpu {
				lod.record_buffer(
					batch.ordinal,
					std::num::NonZeroU32::from(gpu.buffer.id()).get(),
					batch.bytes.len(),
				);
			}
		}
		for lod in &mut sample {
			lod.buffers.sort_unstable();
		}
		for (view, shadow) in ordered.iter().take(MAX_VIEWS).copied() {
			let mut counts = [[0_u64; 2]; 4];
			let mut cells = BTreeSet::new();
			for (batch, _) in instances
				.iter()
				.filter(|(batch, _)| batch.is_active() && batch.shadow_only == shadow)
			{
				if selection.is_some_and(|selection| {
					selection.levels.get(&(view.retained_view_entity, batch.cell))
						== Some(&batch.level)
				}) {
					counts[batch.lod_index()][0] += 1;
					counts[batch.lod_index()][1] += u64::from(batch.instances);
					cells.insert(batch.cell.to_bits());
				}
			}
			bevy::log::tracing::trace!(target:"maybraid.packed.metrics",sample=self.sequence,
    view=view.retained_view_entity.main_entity.id().to_bits(),auxiliary=?view.retained_view_entity.auxiliary_entity.map(|entity|entity.id().to_bits()),
    subview=view.retained_view_entity.subview_index,shadow,candidate_tiles=cells.len(),
    candidate_high_batches=counts[0][0],candidate_high_instances=counts[0][1],candidate_medium_batches=counts[1][0],candidate_medium_instances=counts[1][1],
    candidate_low_batches=counts[2][0],candidate_low_instances=counts[2][1],candidate_ultra_low_batches=counts[3][0],candidate_ultra_low_instances=counts[3][1],scope="cpu_frustum_distance_candidates","packed_view_lod_candidates");
			if shadow {
				continue;
			}
			if let Some(tile) = tile {
				let selected = selection.and_then(|selection| {
					selection.levels.get(&(view.retained_view_entity, tile.cell))
				});
				for (index, lod) in sample.iter().enumerate() {
					bevy::log::tracing::trace!(target:"maybraid.packed.metrics",sample=self.sequence,
      view=view.retained_view_entity.main_entity.id().to_bits(),tile=tile.cell.to_bits(),revision=tile.payload.revision,
      center_x=tile.payload.center.x,center_y=tile.payload.center.y,center_z=tile.payload.center.z,
      min=?tile.payload.bounds.min,max=?tile.payload.bounds.max,phase=tile.extracted_phase,
      lod=LOD_NAMES[index],candidate_selected=selected.is_some_and(|level|instances.iter().any(|(batch,_)|batch.cell==tile.cell&&batch.level==*level&&batch.lod_index()==index)),
      resident_batches=lod.batches,resident_instances=lod.instances,gpu_buffers=lod.buffer_count,
      buffer_ids=?lod.buffers,buffers_truncated=lod.buffer_count>lod.buffers.len(),issued_main_draws_all_views_cumulative=lod.main_draws,"packed_tile_lod");
				}
			}
		}
		bevy::log::tracing::trace!(target:"maybraid.packed.metrics",sample=self.sequence,target=?self.target,
   target_resident=tile.is_some(),views_total=ordered.len(),views_emitted=ordered.len().min(MAX_VIEWS),
   views_truncated=ordered.len()>MAX_VIEWS,selection_available=selection.is_some(),"packed_coverage_complete");
	}
}

impl LodCoverage {
	fn record_buffer(&mut self, ordinal: usize, id: u32, bytes: usize) {
		self.buffer_count += 1;
		let index = self.buffers.partition_point(|entry| entry.0 < ordinal);
		self.buffers.insert(index, (ordinal, id, bytes));
		if self.buffers.len() > MAX_SAMPLE_BUFFERS {
			self.buffers.pop();
		}
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn truncated_buffer_sample_is_stable_across_query_order() {
		let mut forward = LodCoverage::default();
		let mut reverse = LodCoverage::default();
		for ordinal in 0..100 {
			forward.record_buffer(ordinal, ordinal as u32 + 1, 176);
		}
		for ordinal in (0..100).rev() {
			reverse.record_buffer(ordinal, ordinal as u32 + 1, 176);
		}
		assert_eq!(forward.buffers, reverse.buffers);
		assert_eq!(forward.buffer_count, 100);
		assert_eq!(forward.buffers.len(), MAX_SAMPLE_BUFFERS);
		assert_eq!(forward.buffers.last().map(|entry| entry.0), Some(MAX_SAMPLE_BUFFERS - 1));
	}
}

/// Export-format fixture only: real diagnostic systems on an empty headless
/// world. No window, render device, vegetation coverage or GPU result is implied.
#[cfg(feature = "fixture")]
pub fn headless_fixture_once() -> Result<(), bevy::ecs::system::RunSystemError> {
	use bevy::ecs::system::RunSystemOnce;
	let mut world = World::new();
	world.init_resource::<ViewBinnedRenderPhases<Opaque3d>>();
	world.init_resource::<PackedCounters>();
	world.run_system_once(record_main_metrics)?;
	world.run_system_once(record_render_metrics)?;
	Ok(())
}
