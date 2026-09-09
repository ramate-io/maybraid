//---------------------------------------------------------
// Crozon face: designed blink squash + millimetre mouth idle.
// Kind: 0 eye, 1 mouth. Phase = globals.time + instance seed.
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

struct FaceMaterialUniform {
    colors: array<vec4<f32>, 8>,
    noise: vec4<f32>,
    scalars: array<vec4<f32>, 8>,
    rasters: array<array<vec4<f32>, 3>, 8>,
    kind: u32,
    flags: u32,
    _pad: vec2<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: FaceMaterialUniform;

const KIND_EYE: u32 = 0u;
const KIND_MOUTH: u32 = 1u;

struct FaceVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
    @location(8) local_pos: vec3<f32>,
    @location(9) blink: f32,
}

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn face_seed(instance_index: u32, world_from_local: mat4x4<f32>) -> f32 {
    let origin = world_from_local[3].xyz;
    return hash13(vec3<f32>(f32(instance_index) * 0.13, origin.x * 4.1, origin.z * 3.7));
}

fn face_smoothstep(t: f32) -> f32 {
    let u = saturate(t);
    return u * u * (3.0 - 2.0 * u);
}

fn blink_pulse(t: f32, start: f32, close: f32, hold: f32, open: f32) -> f32 {
    let u = t - start;
    if u <= 0.0 || u >= close + hold + open {
        return 0.0;
    }
    if u < close {
        return face_smoothstep(u / close);
    }
    if u < close + hold {
        return 1.0;
    }
    return 1.0 - face_smoothstep((u - close - hold) / open);
}

/// Designed 1D envelope. Do not replace with raw 4D noise.
fn blink_envelope(time: f32, seed: f32) -> f32 {
    let period = 3.4 + seed * 1.8;
    let t = fract((time + seed * 17.0) / period);
    let close = 0.016;
    let hold = 0.008;
    let open = 0.048;
    let first = blink_pulse(t, 0.0, close, hold, open);
    if seed > 0.62 {
        let second_start = close + hold + open + 0.018;
        return max(first, blink_pulse(t, second_start, close, hold, open));
    }
    return first;
}

fn mouth_idle(time: f32, seed: f32, local_pos: vec3<f32>) -> vec3<f32> {
    let idle = sin(time * 0.9 + seed * 5.0) * 0.0025
        + sin(time * 0.31 + seed * 3.2) * 0.0012;
    let lip = saturate(-local_pos.y * 6.0 + 0.15);
    return vec3<f32>(0.0, -idle * (0.35 + 0.65 * lip), 0.0);
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

@vertex
fn vertex(vertex_no_morph: Vertex) -> FaceVertexOutput {
    var out: FaceVertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph, vertex_no_morph.instance_index);
#else
    var vertex = vertex_no_morph;
#endif

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    let seed = face_seed(vertex_no_morph.instance_index, mesh_world_from_local);
    var blink = 0.0;
    if material.kind == KIND_EYE {
        blink = blink_envelope(globals.time, seed);
        let y_scale = mix(1.0, 0.08, blink);
        vertex.position.y *= y_scale;
#ifdef VERTEX_NORMALS
        vertex.normal.y *= mix(1.0, 4.0, blink);
#endif
    } else {
        vertex.position += mouth_idle(globals.time, seed, vertex.position);
    }
    out.blink = blink;

#ifdef SKINNED
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_no_morph.instance_index
    );
#else
    var world_from_local = mesh_world_from_local;
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

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index
    );
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif

    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: FaceVertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();
    let base = material.colors[0].xyz;
    var roughness = 0.55;
    var metallic = 0.02;
    var tint = base;

    if material.kind == KIND_EYE {
        roughness = mix(0.22, 0.55, mesh.blink);
        metallic = 0.08;
        // Closed lid reads darker than the sclera squash alone.
        tint = mix(base, base * vec3<f32>(0.35, 0.28, 0.26), mesh.blink);
    } else {
        roughness = 0.72;
    }

    pbr_input.material.base_color = vec4<f32>(tint, 1.0);
    pbr_input.material.perceptual_roughness = roughness;
    pbr_input.material.metallic = metallic;
    pbr_input.material.reflectance = vec3<f32>(0.18, 0.18, 0.18);
    pbr_input.material.flags = pbr_input.material.flags | STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT;

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    let prepared_normal = fns::prepare_world_normal(mesh.world_normal, true, is_front);
    let n = normalize(prepared_normal);
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit_color.rgb, 1.0), view.color_grading);
}
