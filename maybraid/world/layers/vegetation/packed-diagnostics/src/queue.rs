//! Correlate the main-world sample with actual native render membership.
use crate::visibility::VisibilitySample;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::core_3d::Opaque3d;
use bevy::pbr::{
	PreparedMaterial, RenderMaterialInstances, RenderMeshInstances, ShadowView,
	SpecializedMaterialPipelineCache,
};
use bevy::prelude::*;
use bevy::render::{
	erased_render_asset::ErasedRenderAssets,
	render_phase::ViewBinnedRenderPhases,
	render_resource::PipelineCache,
	sync_world::MainEntity,
	view::{ExtractedView, RenderVisibleEntities, RenderVisibleEntitiesClass},
};
use chico::packed::{GpuPackedInstances, PackedBatchInstance};
use std::collections::HashSet;

#[derive(Debug, Default)]
struct Stages {
	extracted: u64,
	mesh: u64,
	material: u64,
	prepared: u64,
	engine_visible: u64,
	specialized: u64,
	pipeline_ready: u64,
	in_phase: u64,
}
impl Stages {
	fn visible(class: Option<&RenderVisibleEntitiesClass>, main: MainEntity) -> bool {
		class.is_some_and(|class| {
			class
				.entities_cpu_culling
				.binary_search_by_key(&main, |(_, main)| *main)
				.is_ok() || class.entities_gpu_culling.contains_key(&main)
		})
	}
}
#[derive(Debug)]
#[allow(dead_code, reason = "Fields are emitted as bounded diagnostic evidence via Debug.")]
struct Example {
	main: u64,
	render: Option<u64>,
	extracted_phase: Option<u8>,
	gpu_component: Option<bool>,
	mesh: Option<bool>,
	should_batch: Option<bool>,
	material: Option<bool>,
	prepared: Option<bool>,
	engine_visible: Option<bool>,
	specialized: Option<bool>,
	pipeline_ready: Option<bool>,
	in_phase: bool,
}
#[derive(Debug)]
#[allow(dead_code, reason = "Fields are emitted as bounded diagnostic evidence via Debug.")]
struct ViewRow {
	view: u64,
	auxiliary: Option<u64>,
	subview: u32,
	class_available: bool,
	phase_available: bool,
	unbatchable_bins: usize,
	stages: Stages,
	example: Option<Example>,
}

pub(super) fn record(
	sample: Option<Res<VisibilitySample>>,
	proxies: Query<(Entity, &MainEntity, &PackedBatchInstance, Option<&GpuPackedInstances>)>,
	meshes: Option<Res<RenderMeshInstances>>,
	material_instances: Option<Res<RenderMaterialInstances>>,
	materials: Option<Res<ErasedRenderAssets<PreparedMaterial>>>,
	phases: Res<ViewBinnedRenderPhases<Opaque3d>>,
	views: Query<(
		&ExtractedView,
		Has<ShadowView>,
		Option<&RenderVisibleEntities>,
		Option<&RenderLayers>,
	)>,
	specialization: Option<Res<SpecializedMaterialPipelineCache>>,
	pipelines: Option<Res<PipelineCache>>,
	mut previous: Local<u64>,
	mut render_frame: Local<u64>,
) {
	*render_frame += 1;
	let Some(sample) = sample else { return };
	if sample.sequence == 0 || sample.sequence == *previous {
		return;
	}
	*previous = sample.sequence;
	let selected = sample
		.proxy
		.as_ref()
		.and_then(|proxy| Entity::try_from_bits(proxy.entity))
		.map(MainEntity::from);
	let mut ordered = views
		.iter()
		.filter(|(view, shadow, _, layers)| {
			!*shadow
				&& sample.camera == Some(view.retained_view_entity.main_entity.id().to_bits())
				&& layers.is_none_or(|layers| layers.intersects(&RenderLayers::default()))
		})
		.collect::<Vec<_>>();
	ordered.sort_by_key(|(view, _, _, _)| {
		(
			view.retained_view_entity.main_entity.id().to_bits(),
			view.retained_view_entity.auxiliary_entity.map(|e| e.id().to_bits()),
			view.retained_view_entity.subview_index,
		)
	});
	let mut rows = Vec::new();
	for (view, _, visible, _) in ordered.iter().take(8).copied() {
		let class = visible.and_then(|visible| visible.get::<Mesh3d>());
		let phase = phases.0.get(&view.retained_view_entity);
		let in_phase = phase
			.map(|phase| {
				phase
					.unbatchable_meshes
					.values()
					.flat_map(|bin| bin.entities.keys().copied())
					.collect::<HashSet<_>>()
			})
			.unwrap_or_default();
		let specialized = specialization
			.as_deref()
			.and_then(|cache| cache.get(&view.retained_view_entity));
		let mut stages = Stages::default();
		let mut example = None;
		for (render, main, batch, gpu) in &proxies {
			stages.extracted += 1;
			let mesh = meshes.as_deref().map(|meshes| meshes.mesh_asset_id(*main).is_some());
			let should_batch = meshes
				.as_deref()
				.and_then(|meshes| meshes.render_mesh_queue_data(*main))
				.map(|data| data.should_batch());
			let material = material_instances
				.as_deref()
				.and_then(|materials| materials.instances.get(main));
			let prepared = material.and_then(|material| {
				materials.as_deref().map(|materials| materials.get(material.asset_id).is_some())
			});
			let engine_visible = Stages::visible(class, *main);
			let pipeline = specialized.and_then(|cache| cache.get(main));
			let ready = pipeline.and_then(|pipeline| {
				pipelines
					.as_deref()
					.map(|pipelines| pipelines.get_render_pipeline(*pipeline).is_some())
			});
			stages.mesh += u64::from(mesh == Some(true));
			stages.material += u64::from(material.is_some());
			stages.prepared += u64::from(prepared == Some(true));
			stages.engine_visible += u64::from(engine_visible);
			stages.specialized += u64::from(pipeline.is_some());
			stages.pipeline_ready += u64::from(ready == Some(true));
			stages.in_phase += u64::from(in_phase.contains(main));
			if Some(*main) == selected {
				example = Some(Example {
					main: main.id().to_bits(),
					render: Some(render.to_bits()),
					extracted_phase: Some(batch.extracted_phase),
					gpu_component: Some(gpu.is_some()),
					mesh,
					should_batch,
					material: material_instances.as_ref().map(|_| material.is_some()),
					prepared,
					engine_visible: class.map(|_| engine_visible),
					specialized: specialization.as_ref().map(|_| pipeline.is_some()),
					pipeline_ready: ready,
					in_phase: in_phase.contains(main),
				});
			}
		}
		if example.is_none() {
			if let Some(main) = selected {
				let material = material_instances
					.as_deref()
					.and_then(|materials| materials.instances.get(&main));
				let pipeline = specialized.and_then(|cache| cache.get(&main));
				example = Some(Example {
					main: main.id().to_bits(),
					render: None,
					extracted_phase: None,
					gpu_component: None,
					mesh: meshes.as_deref().map(|meshes| meshes.mesh_asset_id(main).is_some()),
					should_batch: meshes
						.as_deref()
						.and_then(|meshes| meshes.render_mesh_queue_data(main))
						.map(|data| data.should_batch()),
					material: material_instances.as_ref().map(|_| material.is_some()),
					prepared: material.and_then(|material| {
						materials
							.as_deref()
							.map(|materials| materials.get(material.asset_id).is_some())
					}),
					engine_visible: class.map(|_| Stages::visible(class, main)),
					specialized: specialization.as_ref().map(|_| pipeline.is_some()),
					pipeline_ready: pipeline.and_then(|pipeline| {
						pipelines
							.as_deref()
							.map(|pipelines| pipelines.get_render_pipeline(*pipeline).is_some())
					}),
					in_phase: in_phase.contains(&main),
				});
			}
		}
		rows.push(ViewRow {
			view: view.retained_view_entity.main_entity.id().to_bits(),
			auxiliary: view.retained_view_entity.auxiliary_entity.map(|e| e.id().to_bits()),
			subview: view.retained_view_entity.subview_index,
			class_available: class.is_some(),
			phase_available: phase.is_some(),
			unbatchable_bins: phase.map_or(0, |phase| phase.unbatchable_meshes.len()),
			stages,
			example,
		});
	}
	bevy::log::tracing::trace!(target:"maybraid.packed.metrics",sample=sample.sequence,main_frame=sample.main_frame,render_invocation=*render_frame,selected_main=?selected.map(|main|main.id().to_bits()),extracted_proxies=proxies.iter().count(),mesh_resource=meshes.is_some(),material_resource=material_instances.is_some(),prepared_resource=materials.is_some(),specialization_resource=specialization.is_some(),pipeline_resource=pipelines.is_some(),views_total=ordered.len(),views_truncated=ordered.len()>8,views=?rows,scope="same extracted main sample; full MainEntity keys; native mesh/material/visible/specialized/unbatchable membership; no rendering changes","packed_queue_sample");
}
#[cfg(test)]
mod tests {
	use crate::queue::Stages;
	use bevy::{
		prelude::*,
		render::{sync_world::MainEntity, view::RenderVisibleEntitiesClass},
	};
	#[test]
	fn native_visibility_uses_main_identity_not_placeholder_render_entity() {
		let mut world = World::new();
		let first = MainEntity::from(world.spawn_empty().id());
		let second = MainEntity::from(world.spawn_empty().id());
		let mut class = RenderVisibleEntitiesClass::default();
		class.entities_cpu_culling.push((Entity::PLACEHOLDER, first));
		assert!(Stages::visible(Some(&class), first));
		assert!(!Stages::visible(Some(&class), second));
		class.entities_gpu_culling.insert(second, Entity::PLACEHOLDER);
		assert!(Stages::visible(Some(&class), second));
		assert!(!Stages::visible(None, first));
	}
}
