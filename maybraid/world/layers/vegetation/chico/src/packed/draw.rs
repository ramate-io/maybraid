//! Direct resident instanced draws inside the existing material phases.

use crate::packed::cache::{ACTIVE, PREPARING, READY_MAIN, READY_PREPASS, READY_SHADOW};
use crate::packed::gpu::{GpuPackedInstances, PackedBatchInstance, PackedCounters};
use crate::packed::select::PackedViewSelection;
use bevy::core_pipeline::core_3d::Opaque3d;
use bevy::ecs::query::ROQueryItem;
use bevy::ecs::system::{lifetimeless::SRes, SystemParamItem};
use bevy::pbr::{
	SetMaterialBindGroup, SetMeshBindGroup, SetMeshViewBindGroup, SetMeshViewBindingArrayBindGroup,
	SetPrepassEmptyMaterialBindGroup, SetPrepassViewBindGroup, SetPrepassViewEmptyBindGroup,
	Shadow,
};
use bevy::render::mesh::{allocator::MeshAllocator, RenderMesh, RenderMeshBufferInfo};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_phase::{
	PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
};
use bevy::render::view::ExtractedView;
use std::any::TypeId;
use std::sync::atomic::Ordering;

pub type DrawPackedMaterial = (
	SetItemPipeline,
	SetMeshViewBindGroup<0>,
	SetMeshViewBindingArrayBindGroup<1>,
	SetMeshBindGroup<2>,
	SetMaterialBindGroup<3>,
	DrawPackedInstances,
);
pub type DrawPackedPrepass = (
	SetItemPipeline,
	SetPrepassViewBindGroup<0>,
	SetPrepassViewEmptyBindGroup<1>,
	SetMeshBindGroup<2>,
	SetMaterialBindGroup<3>,
	DrawPackedInstances,
);
pub type DrawPackedDepthOnly = (
	SetItemPipeline,
	SetPrepassViewBindGroup<0>,
	SetPrepassViewEmptyBindGroup<1>,
	SetMeshBindGroup<2>,
	SetPrepassEmptyMaterialBindGroup<3>,
	DrawPackedInstances,
);

pub struct DrawPackedInstances;

impl<P: PhaseItem + 'static> RenderCommand<P> for DrawPackedInstances {
	type Param = (
		SRes<RenderAssets<RenderMesh>>,
		SRes<MeshAllocator>,
		SRes<PackedViewSelection>,
		SRes<PackedCounters>,
	);
	type ViewQuery = &'static ExtractedView;
	type ItemQuery = (&'static PackedBatchInstance, &'static GpuPackedInstances);
	fn render<'w>(
		_item: &P,
		view: ROQueryItem<'w, '_, Self::ViewQuery>,
		instance: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
		(meshes, allocator, selection, counters): SystemParamItem<'w, '_, Self::Param>,
		pass: &mut TrackedRenderPass<'w>,
	) -> RenderCommandResult {
		let Some((instance, gpu)) = instance else { return RenderCommandResult::Skip };
		let meshes = meshes.into_inner();
		let allocator = allocator.into_inner();
		let Some(render_mesh) = meshes.get(instance.mesh) else { return RenderCommandResult::Skip };
		let Some(vertices) = allocator.mesh_vertex_slice(&instance.mesh) else {
			return RenderCommandResult::Skip;
		};
		let phase = if TypeId::of::<P>() == TypeId::of::<Shadow>() {
			READY_SHADOW
		} else if TypeId::of::<P>() == TypeId::of::<Opaque3d>() {
			READY_MAIN
		} else {
			READY_PREPASS
		};
		instance.signal.acknowledge(instance.ordinal, phase);
		if instance.extracted_phase == PREPARING {
			// Prefix commands prove this actual material/view pipeline is ready.
			// Do not double-render while ordinary scene roots still own the view.
			return RenderCommandResult::Success;
		}
		if instance.extracted_phase != ACTIVE
			|| selection.into_inner().levels.get(&(view.retained_view_entity, instance.cell))
				!= Some(&instance.level)
		{
			return RenderCommandResult::Skip;
		}
		pass.set_vertex_buffer(0, vertices.buffer.slice(..));
		pass.set_vertex_buffer(1, gpu.buffer.slice(..));
		match &render_mesh.buffer_info {
			RenderMeshBufferInfo::Indexed { count, index_format } => {
				let Some(indices) = allocator.mesh_index_slice(&instance.mesh) else {
					return RenderCommandResult::Skip;
				};
				pass.set_index_buffer(indices.buffer.slice(..), *index_format);
				pass.draw_indexed(
					indices.range.start..indices.range.start + *count,
					vertices.range.start as i32,
					0..instance.instances,
				);
			}
			RenderMeshBufferInfo::NonIndexed => pass.draw(vertices.range, 0..instance.instances),
		}
		let counters = counters.into_inner();
		match phase {
			READY_MAIN => {
				#[cfg(feature = "packed-diagnostics")]
				if bevy::log::tracing::enabled!(target:"maybraid.packed.metrics",bevy::log::tracing::Level::TRACE)
				{
					instance.diagnostic_main_draws.fetch_add(1, Ordering::Relaxed);
				}
				counters.0.issued_main_draws.fetch_add(1, Ordering::Relaxed);
				counters
					.0
					.issued_main_instances
					.fetch_add(u64::from(instance.instances), Ordering::Relaxed);
			}
			READY_SHADOW => {
				counters.0.issued_shadow_draws.fetch_add(1, Ordering::Relaxed);
			}
			_ => {
				counters.0.issued_prepass_draws.fetch_add(1, Ordering::Relaxed);
			}
		}
		RenderCommandResult::Success
	}
}
