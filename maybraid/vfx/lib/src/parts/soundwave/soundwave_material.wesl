//---------------------------------------------------------
// Directional soundwave: Bevy torus is around local Y.
// Pose maps that axle onto the live wave heading so the
// disc sits in the gameplay plane (local +Z travel).
// The advancing front is a soft additive compression edge
// with heading-aligned streaks, readable from the player.
//---------------------------------------------------------

import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::forward_io::{Vertex, UncompressedVertex, decompress_vertex};
import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_pbr::render::mesh_view_bindings::view;
import bevy_core_pipeline::tonemapping::tone_mapping;

@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var<uniform> wave: vec4<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(1)
var<uniform> front: vec4<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(2)
var<uniform> tint: vec4<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(3)
var<uniform> extras: vec4<f32>;

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn value_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let n000 = hash21(i.xy);
    let n100 = hash21(i.xy + vec2<f32>(1.0, 0.0));
    let n010 = hash21(i.xy + vec2<f32>(0.0, 1.0));
    let n110 = hash21(i.xy + vec2<f32>(1.0, 1.0));
    let n001 = hash21(i.xz);
    let n101 = hash21(i.xz + vec2<f32>(1.0, 0.0));
    return mix(
        mix(mix(n000, n100, u.x), mix(n010, n110, u.x), u.y),
        mix(n001, n101, u.x),
        u.z,
    );
}

@vertex
fn vertex(packed_vertex: Vertex) -> VertexOutput {
    let vertex_no_morph = decompress_vertex(packed_vertex, packed_vertex.instance_index);
    var out: VertexOutput;
    var vertex = vertex_no_morph;
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    @if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index,
    );
    @if(VERTEX_POSITIONS)
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
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
    let center = wave.xyz;
    let radius = max(wave.w, 1e-3);
    let heading = normalize(front.xyz + vec3<f32>(1e-5, 0.0, 0.0));
    let thickness = max(front.w, 1e-3);
    let intensity = extras.y;
    let fade = extras.z;
    let seed = extras.w;

    let world = mesh.world_position.xyz;
    let rel = world - center;
    let along = dot(rel, heading);
    let radial = rel - heading * along;
    let ring = length(radial);

    // Off-axis basis around the travel heading. Radius+along noise is
    // rotationally symmetric; around/across break the pale disc.
    let up = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(heading.y) > 0.9);
    let side = normalize(cross(heading, up));
    let lift = cross(side, heading);
    let around = atan2(dot(radial, lift), dot(radial, side));
    let across = dot(radial, side);

    let view_dir = normalize(view.world_position.xyz - world);
    let n = normalize(mesh.world_normal);
    let facing = saturate(abs(dot(n, view_dir)));

    let front_band = 1.0 - smoothstep(0.0, thickness, abs(along - thickness * 0.15));
    let leading = smoothstep(-thickness, thickness * 0.35, along);
    let rim = 1.0 - smoothstep(radius * 0.72, radius, ring);
    let hole = smoothstep(radius * 0.18, radius * 0.42, ring);

    let streak_uv = vec3<f32>(
        ring * 0.55 + around * 1.8 + seed,
        along * 2.4 + extras.x * 3.1,
        across * 1.35 + seed * 0.7,
    );
    let wisp_uv = vec3<f32>(
        around * 2.6 + seed * 1.3,
        along * 3.2 + extras.x * 2.1,
        across * 2.1 + ring * 0.4,
    );
    let streaks = mix(0.28, 1.2, value_noise(streak_uv));
    let wisps = mix(0.08, 1.45, value_noise(wisp_uv * 1.7));
    let jets = mix(0.0, 1.0, value_noise(wisp_uv * 3.1 + vec3<f32>(around * 4.0, along * 5.0, seed)));
    let directional = pow(saturate(wisps), 1.25) * (0.4 + 0.95 * jets);

    let compress = front_band * leading * hole * rim;
    let body = hole * rim * 0.18 * (1.0 - leading * 0.35);
    let trail = hole * rim * (1.0 - leading) * directional * 0.55;
    let alpha = saturate((compress * 1.2 + body + trail) * facing * fade);

    var rgb = tint.xyz * (0.22 + compress * 1.75) * streaks * (0.35 + directional) * intensity;
    rgb += tint.xyz * leading * compress * (0.65 + directional * 1.25);
    rgb += tint.xyz * trail * 1.15;
    let mapped = tone_mapping(vec4<f32>(rgb, 1.0), view.color_grading);
    return vec4<f32>(mapped.rgb, alpha);
}
