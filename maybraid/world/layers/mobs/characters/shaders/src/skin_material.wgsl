//---------------------------------------------------------
// Bikanwarda / Epiphant skin: palette[0] hide + UV0 tattoo emission.
//
// Mapping 1 ([#1128](https://github.com/ramate-io/maybraid/issues/1128)):
// sample the authored mask on mesh UV0 when present. Body / ear / trunk
// GLBs have no TEXCOORD_0 yet; the head's UV0 is a collapsed pole. Those
// kinds use the documented provisional unwraps (see tattoos README).
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::{morph_position, morph_normal, morph_tangent},
    forward_io::Vertex,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct SkinMaterialUniform {
    colors: array<vec4<f32>, 8>,
    noise: vec4<f32>,
    scalars: array<vec4<f32>, 8>,
    rasters: array<array<vec4<f32>, 3>, 8>,
    kind: u32,
    flags: u32,
    _pad: vec2<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: SkinMaterialUniform;
@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var tattoo_mask: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var tattoo_sampler: sampler;

const KIND_BODY: u32 = 0u;
const KIND_HEAD: u32 = 1u;
const KIND_EAR: u32 = 2u;
const KIND_TRUNK: u32 = 3u;

// Bind-pose AABB of `characters/bodies/quadruped/epiphant.glb` (Z long axis).
const BODY_UNWRAP_Z_MIN: f32 = -0.9804133;
const BODY_UNWRAP_Z_SPAN: f32 = 2.0035005;
const EAR_UNWRAP_X_MIN: f32 = -0.29522276;
const EAR_UNWRAP_X_SPAN: f32 = 2.3301609;
const EAR_UNWRAP_Y_MIN: f32 = -1.6527351;
const EAR_UNWRAP_Y_SPAN: f32 = 2.9819443;
const TRUNK_UNWRAP_Y_MIN: f32 = -1.932509;
const TRUNK_UNWRAP_Y_SPAN: f32 = 2.4697547;
const HEAD_UNWRAP_Y_MIN: f32 = -0.9791473;
const HEAD_UNWRAP_Y_SPAN: f32 = 2.0418647;
const INV_TWO_PI: f32 = 0.15915494309189535;

struct SkinVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
    @location(8) local_pos: vec3<f32>,
}

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex, instance_index: u32) -> Vertex {
    var vertex = vertex_in;
    let first_vertex = mesh[instance_index].first_vertex_index;
    let vertex_index = vertex.index - first_vertex;
    let weight_count = bevy_pbr::morph::layer_count(instance_index);
    for (var i: u32 = 0u; i < weight_count; i++) {
        let weight = bevy_pbr::morph::weight_at(i, instance_index);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph_position(vertex_index, i, instance_index);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph_normal(vertex_index, i, instance_index);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * morph_tangent(vertex_index, i, instance_index), 0.0);
#endif
    }
    return vertex;
}
#endif

fn body_provisional_uv(local: vec3<f32>) -> vec2<f32> {
    // Cylindrical about +Z; seam on the belly (u = 0/1), spine at u = 0.5.
    let u = atan2(local.x, local.y) * INV_TWO_PI + 0.5;
    let v = saturate((local.z - BODY_UNWRAP_Z_MIN) / BODY_UNWRAP_Z_SPAN);
    return vec2<f32>(u, v);
}

fn ear_provisional_uv(local: vec3<f32>) -> vec2<f32> {
    let u = saturate((local.x - EAR_UNWRAP_X_MIN) / EAR_UNWRAP_X_SPAN);
    let v = saturate((local.y - EAR_UNWRAP_Y_MIN) / EAR_UNWRAP_Y_SPAN);
    return vec2<f32>(u, v);
}

fn trunk_provisional_uv(local: vec3<f32>) -> vec2<f32> {
    // Cylindrical about +Y (trunk hangs down); rings are constant v.
    let u = atan2(local.x, local.z) * INV_TWO_PI + 0.5;
    let v = saturate((local.y - TRUNK_UNWRAP_Y_MIN) / TRUNK_UNWRAP_Y_SPAN);
    return vec2<f32>(u, v);
}

fn head_provisional_uv(local: vec3<f32>) -> vec2<f32> {
    // Authored UV0 is a collapsed pole plus a tiny forehead island, so the
    // placeholder samples this cylindrical-about-Y unwrap (face at u = 0.5).
    let u = atan2(local.x, local.z) * INV_TWO_PI + 0.5;
    let v = saturate((local.y - HEAD_UNWRAP_Y_MIN) / HEAD_UNWRAP_Y_SPAN);
    return vec2<f32>(u, v);
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> SkinVertexOutput {
    var out: SkinVertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph, vertex_no_morph.instance_index);
#else
    var vertex = vertex_no_morph;
#endif

#ifdef SKINNED
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_no_morph.instance_index
    );
#else
    var world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index
    );
#endif
#endif

    out.local_pos = vertex.position;
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.position = position_world_to_clip(out.world_position.xyz);

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif
    return out;
}

fn tattoo_uv(mesh: SkinVertexOutput) -> vec2<f32> {
    if material.kind == KIND_HEAD {
        return head_provisional_uv(mesh.local_pos);
    }
#ifdef VERTEX_UVS_A
    return mesh.uv;
#else
    if material.kind == KIND_EAR {
        return ear_provisional_uv(mesh.local_pos);
    } else if material.kind == KIND_TRUNK {
        return trunk_provisional_uv(mesh.local_pos);
    }
    return body_provisional_uv(mesh.local_pos);
#endif
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: SkinVertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();
    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);
    let n = normalize(fns::prepare_world_normal(mesh.world_normal, true, is_front));

    let skin = material.colors[0].xyz;
    let glow = material.colors[1].xyz;
    let intensity = material.scalars[0].x;
    let pulse_rate = material.scalars[0].y;
    let pulse_amp = material.scalars[0].z;
    var pulse = 1.0;
    if pulse_rate > 1e-4 {
        pulse = 1.0 + pulse_amp * sin(globals.time * pulse_rate * 6.2831855);
    }

    let uv = tattoo_uv(mesh);
    let mask_s = textureSample(tattoo_mask, tattoo_sampler, uv);
    let mask = max(max(mask_s.r, mask_s.g), mask_s.b) * mask_s.a;
    let emissive = glow * mask * intensity * pulse;

    // Investigation overlay: 8×8 UV checker (stretch / pinch) + mask in red.
    if (material.flags & 1u) != 0u {
        let cx = step(0.5, fract(uv.x * 8.0));
        let cy = step(0.5, fract(uv.y * 8.0));
        let checker = abs(cx - cy);
        return vec4<f32>(mask * 0.85 + checker * 0.25, checker * 0.55, checker * 0.15 + mask * 0.15, 1.0);
    }

    pbr_input.material.base_color = vec4<f32>(skin, 1.0);
    pbr_input.material.perceptual_roughness = 0.72;
    pbr_input.material.metallic = 0.0;
    pbr_input.material.reflectance = vec3<f32>(0.10, 0.10, 0.10);
    pbr_input.material.flags = pbr_input.material.flags | STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT;
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit_color.rgb + emissive, 1.0), view.color_grading);
}
