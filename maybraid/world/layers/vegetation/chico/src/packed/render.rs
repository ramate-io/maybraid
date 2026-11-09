//! Install custom draw IDs before Bevy specializes these wrapper materials.
//! Prepared properties can already be shared: clone explicitly rather than
//! silently leaving a default DrawMesh path after Arc::get_mut fails.
use super::{
	draw::*,
	material::{PackedLeafMaterial, PackedShadowMaterial, PackedStickMaterial},
};
use bevy::core_pipeline::{core_3d::Opaque3d, prepass::Opaque3dPrepass};
use bevy::material::{labels::DrawFunctionLabel, MaterialProperties};
use bevy::pbr::{
	MainPassOpaqueDrawFunction, PreparedMaterial, PrepassOpaqueDepthOnlyDrawFunction,
	PrepassOpaqueDrawFunction, Shadow, ShadowsDepthOnlyDrawFunction, ShadowsDrawFunction,
};
use bevy::prelude::*;
use bevy::render::erased_render_asset::ErasedRenderAssets;
use bevy::render::render_phase::DrawFunctions;
use std::{any::TypeId, sync::Arc};

fn copy_properties(p: &MaterialProperties) -> MaterialProperties {
	MaterialProperties {
		render_method: p.render_method,
		alpha_mode: p.alpha_mode,
		mesh_pipeline_key_bits: p.mesh_pipeline_key_bits.clone(),
		depth_bias: p.depth_bias,
		reads_view_transmission_texture: p.reads_view_transmission_texture,
		render_phase_type: p.render_phase_type,
		material_layout: p.material_layout.clone(),
		draw_functions: p.draw_functions.clone(),
		shaders: p.shaders.clone(),
		bindless: p.bindless,
		base_specialize: p.base_specialize,
		prepass_specialize: p.prepass_specialize,
		user_specialize: p.user_specialize,
		material_key: p.material_key.clone(),
		shadows_enabled: p.shadows_enabled,
		prepass_enabled: p.prepass_enabled,
		oit_enabled: p.oit_enabled,
	}
}

pub fn install_packed_draw_functions(
	mut materials: ResMut<ErasedRenderAssets<PreparedMaterial>>,
	main: Res<DrawFunctions<Opaque3d>>,
	prepass: Res<DrawFunctions<Opaque3dPrepass>>,
	shadow: Res<DrawFunctions<Shadow>>,
) {
	if !materials.is_changed() {
		return;
	}
	let replacements = [
		(MainPassOpaqueDrawFunction.intern(), main.read().id::<DrawPackedMaterial>()),
		(PrepassOpaqueDrawFunction.intern(), prepass.read().id::<DrawPackedPrepass>()),
		(PrepassOpaqueDepthOnlyDrawFunction.intern(), prepass.read().id::<DrawPackedDepthOnly>()),
		(ShadowsDrawFunction.intern(), shadow.read().id::<DrawPackedPrepass>()),
		(ShadowsDepthOnlyDrawFunction.intern(), shadow.read().id::<DrawPackedDepthOnly>()),
	];
	for (id, material) in materials.iter_mut() {
		if ![
			TypeId::of::<PackedLeafMaterial>(),
			TypeId::of::<PackedStickMaterial>(),
			TypeId::of::<PackedShadowMaterial>(),
		]
		.contains(&id.type_id())
		{
			continue;
		}
		if replacements.iter().all(|(label, id)| {
			material.properties.draw_functions.iter().any(|entry| entry == &(*label, *id))
		}) {
			continue;
		}
		if Arc::get_mut(&mut material.properties).is_none() {
			material.properties = Arc::new(copy_properties(&material.properties));
		}
		if let Some(properties) = Arc::get_mut(&mut material.properties) {
			for (label, id) in replacements {
				if let Some(entry) =
					properties.draw_functions.iter_mut().find(|entry| entry.0 == label)
				{
					entry.1 = id;
				} else {
					properties.draw_functions.push((label, id));
				}
			}
		}
	}
}
