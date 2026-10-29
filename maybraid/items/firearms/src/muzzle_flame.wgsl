//---------------------------------------------------------
// Muzzle cone. Polar UVs: tip at (0.5, 0.5), base rim at radius 0.5.
// White core along the axis, warm limb at the silhouette.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> core: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> limb: vec4<f32>;

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var vertex = vertex_no_morph;
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
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

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
#ifdef VERTEX_UVS_A
    let uv = mesh.uv;
#else
    let uv = vec2<f32>(0.5, 0.5);
#endif
    // 0 at the tip, 1 at the base rim.
    let along = saturate(length(uv - vec2<f32>(0.5)) * 2.0);
    let n = mesh.world_normal;
    let n_len = length(n);
    let view_dir = normalize(view.world_position.xyz - mesh.world_position.xyz);
    // The cone tip's authored normal is zero; treat that as looking into the core.
    let facing = select(
        1.0,
        saturate(abs(dot(n / max(n_len, 1e-5), view_dir))),
        n_len > 1e-4,
    );
    let boil = hash21(uv * 8.0 + vec2<f32>(globals.time * 9.0, along * 4.0));
    let axis = 1.0 - smoothstep(0.06, 0.78, along);
    let hot = saturate(max(axis, facing * 0.9) + (boil - 0.5) * 0.22);
    let color = mix(limb.xyz, core.xyz, hot);
    let gain = max(core.w, 1.0);
    let flicker = 0.9 + 0.1 * sin(globals.time * 42.0 + along * 16.0 + boil * 6.0);
    let alpha = smoothstep(1.0, 0.28, along) * mix(0.45, 1.0, hot);
    let mapped = tone_mapping(vec4<f32>(color * gain * flicker, 1.0), view.color_grading);
    return vec4<f32>(mapped.rgb, alpha);
}
