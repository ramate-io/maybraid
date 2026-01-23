#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    mesh_view_bindings::view_transmission_texture,
    mesh_view_bindings::view_transmission_sampler,
    mesh_view_bindings::depth_prepass_texture,
    utils::coords_to_viewport_uv,
    view_transformations::depth_ndc_to_view_z,
}

// --------------------
// Material uniforms
// --------------------
@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> water_color: vec4<f32>; // rgb tint, a = opacity

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> shallow_color: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> deep_color: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<uniform> distortion_strength: f32;

@group(#{MATERIAL_BIND_GROUP}) @binding(4)
var<uniform> depth_scale: f32;

@group(#{MATERIAL_BIND_GROUP}) @binding(5)
var<uniform> close_fade_strength: f32;


// --------------------
// Helpers
// --------------------
fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn posterize3(c: vec3<f32>, steps: f32) -> vec3<f32> {
    let s = max(steps, 1.0);
    return floor(c * s) / s;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Screen UV in [0,1]
    var viewport_uv = coords_to_viewport_uv(in.position.xy, view.viewport);

    // vec3 world/camera positions
    let cam_pos = view.world_position.xyz;
    let world_pos = in.world_position.xyz;

    // Depth difference: scene behind minus water surface depth
    let depth_raw = textureLoad(depth_prepass_texture, vec2<i32>(in.position.xy), 0);
    let water_view_z = depth_ndc_to_view_z(in.position.z);
    let scene_view_z = depth_ndc_to_view_z(depth_raw);
    let depth = max(scene_view_z - water_view_z, 0.0);

    // Normal
    let n = normalize(in.world_normal);

    // Fade distortion near camera (reduces shimmer)
    let dist_to_cam = length(world_pos - cam_pos);
    let close_fade = saturate((dist_to_cam - 2.0) / 6.0);
    let fade = mix(1.0, close_fade, saturate(close_fade_strength));

    // Distort UV (tiny)
    viewport_uv += n.xz * distortion_strength * fade;

    // ONE sample only (this is the big perf win)
    let scene_col = textureSample(
        view_transmission_texture,
        view_transmission_sampler,
        viewport_uv
    ).rgb;

    // Depth tint
    let deepness = saturate(depth / max(depth_scale, 0.0001));
    let tint = mix(shallow_color.rgb, deep_color.rgb, deepness);

    // Manual compositing (fake transparency)
    let base_opacity = saturate(water_color.a);
    let opacity = saturate(base_opacity * (0.25 + deepness * 0.75));

    var col = mix(scene_col, tint, opacity);

    // Stylize: multiply tint + posterize
    col = mix(col, col * water_color.rgb, 0.45);

    // Simple ink rim darken (cheap, no pow)
    let V = normalize(world_pos - cam_pos);
    let NoV = saturate(dot(n, V));
    let rim = saturate((1.0 - NoV) * 1.1);
    col *= (1.0 - rim * 0.25);

    // Toon-ish steps
    col = posterize3(col, 6.0);

    return vec4<f32>(col, 1.0);
}
