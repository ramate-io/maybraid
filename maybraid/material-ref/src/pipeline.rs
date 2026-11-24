//! Opt-in shader-input normalization for opaque procedural materials.
//!
//! This is a material-author contract, separate from MaterialRef identity or
//! fulfillment. Call only when the main fragment and its reachable imports do
//! not consume mesh UV0, UV0 textures, tangent-space normal maps or alpha masks.
//! The exact main-opaque label is admitted; all prepass/shadow/custom descriptors
//! remain unchanged. Uploaded vertex strides and remaining offsets are preserved.
use bevy::{render::render_resource::RenderPipelineDescriptor, shader::ShaderDefVal};

/// Shader-input policy for materials whose opaque main pass ignores mesh UV0.
///
/// This does not repack meshes or merge PSOs with different physical layouts.
/// Callers retain responsibility for their shader/alpha/normal-map contract.
pub struct OpaqueMaterialInputs;

impl OpaqueMaterialInputs {
	/// Omit unused UV0 definitions and main-pass shader-location2.
	///
	/// UV1, every other attribute, physical stride/offsets and pipeline state
	/// remain intact. Unknown, prepass and shadow labels are a no-op.
	pub fn omit_unused_uv0(descriptor: &mut RenderPipelineDescriptor) {
		// Main mesh shader UV0 is location2; prepass UV0 is location1 and UV1
		// is location2. Leave every prepass/shadow/custom descriptor unchanged.
		if descriptor.label.as_deref() != Some("opaque_mesh_pipeline") {
			return;
		}
		Self::shader_defs(&mut descriptor.vertex.shader_defs);
		if let Some(fragment) = &mut descriptor.fragment {
			Self::shader_defs(&mut fragment.shader_defs);
		}
		for buffer in &mut descriptor.vertex.buffers {
			buffer.attributes.retain(|attribute| attribute.shader_location != 2);
		}
		// Preserve array_stride and all remaining offsets: buffers are not repacked.
	}

	fn name(definition: &ShaderDefVal) -> &str {
		match definition {
			ShaderDefVal::Bool(name, _)
			| ShaderDefVal::Int(name, _)
			| ShaderDefVal::UInt(name, _) => name,
		}
	}

	fn shader_defs(definitions: &mut Vec<ShaderDefVal>) {
		let has_uv1 = definitions.iter().any(|definition| Self::name(definition) == "VERTEX_UVS_B");
		definitions.retain(|definition| match Self::name(definition) {
			"VERTEX_UVS_A" | "VERTEX_UVS_A_COMPRESSED" => false,
			"VERTEX_UVS" => has_uv1,
			_ => true,
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::{
		mesh::VertexBufferLayout,
		render::render_resource::{FragmentState, VertexAttribute, VertexFormat, VertexStepMode},
	};

	fn descriptor(with_uv0: bool) -> RenderPipelineDescriptor {
		let mut descriptor = RenderPipelineDescriptor {
			label: Some("opaque_mesh_pipeline".into()),
			fragment: Some(FragmentState::default()),
			..Default::default()
		};
		let mut definitions = vec![
			"MESH_PIPELINE".into(),
			"VERTEX_POSITIONS".into(),
			"VERTEX_NORMALS".into(),
			"VERTEX_COLORS".into(),
			"DISTANCE_FOG".into(),
			"DEPTH_PREPASS".into(),
		];
		if with_uv0 {
			definitions.extend(["VERTEX_UVS".into(), "VERTEX_UVS_A".into()]);
		}
		descriptor.vertex.shader_defs = definitions.clone();
		if let Some(fragment) = &mut descriptor.fragment {
			fragment.shader_defs = definitions;
		}
		let mut attributes = vec![
			VertexAttribute { format: VertexFormat::Float32x3, offset: 0, shader_location: 0 },
			VertexAttribute { format: VertexFormat::Float32x3, offset: 12, shader_location: 1 },
			VertexAttribute {
				format: VertexFormat::Float32x4,
				offset: if with_uv0 { 32 } else { 24 },
				shader_location: 5,
			},
		];
		if with_uv0 {
			attributes.insert(
				2,
				VertexAttribute { format: VertexFormat::Float32x2, offset: 24, shader_location: 2 },
			);
		}
		descriptor.vertex.buffers.push(VertexBufferLayout {
			array_stride: if with_uv0 { 48 } else { 40 },
			step_mode: VertexStepMode::Vertex,
			attributes,
		});
		descriptor
	}

	#[test]
	fn uv_variants_share_shader_definitions_without_changing_remaining_vertex_addresses() {
		let baseline = descriptor(false);
		let mut candidate = descriptor(true);
		let before = candidate.clone();
		OpaqueMaterialInputs::omit_unused_uv0(&mut candidate);
		assert_eq!(candidate.vertex.shader_defs, baseline.vertex.shader_defs);
		assert_eq!(
			candidate.fragment.as_ref().map(|fragment| &fragment.shader_defs),
			baseline.fragment.as_ref().map(|fragment| &fragment.shader_defs)
		);
		assert_eq!(candidate.vertex.buffers[0].array_stride, 48);
		assert_eq!(
			candidate.vertex.buffers[0]
				.attributes
				.iter()
				.find(|attribute| attribute.shader_location == 5)
				.map(|attribute| attribute.offset),
			Some(32)
		);
		let mut expected = before;
		expected.vertex.shader_defs = baseline.vertex.shader_defs;
		if let (Some(expected_fragment), Some(baseline_fragment)) =
			(&mut expected.fragment, baseline.fragment)
		{
			expected_fragment.shader_defs = baseline_fragment.shader_defs;
		}
		expected.vertex.buffers[0]
			.attributes
			.retain(|attribute| attribute.shader_location != 2);
		assert_eq!(candidate, expected);
		let once = candidate.clone();
		OpaqueMaterialInputs::omit_unused_uv0(&mut candidate);
		assert_eq!(candidate, once);
	}

	fn urban_descriptor(with_uv0: bool) -> RenderPipelineDescriptor {
		let mut descriptor = descriptor(with_uv0);
		descriptor
			.vertex
			.shader_defs
			.retain(|definition| OpaqueMaterialInputs::name(definition) != "VERTEX_COLORS");
		if let Some(fragment) = &mut descriptor.fragment {
			fragment
				.shader_defs
				.retain(|definition| OpaqueMaterialInputs::name(definition) != "VERTEX_COLORS");
		}
		descriptor.vertex.buffers[0]
			.attributes
			.retain(|attribute| attribute.shader_location != 5);
		descriptor.vertex.buffers[0].array_stride = if with_uv0 { 32 } else { 24 };
		descriptor
	}

	#[test]
	fn recorded_urban_108_and_129_layouts_share_shader_keys_with_distinct_strides() {
		let mut earlier = urban_descriptor(true);
		let mut movement = urban_descriptor(false);
		OpaqueMaterialInputs::omit_unused_uv0(&mut earlier);
		OpaqueMaterialInputs::omit_unused_uv0(&mut movement);
		assert_eq!(earlier.vertex.shader_defs, movement.vertex.shader_defs);
		assert_eq!(earlier.fragment, movement.fragment);
		assert_eq!(earlier.vertex.buffers[0].array_stride, 32);
		assert_eq!(movement.vertex.buffers[0].array_stride, 24);
		assert_eq!(earlier.vertex.buffers[0].attributes, movement.vertex.buffers[0].attributes);
		// Full descriptors retain exactly the recorded packed-stride difference.
		earlier.vertex.buffers[0].array_stride = 24;
		assert_eq!(earlier, movement);
	}

	#[test]
	fn uv1_definitions_and_location_are_preserved() {
		let mut candidate = descriptor(true);
		candidate
			.vertex
			.shader_defs
			.extend(["VERTEX_UVS_B".into(), "VERTEX_UVS_B_COMPRESSED".into()]);
		if let Some(fragment) = &mut candidate.fragment {
			fragment
				.shader_defs
				.extend(["VERTEX_UVS_B".into(), "VERTEX_UVS_B_COMPRESSED".into()]);
		}

		candidate.vertex.buffers[0].attributes.push(VertexAttribute {
			format: VertexFormat::Float32x2,
			offset: 48,
			shader_location: 3,
		});
		OpaqueMaterialInputs::omit_unused_uv0(&mut candidate);
		assert!(candidate.vertex.shader_defs.contains(&"VERTEX_UVS".into()));
		assert!(candidate.vertex.shader_defs.contains(&"VERTEX_UVS_B_COMPRESSED".into()));
		assert!(candidate.fragment.as_ref().is_some_and(|fragment| fragment
			.shader_defs
			.contains(&"VERTEX_UVS".into())
			&& fragment.shader_defs.contains(&"VERTEX_UVS_B_COMPRESSED".into())
			&& !fragment.shader_defs.contains(&"VERTEX_UVS_A".into())));

		assert!(candidate.vertex.buffers[0].attributes.iter().any(|attribute| attribute
			.shader_location
			== 3 && attribute
			.offset
			== 48));
	}

	#[test]
	fn prepass_shadow_and_unknown_descriptors_are_unchanged() {
		for label in [Some("prepass_pipeline"), Some("shadow_pipeline"), None] {
			let mut candidate = descriptor(true);
			candidate.label = label.map(Into::into);
			let before = candidate.clone();
			OpaqueMaterialInputs::omit_unused_uv0(&mut candidate);
			assert_eq!(candidate, before);
		}
	}
}
