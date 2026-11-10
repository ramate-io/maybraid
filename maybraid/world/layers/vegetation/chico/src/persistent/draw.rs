//! Direct resident instanced draws inside the existing material phases.

#[cfg(feature = "persistent-vegetation-diagnostics")]
use crate::persistent::gpu::PersistentCounters;
use crate::persistent::gpu::{GpuPackedInstances, PackedBatchInstance, PackedDrawEntities};
#[cfg(feature = "persistent-vegetation-diagnostics")]
use bevy::core_pipeline::core_3d::Opaque3d;
use bevy::ecs::query::ROQueryItem;
use bevy::ecs::system::{
	SystemParamItem,
	lifetimeless::{SQuery, SRes},
};
#[cfg(feature = "persistent-vegetation-diagnostics")]
use bevy::pbr::Shadow;
use bevy::pbr::{
	SetMaterialBindGroup, SetMeshBindGroup, SetMeshViewBindGroup, SetMeshViewBindingArrayBindGroup,
	SetPrepassEmptyMaterialBindGroup, SetPrepassViewBindGroup, SetPrepassViewEmptyBindGroup,
};
use bevy::render::mesh::{RenderMesh, RenderMeshBufferInfo, allocator::MeshAllocator};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_phase::{
	PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
};
use bevy::render::sync_world::MainEntity;
use bevy::render::view::ExtractedView;
#[cfg(feature = "persistent-vegetation-diagnostics")]
use std::any::TypeId;
#[cfg(feature = "persistent-vegetation-diagnostics")]
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
#[cfg(feature = "persistent-vegetation-diagnostics")]
type CounterParam = SRes<PersistentCounters>;
#[cfg(not(feature = "persistent-vegetation-diagnostics"))]
type CounterParam = ();

impl<P: PhaseItem + 'static> RenderCommand<P> for DrawPackedInstances {
	type Param = (
		SRes<RenderAssets<RenderMesh>>,
		SRes<MeshAllocator>,
		CounterParam,
		SRes<PackedDrawEntities>,
		SQuery<(&'static MainEntity, &'static PackedBatchInstance, &'static GpuPackedInstances)>,
	);
	type ViewQuery = &'static ExtractedView;
	// Bevy's ordinary opaque queue sets item.entity() to PLACEHOLDER.
	// Prefix commands use main_entity(); this final command must do so too.
	type ItemQuery = ();
	fn render<'w>(
		item: &P,
		_view: ROQueryItem<'w, '_, Self::ViewQuery>,
		_item_query: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
		(meshes, allocator, _counters, draw_entities, batches): SystemParamItem<
			'w,
			'_,
			Self::Param,
		>,
		pass: &mut TrackedRenderPass<'w>,
	) -> RenderCommandResult {
		#[cfg(feature = "persistent-vegetation-diagnostics")]
		let phase = if TypeId::of::<P>() == TypeId::of::<Shadow>() {
			2
		} else if TypeId::of::<P>() == TypeId::of::<Opaque3d>() {
			0
		} else {
			1
		};
		#[cfg(feature = "persistent-vegetation-diagnostics")]
		let counters = _counters.into_inner();
		let key = item.main_entity();
		let Some(entity) = draw_entities.into_inner().get(key) else {
			return RenderCommandResult::Skip;
		};
		let Ok((main, instance, gpu)) = batches.get_inner(entity) else {
			return RenderCommandResult::Skip;
		};
		if *main != key {
			return RenderCommandResult::Skip;
		}
		let meshes = meshes.into_inner();
		let allocator = allocator.into_inner();
		let (Some(render_mesh), Some(vertices)) =
			(meshes.get(instance.mesh), allocator.mesh_vertex_slice(&instance.mesh))
		else {
			#[cfg(feature = "persistent-vegetation-diagnostics")]
			counters.0.mesh_misses.fetch_add(1, Ordering::Relaxed);
			return RenderCommandResult::Skip;
		};

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
		#[cfg(feature = "persistent-vegetation-diagnostics")]
		match phase {
			0 => {
				counters.0.issued_main_draws.fetch_add(1, Ordering::Relaxed);
				counters
					.0
					.issued_main_instances
					.fetch_add(u64::from(instance.instances), Ordering::Relaxed);
			}
			2 => {
				counters.0.issued_shadow_draws.fetch_add(1, Ordering::Relaxed);
			}
			_ => {
				counters.0.issued_prepass_draws.fetch_add(1, Ordering::Relaxed);
			}
		}
		RenderCommandResult::Success
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::core_pipeline::core_3d::{Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey};
	use bevy::material::labels::DrawFunctionId;
	use bevy::prelude::*;
	use bevy::render::{
		mesh::allocator::MeshSlabs,
		render_phase::{BinnedPhaseItem, PhaseItemExtraIndex},
		render_resource::CachedRenderPipelineId,
	};

	#[test]
	fn opaque_placeholder_uses_canonical_main_identity_and_cannot_reuse_removed_entities()
	-> anyhow::Result<()> {
		let mut main_world = World::new();
		for _ in 0..64 {
			main_world.spawn_empty();
		}
		let main = MainEntity::from(main_world.spawn_empty().id());
		let mut render_world = World::new();
		let render = render_world.spawn(main).id();
		assert_ne!(render, main.id());
		let item = Opaque3d::new(
			Opaque3dBatchSetKey {
				pipeline: CachedRenderPipelineId::INVALID,
				draw_function: DrawFunctionId(0),
				material_bind_group_index: None,
				slabs: MeshSlabs::default(),
				lightmap_slab: None,
			},
			Opaque3dBinKey { asset_id: Handle::<Mesh>::default().id().untyped() },
			(Entity::PLACEHOLDER, main),
			0..1,
			PhaseItemExtraIndex::None,
		);
		assert_eq!(item.entity(), Entity::PLACEHOLDER);
		let mut mapping = PackedDrawEntities::default();
		mapping.0.insert(main, render);
		let resolved = mapping
			.get(item.main_entity())
			.ok_or_else(|| anyhow::anyhow!("canonical identity absent"))?;
		assert_eq!(render_world.get::<MainEntity>(resolved), Some(&main));
		render_world.despawn(render);
		mapping.0.clear();
		assert!(mapping.get(main).is_none());
		main_world.despawn(main.id());
		let replacement = MainEntity::from(main_world.spawn_empty().id());
		assert_ne!(main, replacement);
		assert!(mapping.get(replacement).is_none());
		Ok(())
	}
}
