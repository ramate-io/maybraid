//! Direct resident instanced draws inside the existing material phases.

use crate::packed::cache::{ACTIVE, PREPARING, READY_MAIN, READY_PREPASS, READY_SHADOW};
use crate::packed::gpu::{
	GpuPackedInstances, PackedBatchInstance, PackedCounters, PackedDrawEntities,
};
use crate::packed::select::PackedViewSelection;
use bevy::core_pipeline::core_3d::Opaque3d;
use bevy::ecs::query::ROQueryItem;
use bevy::ecs::system::{
	lifetimeless::{SQuery, SRes},
	SystemParamItem,
};
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
use bevy::render::sync_world::MainEntity;
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
		SRes<PackedDrawEntities>,
		SQuery<(&'static MainEntity, &'static PackedBatchInstance, &'static GpuPackedInstances)>,
	);
	type ViewQuery = &'static ExtractedView;
	// Bevy's ordinary opaque queue sets item.entity() to PLACEHOLDER.
	// Prefix commands use main_entity(); this final command must do so too.
	type ItemQuery = ();
	fn render<'w>(
		item: &P,
		view: ROQueryItem<'w, '_, Self::ViewQuery>,
		_item_query: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
		(meshes, allocator, selection, counters, draw_entities, batches): SystemParamItem<
			'w,
			'_,
			Self::Param,
		>,
		pass: &mut TrackedRenderPass<'w>,
	) -> RenderCommandResult {
		let phase = if TypeId::of::<P>() == TypeId::of::<Shadow>() {
			READY_SHADOW
		} else if TypeId::of::<P>() == TypeId::of::<Opaque3d>() {
			READY_MAIN
		} else {
			READY_PREPASS
		};
		let counters = counters.into_inner();
		#[cfg(feature = "packed-diagnostics")]
		let diagnostic = bevy::log::tracing::enabled!(target:"maybraid.packed.metrics",bevy::log::tracing::Level::TRACE);
		#[cfg(feature = "packed-diagnostics")]
		let phase_index = match phase {
			READY_MAIN => 0,
			READY_PREPASS => 1,
			_ => 2,
		};
		#[cfg(feature = "packed-diagnostics")]
		if diagnostic {
			counters.0.draw_callbacks[phase_index].fetch_add(1, Ordering::Relaxed);
		}
		let key = item.main_entity();
		let Some(entity) = draw_entities.into_inner().get(key) else {
			#[cfg(feature = "packed-diagnostics")]
			if diagnostic {
				counters.0.draw_lookup_misses[phase_index].fetch_add(1, Ordering::Relaxed);
			}
			return RenderCommandResult::Skip;
		};
		let Ok((main, instance, gpu)) = batches.get_inner(entity) else {
			#[cfg(feature = "packed-diagnostics")]
			if diagnostic {
				counters.0.draw_lookup_misses[phase_index].fetch_add(1, Ordering::Relaxed);
			}
			return RenderCommandResult::Skip;
		};
		if *main != key {
			#[cfg(feature = "packed-diagnostics")]
			if diagnostic {
				counters.0.draw_lookup_misses[phase_index].fetch_add(1, Ordering::Relaxed);
			}
			return RenderCommandResult::Skip;
		}
		let meshes = meshes.into_inner();
		let allocator = allocator.into_inner();
		let (Some(render_mesh), Some(vertices)) =
			(meshes.get(instance.mesh), allocator.mesh_vertex_slice(&instance.mesh))
		else {
			#[cfg(feature = "packed-diagnostics")]
			if diagnostic {
				counters.0.draw_mesh_misses[phase_index].fetch_add(1, Ordering::Relaxed);
			}
			return RenderCommandResult::Skip;
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::packed::cache::{TilePayload, TileSignal, READY_BUFFER, RETIRED};
	use bevy::core_pipeline::{
		core_3d::{Opaque3dBatchSetKey, Opaque3dBinKey},
		prepass::{Opaque3dPrepass, OpaqueNoLightmap3dBatchSetKey, OpaqueNoLightmap3dBinKey},
	};
	use bevy::material::labels::DrawFunctionId;
	use bevy::pbr::{ShadowBatchSetKey, ShadowBinKey};
	use bevy::prelude::*;
	use bevy::render::{
		mesh::allocator::MeshSlabs,
		render_phase::{BinnedPhaseItem, PhaseItemExtraIndex},
		render_resource::CachedRenderPipelineId,
	};
	use std::sync::Arc;
	fn batch(main: Entity) -> PackedBatchInstance {
		let payload = TilePayload {
			#[cfg(feature = "packed-diagnostics")]
			diagnostic_family: "Orchard",
			revision: 1,
			proxy_shadows: false,
			bounds: bevy::shape::Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE),
			center: Vec3::ZERO,
			radius: 1.0,
			structural_lod: vegetation_components::StructuralLod::default(),
			plant_count: 0,
			lods: std::array::from_fn(|_| Arc::from([])),
			cpu_payload_bytes: 0,
			gpu_payload_bytes: 0,
		};
		PackedBatchInstance {
			cell: main,
			mesh: Handle::<Mesh>::default().id(),
			payload: Arc::new(payload),
			signal: Arc::new(TileSignal::new([READY_MAIN | READY_BUFFER])),
			extracted_phase: PREPARING,
			ordinal: 0,
			level: lod::LodSceneLevel::High,
			bytes: Arc::from([]),
			instances: 1,
			shadow_only: false,
			#[cfg(feature = "packed-diagnostics")]
			diagnostic_main_draws: Arc::new(std::sync::atomic::AtomicU64::new(0)),
		}
	}
	fn opaque(main: MainEntity) -> Opaque3d {
		Opaque3d::new(
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
		)
	}
	#[test]
	fn opaque_placeholder_resolves_exact_main_identity_and_shared_passes() -> anyhow::Result<()> {
		let mut main_world = World::new();
		for _ in 0..64 {
			main_world.spawn_empty();
		}
		let main = MainEntity::from(main_world.spawn_empty().id());
		let mut render_world = World::new();
		let render = render_world.spawn((main, batch(main.id()))).id();
		assert_ne!(render, main.id());
		let mut entities = PackedDrawEntities::default();
		entities.begin_frame();
		entities.insert_live(main, render, PREPARING);
		let item = opaque(main);
		assert_eq!(item.entity(), Entity::PLACEHOLDER);
		let mut query = render_world.query::<(&MainEntity, &PackedBatchInstance)>();
		assert!(
			query.get(&render_world, item.entity()).is_err(),
			"old ItemQuery path must fail on Bevy's opaque placeholder"
		);
		let resolved = entities
			.get(item.main_entity())
			.ok_or_else(|| anyhow::anyhow!("opaque main identity missing"))?;
		let (found, batch) = query
			.get(&render_world, resolved)
			.map_err(|error| anyhow::anyhow!("actual payload query failed: {error}"))?;
		assert_eq!(*found, item.main_entity());
		assert_eq!(batch.cell, main.id());
		let prepass = Opaque3dPrepass::new(
			OpaqueNoLightmap3dBatchSetKey {
				pipeline: CachedRenderPipelineId::INVALID,
				draw_function: DrawFunctionId(0),
				material_bind_group_index: None,
				slabs: MeshSlabs::default(),
			},
			OpaqueNoLightmap3dBinKey { asset_id: batch.mesh.untyped() },
			(render, main),
			0..1,
			PhaseItemExtraIndex::None,
		);
		let shadow = Shadow::new(
			ShadowBatchSetKey {
				pipeline: CachedRenderPipelineId::INVALID,
				draw_function: DrawFunctionId(0),
				material_bind_group_index: None,
				slabs: MeshSlabs::default(),
			},
			ShadowBinKey { asset_id: batch.mesh.untyped() },
			(render, main),
			0..1,
			PhaseItemExtraIndex::None,
		);
		assert_eq!(entities.get(prepass.main_entity()), Some(render));
		assert_eq!(entities.get(shadow.main_entity()), Some(render));
		let mut complete =
			render_world.query::<(&MainEntity, &PackedBatchInstance, &GpuPackedInstances)>();
		assert!(
			complete.get(&render_world, resolved).is_err(),
			"missing GPU payload must safely fail the new draw query"
		);
		Ok(())
	}
	#[test]
	fn empty_retired_removed_and_reused_identity_cannot_reuse_old_draw_mapping() {
		let mut main_world = World::new();
		let old = main_world.spawn_empty().id();
		main_world.despawn(old);
		let replacement = main_world.spawn_empty().id();
		assert_ne!(old, replacement);
		let mut render_world = World::new();
		let render = render_world.spawn((MainEntity::from(old), batch(old))).id();
		let mut entities = PackedDrawEntities::default();
		entities.begin_frame();
		entities.insert_live(old.into(), render, ACTIVE);
		assert_eq!(entities.get(old.into()), Some(render));
		assert!(entities.get(replacement.into()).is_none());
		render_world.despawn(render);
		let mut query = render_world.query::<(&MainEntity, &PackedBatchInstance)>();
		assert!(query.get(&render_world, render).is_err());
		entities.begin_frame();
		assert!(
			entities.get(old.into()).is_none(),
			"empty prepare frames must clear all previous entries"
		);
		entities.insert_live(old.into(), render, RETIRED);
		assert!(entities.get(old.into()).is_none());
	}
}
