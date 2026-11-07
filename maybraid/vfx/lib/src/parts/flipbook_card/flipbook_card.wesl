// Eight authored capture planes; no density texture or runtime volume integration.
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::forward_io::{Vertex, decompress_vertex};
import bevy_pbr::render::mesh_view_bindings::view;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import bevy_core_pipeline::tonemapping::tone_mapping;

@if(DEPTH_PREPASS)
import bevy_pbr::prepass::utils::prepass_depth;
@if(DEPTH_PREPASS)
import bevy_pbr::render::view_transformations::position_ndc_to_world;
@if(DEPTH_PREPASS)
import bevy_pbr::render::view_transformations::frag_coord_to_ndc;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var atlas_image: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var atlas_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> time: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> atlas: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var<uniform> frame: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var<uniform> center: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var<uniform> capture: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(7) var<uniform> tint: vec4<f32>;

struct DustVertex {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) local_position: vec3<f32>,
    @location(2) @interpolate(flat) local_camera: vec3<f32>,
}

@vertex
fn vertex(packed_vertex: Vertex) -> DustVertex {
    let vertex = decompress_vertex(packed_vertex, packed_vertex.instance_index);
    let model = mesh_functions::get_world_from_local(vertex.instance_index);
    let axis_x = model[0].xyz;
    let axis_y = model[1].xyz;
    let axis_z = model[2].xyz;
    let local_from_world = mesh_functions::get_local_from_world(vertex.instance_index);
    let world_center = (model * vec4(center.xyz, 1.0)).xyz;
    // Camera axes also remain defined while looking straight down.
    let right = view.world_from_view[0].xyz;
    let up = view.world_from_view[1].xyz;
    // A square of sqrt(2) * max(width,height) encloses every rotated capture plane.
    let size = 1.414214 * max(capture.x, capture.y) * max(length(axis_x), max(length(axis_y), length(axis_z)));
    let world = world_center + (right * vertex.position.x + up * vertex.position.y) * size;
    var out: DustVertex;
    out.position = position_world_to_clip(world);
    out.world_position = world;
    out.local_position = (local_from_world * vec4(world, 1.0)).xyz;
    out.local_camera = (local_from_world * vec4(view.world_position.xyz, 1.0)).xyz;
    return out;
}

fn sample_frame(uv: vec2<f32>, index: u32, layer: i32) -> vec4<f32> {
    let tile = vec2<f32>(f32(index % u32(atlas.z)), f32(index / u32(atlas.z)));
    // Sample interior texel centers: even bilinear filtering cannot cross a frame gutter.
    let pixel = tile * (frame.xy + 2.0 * frame.z) + frame.z + 0.5 + clamp(uv, vec2(0.0), vec2(1.0)) * (frame.xy - 1.0);
    let sample = textureSampleLevel(atlas_image, atlas_sampler, pixel / atlas.xy, layer, 0.0);
    // sRGB decoding is provided by the texture. Interpolate associated linear RGB.
    return vec4(mix(sample.rgb, sample.rgb * sample.a, frame.w), sample.a);
}

struct PlaneSample { color: vec4<f32>, distance: f32, }
fn sample_plane(ro: vec3<f32>, rd: vec3<f32>, index: u32, limit: f32, ray_length: f32) -> PlaneSample {
    let angle = center.w + f32(index) * 6.28318530718 / capture.w;
    let right = vec3(cos(angle), 0.0, -sin(angle));
    let up = vec3(-sin(capture.z) * sin(angle), cos(capture.z), -sin(capture.z) * cos(angle));
    let normal = cross(right, up);
    let denom = dot(rd, normal);
    var out: PlaneSample;
    out.color = vec4(0.0);
    out.distance = 0.0;
    if abs(denom) < 0.00001 { return out; }
    let distance = dot(center.xyz - ro, normal) / denom;
    if distance <= 0.0 || distance >= limit { return out; }
    let delta = ro + rd * distance - center.xyz;
    let uv = vec2(dot(delta, right) / capture.x + 0.5, 0.5 - dot(delta, up) / capture.y);
    if any(uv < vec2(0.0)) || any(uv > vec2(1.0)) { return out; }
    out.color = mix(sample_frame(uv, u32(time.x), i32(index)), sample_frame(uv, u32(time.y), i32(index)), time.z);
    out.color *= smoothstep(0.0, 0.18, (limit - distance) * ray_length);
    out.distance = distance;
    return out;
}

struct DustFragment { @location(0) color: vec4<f32>, @builtin(frag_depth) depth: f32, }
@fragment
fn fragment(input: DustVertex) -> DustFragment {
    let offset = input.local_camera - center.xyz;
    let angle = fract((atan2(offset.x, offset.z) - center.w) / 6.28318530718) * capture.w;
    let first = u32(floor(angle)) % u32(capture.w);
    let second = (first + 1u) % u32(capture.w);
    let fraction = fract(angle);
    let blend = fraction * fraction * (3.0 - 2.0 * fraction);
    let ray = input.local_position - input.local_camera;
    let world_ray = input.world_position - view.world_position.xyz;
    let ray_length = length(world_ray);
    var limit = 1e20;
    @if(DEPTH_PREPASS)
    // Reject and soften EACH capture plane before blending, as in the authored demo.
    let opaque = prepass_depth(input.position, 0u);
    if opaque > 0.0 {
        let ndc = frag_coord_to_ndc(input.position);
        let opaque_world = position_ndc_to_world(vec3(ndc.xy, opaque));
        limit = max(0.0, dot(opaque_world - view.world_position.xyz, world_ray) / dot(world_ray, world_ray));
    }
    let a = sample_plane(input.local_camera, ray, first, limit, ray_length);
    let b = sample_plane(input.local_camera, ray, second, limit, ray_length);
    let sample = mix(a.color, b.color, blend);
    if sample.a < 0.00001 { discard; }
    let straight = sample.rgb / sample.a * tint.rgb;
    // Alpha-weighted depth avoids an empty neighboring plane moving the visible one.
    let distance = (a.distance * a.color.a * (1.0 - blend) + b.distance * b.color.a * blend) / sample.a;
    let world = view.world_position.xyz + (input.world_position - view.world_position.xyz) * distance;
    let clip = position_world_to_clip(world);
    var out: DustFragment;
    out.depth = clamp(clip.z / clip.w, 0.0, 1.0);
    var rgb = straight;
    @if(!HDR)
    rgb = tone_mapping(vec4(rgb, 1.0), view.color_grading).rgb;
    out.color = vec4(rgb, sample.a * time.w * tint.a);
    return out;
}
