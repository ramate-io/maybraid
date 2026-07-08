//---------------------------------------------------------
// Watercolor post-process: edge-aware 3x3 blur blended
// back into the scene color.
//---------------------------------------------------------

struct WatercolorPostProcessSettings {
    blur_amount: f32,
    blur_radius: f32,
    depth_edge_sharpness: f32,
    edge_aware: f32,
}

@group(0) @binding(0) var color_tex: texture_2d<f32>;
@group(0) @binding(1) var color_sampler: sampler;
#ifdef MULTISAMPLED_DEPTH
@group(0) @binding(2) var depth_tex: texture_depth_multisampled_2d;
#else
@group(0) @binding(2) var depth_tex: texture_depth_2d;
#endif
@group(0) @binding(3) var<uniform> settings: WatercolorPostProcessSettings;

fn sample_scene(uv: vec2<f32>) -> vec3<f32> {
    return textureSample(color_tex, color_sampler, uv).rgb;
}

fn sample_depth(uv: vec2<f32>) -> f32 {
    let dims = textureDimensions(depth_tex);
    let max_coord = vec2<f32>(dims) - vec2<f32>(1.0);
    let coord = vec2<i32>(clamp(uv * vec2<f32>(dims), vec2<f32>(0.0), max_coord));
    return textureLoad(depth_tex, coord, 0);
}

fn edge_weight(center_uv: vec2<f32>, offset_uv: vec2<f32>) -> f32 {
    if (settings.edge_aware < 0.5) {
        return 1.0;
    }

    let center_depth = sample_depth(center_uv);
    let neighbor_depth = sample_depth(offset_uv);
    let depth_diff = abs(center_depth - neighbor_depth);
    return exp(-depth_diff * settings.depth_edge_sharpness);
}

fn blur_3x3(uv: vec2<f32>, texel: vec2<f32>) -> vec3<f32> {
    let radius = max(settings.blur_radius, 0.001);
    let step = texel * radius;

    var c = sample_scene(uv) * 0.36 * edge_weight(uv, uv);

    c += sample_scene(uv + step * vec2<f32>( 1.0,  0.0)) * 0.12 * edge_weight(uv, uv + step * vec2<f32>( 1.0,  0.0));
    c += sample_scene(uv + step * vec2<f32>(-1.0,  0.0)) * 0.12 * edge_weight(uv, uv + step * vec2<f32>(-1.0,  0.0));
    c += sample_scene(uv + step * vec2<f32>( 0.0,  1.0)) * 0.12 * edge_weight(uv, uv + step * vec2<f32>( 0.0,  1.0));
    c += sample_scene(uv + step * vec2<f32>( 0.0, -1.0)) * 0.12 * edge_weight(uv, uv + step * vec2<f32>( 0.0, -1.0));

    c += sample_scene(uv + step * vec2<f32>( 1.0,  1.0)) * 0.07 * edge_weight(uv, uv + step * vec2<f32>( 1.0,  1.0));
    c += sample_scene(uv + step * vec2<f32>(-1.0,  1.0)) * 0.07 * edge_weight(uv, uv + step * vec2<f32>(-1.0,  1.0));
    c += sample_scene(uv + step * vec2<f32>( 1.0, -1.0)) * 0.07 * edge_weight(uv, uv + step * vec2<f32>( 1.0, -1.0));
    c += sample_scene(uv + step * vec2<f32>(-1.0, -1.0)) * 0.07 * edge_weight(uv, uv + step * vec2<f32>(-1.0, -1.0));

    return c;
}

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(color_tex);
    let texel = 1.0 / vec2<f32>(dims);

    let original = sample_scene(uv);
    let blurred = blur_3x3(uv, texel);
    let watercolor = mix(original, blurred, clamp(settings.blur_amount, 0.0, 1.0));

    return vec4<f32>(watercolor, 1.0);
}
