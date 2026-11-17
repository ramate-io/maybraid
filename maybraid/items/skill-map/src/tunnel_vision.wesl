//---------------------------------------------------------
// Skill-map tunnel vision: vignette-style edge blur + darken
// of the 3D view so the centre stays sharp enough to navigate.
//---------------------------------------------------------
import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

struct TunnelVisionParams {
    amount: f32,
    strength: f32,
    radius: f32,
    darken: f32,
}

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> params: TunnelVisionParams;

fn edge_weight(uv: vec2<f32>) -> f32 {
    let d = length(uv - vec2<f32>(0.5, 0.5)) * 2.0;
    return smoothstep(params.radius, 1.0, d) * saturate(params.amount);
}

fn blur_rgb(uv: vec2<f32>, radius: f32) -> vec3<f32> {
    let o = radius;
    let d = o * 0.70710678;
    var acc = textureSample(screen_texture, texture_sampler, uv).rgb;
    acc += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(o, 0.0)).rgb;
    acc += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(-o, 0.0)).rgb;
    acc += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(0.0, o)).rgb;
    acc += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(0.0, -o)).rgb;
    acc += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(d, d)).rgb;
    acc += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(-d, d)).rgb;
    acc += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(d, -d)).rgb;
    acc += textureSample(screen_texture, texture_sampler, uv + vec2<f32>(-d, -d)).rgb;
    return acc / 9.0;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let sharp = textureSample(screen_texture, texture_sampler, in.uv);
    let edge = edge_weight(in.uv);
    let blurred = blur_rgb(in.uv, params.strength * edge);
    let rgb = mix(sharp.rgb, blurred, edge) * (1.0 - edge * params.darken);
    return vec4<f32>(rgb, sharp.a);
}
