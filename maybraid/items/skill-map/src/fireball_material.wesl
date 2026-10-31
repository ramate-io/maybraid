//---------------------------------------------------------
// Shared fireball bead: short object-space tail, one material for all.
// Pose and scale live on Transform. Flight is object +Y.
//---------------------------------------------------------

import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::forward_io::{Vertex, UncompressedVertex, decompress_vertex};
import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::view;
import bevy_pbr::render::mesh_view_bindings::globals;
import bevy_core_pipeline::tonemapping::tone_mapping;

@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var<uniform> base_color: vec4<f32>;

const FIREBALL_RADIUS: f32 = 1.18;
const TAIL: f32 = 0.7;

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

fn displace_local(local: vec3<f32>, normal: vec3<f32>, instance_index: u32) -> vec3<f32> {
    let t = globals.time + f32(instance_index) * 0.17;
    let boil = value_noise_3d(local * 2.2 + vec3<f32>(t * 1.7, f32(instance_index) * 0.03, -t));
    var p = local + normal * ((boil - 0.5) * 0.14);
    if p.y > 0.0 {
        let r = length(p);
        if r > 1e-4 {
            p = mix(p, normalize(p) * FIREBALL_RADIUS, 0.72);
        }
        return p;
    }
    let aft = saturate(-p.y / FIREBALL_RADIUS);
    p.y -= aft * aft * TAIL;
    let pinch = 1.0 - aft * 0.4;
    p.x *= pinch;
    p.z *= pinch;
    return p;
}

@vertex
fn vertex(packed_vertex: Vertex) -> VertexOutput {
    let vertex_no_morph = decompress_vertex(packed_vertex, packed_vertex.instance_index);
    var out: VertexOutput;
    var vertex = vertex_no_morph;
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    @if(VERTEX_NORMALS)
    let normal = vertex.normal;
    @if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index,
    );
    @if(!VERTEX_NORMALS)
    let normal = vec3<f32>(0.0, 1.0, 0.0);
    @if(!VERTEX_NORMALS)
    out.world_normal = vec3<f32>(0.0, 1.0, 0.0);
    @if(VERTEX_POSITIONS)
    let local = displace_local(vertex.position, normal, vertex_no_morph.instance_index);
    @if(VERTEX_POSITIONS)
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(local, 1.0),
    );
    @if(VERTEX_POSITIONS)
    out.position = position_world_to_clip(out.world_position.xyz);
    @if(VERTEX_UVS_A)
    out.uv = vertex.uv;
    @if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;
    @if(VERTEX_TANGENTS)
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index,
    );
    @if(VERTEX_COLORS)
    out.color = vertex.color;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_no_morph.instance_index;
    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index,
        world_from_local[3],
    );
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
