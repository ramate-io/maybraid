// True density integration. No captured views or card-space opacity lookup.
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::forward_io::{Vertex, decompress_vertex};
import bevy_pbr::render::mesh_view_bindings::view;
import bevy_pbr::render::view_transformations::{position_world_to_clip, position_ndc_to_world, frag_coord_to_ndc};
import bevy_core_pipeline::tonemapping::tone_mapping;
@if(DEPTH_PREPASS)
import bevy_pbr::prepass::utils::prepass_depth;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var frame_a: texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var density_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var frame_b: texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> local_to_texture: mat4x4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var<uniform> world_to_local: mat4x4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var<uniform> bounds_min: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var<uniform> bounds_max: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(7) var<uniform> settings: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(8) var<uniform> tint: vec4<f32>;

struct VolumeVertex { @builtin(position) position: vec4<f32>, }
@vertex
fn vertex(packed: Vertex) -> VolumeVertex {
    let v = decompress_vertex(packed, packed.instance_index);
    let model = mesh_functions::get_world_from_local(v.instance_index);
    var lo = vec2(1.0); var hi = vec2(-1.0);
    var crosses_camera = false;
    for (var i = 0u; i < 8u; i += 1u) {
        let local = vec3(select(bounds_min.x, bounds_max.x, (i & 1u) != 0u),
                         select(bounds_min.y, bounds_max.y, (i & 2u) != 0u),
                         select(bounds_min.z, bounds_max.z, (i & 4u) != 0u));
        let clip = position_world_to_clip((model * vec4(local, 1.0)).xyz);
        if clip.w <= 0.00001 { crosses_camera = true; }
        else { lo = min(lo, clip.xy / clip.w); hi = max(hi, clip.xy / clip.w); }
    }
    // A near/camera-plane intersection can project without bound, including from inside.
    // In that case rasterize the viewport; the ray-box test still rejects all empty pixels.
    if crosses_camera { lo = vec2(-1.0); hi = vec2(1.0); }
    lo = clamp(lo, vec2(-1.0), vec2(1.0)); hi = clamp(hi, vec2(-1.0), vec2(1.0));
    var out: VolumeVertex;
    out.position = vec4(mix(lo, hi, v.position.xy + 0.5), 0.0, 1.0);
    if settings.w < 0.5 { out.position = vec4(2.0, 2.0, 0.0, 1.0); }
    return out;
}

// Slab intersection avoids reciprocal zero/NaN for parallel rays at a boundary.
fn intersect_box(origin: vec3<f32>, direction: vec3<f32>) -> vec2<f32> {
    var enter = 0.0; var leave = 1e20;
    for (var axis = 0u; axis < 3u; axis += 1u) {
        if abs(direction[axis]) < 1e-10 {
            if origin[axis] < 0.0 || origin[axis] > 1.0 { return vec2(1.0, 0.0); }
        } else {
            let a = -origin[axis] / direction[axis];
            let b = (1.0 - origin[axis]) / direction[axis];
            enter = max(enter, min(a, b)); leave = min(leave, max(a, b));
        }
    }
    return vec2(enter, leave);
}

@fragment
fn fragment(input: VolumeVertex) -> @location(0) vec4<f32> {
    let ndc = frag_coord_to_ndc(input.position);
    // Starting on the near plane works for perspective, orthographic, and inside-volume cameras.
    let world_origin = position_ndc_to_world(vec3(ndc.xy, 1.0));
    let world_direction = normalize(position_ndc_to_world(vec3(ndc.xy, 0.0001)) - world_origin);
    let local_origin = (world_to_local * vec4(world_origin, 1.0)).xyz;
    let local_direction = (world_to_local * vec4(world_direction, 0.0)).xyz;
    let origin = (local_to_texture * vec4(local_origin, 1.0)).xyz;
    let direction = (local_to_texture * vec4(local_direction, 0.0)).xyz;
    var interval = intersect_box(origin, direction);
    @if(DEPTH_PREPASS)
    let opaque = prepass_depth(input.position, 0u);
    @if(DEPTH_PREPASS)
    if opaque > 0.0 {
        let world_hit = position_ndc_to_world(vec3(ndc.xy, opaque));
        interval.y = min(interval.y, dot(world_hit - world_origin, world_direction));
    }
    if interval.y <= interval.x { discard; }
    let step_world = (interval.y - interval.x) / 96.0;
    let step_local = step_world * length(local_direction);
    var transmission = 1.0;
    for (var i = 0u; i < 96u; i += 1u) {
        let uv = origin + direction * (interval.x + (f32(i) + 0.5) * step_world);
        let a = textureSampleLevel(frame_a, density_sampler, uv, 0.0).r;
        let b = textureSampleLevel(frame_b, density_sampler, uv, 0.0).r;
        let density = mix(a, b, settings.x);
        if density >= 0.0 && density <= 65504.0 {
            transmission *= exp(-density * settings.z * step_local);
        }
        if transmission < 0.01 { break; }
    }
    let opacity = (1.0 - transmission) * settings.y * tint.a;
    if opacity < 0.00001 { discard; }
    var rgb = tint.rgb;
    @if(!HDR)
    rgb = tone_mapping(vec4(rgb, 1.0), view.color_grading).rgb;
    return vec4(rgb, opacity);
}
