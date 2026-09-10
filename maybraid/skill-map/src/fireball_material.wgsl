//---------------------------------------------------------
// Fireball: default-layout VertexOutput + aft pull along object -Y.
// Flight is object +Y. Color fill unchanged; no rim discard yet.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> base_color: vec4<f32>;

// seed, speed, spawn_time, max_age
@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> displace: vec4<f32>;

const FIREBALL_RADIUS: f32 = 1.18;
const TAIL_START: f32 = 0.35;
const TAIL_END: f32 = 4.6;

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33, 33.33, 33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn value_noise_3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (vec3<f32>(3.0, 3.0, 3.0) - 2.0 * f0);

    let n000 = hash13(i + vec3<f32>(0.0, 0.0, 0.0));
    let n100 = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash13(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash13(i + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = hash13(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash13(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash13(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash13(i + vec3<f32>(1.0, 1.0, 1.0));

    let nx00 = mix(n000, n100, f.x);
    let nx10 = mix(n010, n110, f.x);
    let nx01 = mix(n001, n101, f.x);
    let nx11 = mix(n011, n111, f.x);
    return mix(mix(nx00, nx10, f.y), mix(nx01, nx11, f.y), f.z);
}

fn phase() -> f32 {
    return globals.time + displace.x * 0.0013;
}

fn life() -> f32 {
    let age = max(globals.time - displace.z, 0.0);
    return saturate(age / max(displace.w, 1e-3));
}

fn tail_length() -> f32 {
    let u = life();
    let grow = u * (2.0 - u);
    return mix(TAIL_START, TAIL_END, grow);
}

fn displace_local(local: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let t = phase();
    let seed = displace.x * 0.01;
    let boil = value_noise_3d(local * 2.2 + vec3<f32>(seed, t * 1.7, seed * 2.0));
    var p = local + normal * ((boil - 0.5) * 0.16);

    // Keep the leading hemisphere (object +Y) ball-like.
    if p.y > 0.0 {
        let r = length(p);
        if r > 1e-4 {
            p = mix(p, normalize(p) * FIREBALL_RADIUS, 0.72);
        }
    }

    let aft = saturate(-p.y / FIREBALL_RADIUS);
    let speed_k = saturate(displace.y / 30.0);
    let grow = life();
    let stretch = aft * aft * tail_length() * (0.55 + 0.45 * speed_k);
    p.y -= stretch;
    let pinch = 1.0 - aft * mix(0.28, 0.62, grow);
    p.x *= pinch;
    p.z *= pinch;
    return p;
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var vertex = vertex_no_morph;
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

#ifdef VERTEX_POSITIONS
#ifdef VERTEX_NORMALS
    let normal = vertex.normal;
#else
    let normal = vec3<f32>(0.0, 1.0, 0.0);
#endif
    vertex.position = displace_local(vertex.position, normal);
#endif

#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index,
    );
#endif

#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.position = position_world_to_clip(out.world_position.xyz);
#endif

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index,
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index,
        world_from_local[3],
    );
#endif
    return out;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(mesh.world_normal);
    let v = normalize(view.world_position.xyz - mesh.world_position.xyz);
    let facing = saturate(dot(n, v));
    let core = vec3<f32>(1.0, 0.95, 0.62);
    let color = mix(base_color.xyz, core, facing * facing);
    return tone_mapping(vec4<f32>(color * 1.55, 1.0), view.color_grading);
}
