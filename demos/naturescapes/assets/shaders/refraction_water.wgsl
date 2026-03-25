#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    mesh_view_bindings::view_transmission_texture,
    mesh_view_bindings::view_transmission_sampler,
    mesh_view_bindings::depth_prepass_texture,
    prepass_utils::prepass_depth,
    utils::coords_to_viewport_uv,
    view_transformations::depth_ndc_to_view_z,
    mesh_view_bindings::globals,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> water_color: vec4<f32>; // rgb tint, a = opacity

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<uniform> shallow_color: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(4)
var<uniform> deep_color: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(5)
var<uniform> distortion_strength: f32;

@group(#{MATERIAL_BIND_GROUP}) @binding(6)
var<uniform> depth_scale: f32;

@group(#{MATERIAL_BIND_GROUP}) @binding(7)
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

fn tri_wave(x: f32) -> f32 {
    let f = fract(x);
    return 1.0 - abs(f * 2.0 - 1.0);
}

// cheap hash (no sin)
fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 345.45));
    let d = dot(q, q + 34.345);
    return fract(q.x * q.y + d);
}

fn rot2(p: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(c * p.x - s * p.y, s * p.x + c * p.y);
}


@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Screen UV in [0,1]
    var viewport_uv = coords_to_viewport_uv(in.position.xy, view.viewport);
    let viewport = textureSample(view_transmission_texture, view_transmission_sampler, viewport_uv).rgb;

    let visual_depth = textureLoad(depth_prepass_texture, vec2<i32>(in.position.xy), 0);
    let volume_depth = depth_ndc_to_view_z(in.position.z) - depth_ndc_to_view_z(visual_depth);

    // vec3 world/camera positions
    let cam_pos = view.world_position.xyz;
    let world_pos = in.world_position.xyz;

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
    let deepness = saturate((volume_depth + visual_depth) / depth_scale);
    let tint = mix(shallow_color.rgb, deep_color.rgb, deepness);

    // Manual compositing (fake transparency)
    let base_opacity = saturate(water_color.a);
    let opacity = saturate(base_opacity * deepness);

    var col = mix(scene_col, tint, opacity);

    // Stylize: multiply tint + posterize
    col = mix(col, col * water_color.rgb, 0.45);

    // Toon-ish steps
    col = posterize3(col, 40.0);

    // -------------------------
    // Caustics / white flecks
    // -------------------------
    // Use world XZ so it "sticks" to the world, not the camera.
    let uv = world_pos.xz;
    let t = globals.time/4.0;

    // cell density
    let cell_scale = 0.45;

    // quantize into base cells
    let cell = floor(uv * cell_scale);

    // per-cell random
    let r = hash21(cell);

    // warp uv inside the cell (irregularity)
    let warp = (r - 0.5) * 0.75; // strength
    let warped_uv = uv + warp;
    let p = warped_uv;
    let angle = hash21(cell) * 6.2831853;
    let cell_uv = fract(warped_uv * cell_scale); // 0..1 within cell
    let local = cell_uv - 0.5;                   // center at 0
    let pr = rot2(local, angle);


    // Two moving line fields at different angles
    let u1 = pr.x * 3.0 + t * 0.6;
    let u2 = pr.y * 2.2 - t * 0.45;

    // Triangle wave gives repeating ridges
    let w1 = tri_wave(u1);

    // Turn ridges into thin bright lines
    let line1 = smoothstep(0.90, 0.985, w1);

    // Combine as line network
    var caustic = line1;

    // Probabilistic presence per cell (keep your patchiness)
    let cell_r = hash21(cell);
    let presence = smoothstep(0.65, 0.75, cell_r);
    caustic *= presence;

    // Optional strength variation
    let cell_strength = mix(0.35, 1.25, hash21(cell + vec2<f32>(7.0, 13.0)));
    caustic *= cell_strength;

    // Apply as white line highlights
    col += vec3<f32>(1.0) * caustic * (0.12 + (deepness * 0.25));

    return vec4(col, 1.0);
}
