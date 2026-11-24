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

    // Keep energy on a thin broken ring. hole*rim filled the disc into an oval.
    let radial_band = exp(-pow(abs(ring - radius) / max(thickness * 0.18, 0.04), 2.0));

    // Discrete spirit filaments around the heading (PR 1585 streak language).
    let turns = around * 0.15915494;
    let lobe_a = fract(turns * 7.0 + seed * 2.7 + extras.x * 0.31);
    let lobe_b = fract(turns * 5.0 - seed * 1.9 + extras.x * 0.17);
    let filament = saturate(
        pow(1.0 - abs(lobe_a * 2.0 - 1.0), 14.0)
            + 0.55 * pow(1.0 - abs(lobe_b * 2.0 - 1.0), 10.0),
    );

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
    let streaks = mix(0.35, 1.35, value_noise(streak_uv));
    let wisps = mix(0.05, 1.55, value_noise(wisp_uv * 1.7));
    let jets = mix(0.0, 1.0, value_noise(wisp_uv * 3.1 + vec3<f32>(around * 4.0, along * 5.0, seed)));
    let crackle = mix(0.2, 1.0, value_noise(vec3<f32>(around * 9.0, along * 6.0, extras.x * 4.0 + seed)));
    let spirit = pow(saturate(wisps), 1.15) * (0.25 + 0.85 * jets) * crackle;
    let directional = filament * spirit;

    let front_band = 1.0 - smoothstep(0.0, thickness * 0.55, abs(along - thickness * 0.08));
    let leading = smoothstep(-thickness * 0.25, thickness * 0.4, along);
    let behind = saturate(-along / max(thickness * 4.0, 0.25));
    let trail_falloff = exp(-behind * 2.4) * (1.0 - leading);

    let grown = smoothstep(0.06, 0.28, extras.x);
    // Filament ticks only — a filled compress body reads as a translucent oval.
    let compress = front_band * leading * radial_band * filament;
    let trail = radial_band * filament * spirit * trail_falloff * grown * 1.35;
    let alpha = saturate((compress * 0.9 + trail * 0.55) * facing * fade);
    if (alpha < 0.04) {
        discard;
    }

    var rgb = tint.xyz * compress * (0.7 + streaks * 0.9) * intensity;
    rgb += tint.xyz * trail * (0.35 + directional * 1.6) * intensity;
    let mapped = tone_mapping(vec4<f32>(rgb, 1.0), view.color_grading);
    return vec4<f32>(mapped.rgb, alpha);
}
