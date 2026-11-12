//---------------------------------------------------------
// Skill-map overlay compositor: stained-glass warp of the map
// camera image so the world shows through coloured, refracted panes.
//---------------------------------------------------------
import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct OverlayParams {
    amount: f32,
    warp: f32,
    glass: f32,
    _pad: f32,
}

@group(1) @binding(0)
var map_texture: texture_2d<f32>;
@group(1) @binding(1)
var map_sampler: sampler;
@group(1) @binding(2)
var<uniform> params: OverlayParams;

// Compositor strengths. Previous values (warp 0.046, fringe 0.006, pane 0.55,
// came 0.82, sample 0.52, bias 0.20, came-alpha 0.25) veiling the world.
const OVERLAY_WARP_SCALE: f32 = 0.028;
const OVERLAY_FRINGE_SCALE: f32 = 0.0025;
const OVERLAY_PANE_MIX: f32 = 0.32;
const OVERLAY_CAME_MIX: f32 = 0.28;
// Keep SAMPLE/BIAS/CAME_ALPHA in sync with overlay.rs OVERLAY_COMPOSITOR_*.
const OVERLAY_ALPHA_SAMPLE: f32 = 0.88;
const OVERLAY_ALPHA_BIAS: f32 = 0.045;
const OVERLAY_CAME_ALPHA: f32 = 0.10;

fn hash12(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = p3 + vec3<f32>(dot(p3, p3.yzx + 33.33));
    return fract((d.x + d.y) * d.z);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * f0 * (f0 * (f0 * 6.0 - 15.0) + 10.0);
    let a = hash12(i);
    let b = hash12(i + vec2<f32>(1.0, 0.0));
    let c = hash12(i + vec2<f32>(0.0, 1.0));
    let d = hash12(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

/// Irregular leaded-glass cells. Returns local coordinates and a came mask.
fn glass_cell(uv: vec2<f32>) -> vec3<f32> {
    let scale = vec2<f32>(7.0, 5.5);
    let p = uv * scale;
    let cell = floor(p);
    let f = fract(p);
    var min_d = 8.0;
    var min_d2 = 8.0;
    var local = vec2<f32>(0.0);
    var tint = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let n = cell + vec2<f32>(f32(x), f32(y));
            let jitter = vec2<f32>(hash12(n), hash12(n + 19.7));
            let site = vec2<f32>(f32(x), f32(y)) + jitter * 0.72;
            let delta = f - site;
            let dist = length(delta);
            if dist < min_d {
                min_d2 = min_d;
                min_d = dist;
                local = delta;
                tint = hash12(n + 3.1);
            } else if dist < min_d2 {
                min_d2 = dist;
            }
        }
    }
    let came = 1.0 - smoothstep(0.02, 0.07, min_d2 - min_d);
    return vec3<f32>(local.x, local.y, came);
}

fn glass_tint(seed: f32) -> vec3<f32> {
    let ruby = vec3<f32>(0.62, 0.12, 0.16);
    let amber = vec3<f32>(0.78, 0.48, 0.12);
    let sapphire = vec3<f32>(0.16, 0.32, 0.72);
    let emerald = vec3<f32>(0.12, 0.48, 0.28);
    let violet = vec3<f32>(0.42, 0.16, 0.62);
    let t = fract(seed * 4.97);
    if t < 0.2 {
        return ruby;
    } else if t < 0.4 {
        return amber;
    } else if t < 0.6 {
        return sapphire;
    } else if t < 0.8 {
        return emerald;
    }
    return violet;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let amount = saturate(params.amount);
    let cell = glass_cell(in.uv);
    let came = cell.z;
    let local = cell.xy;
    let seed = hash12(floor(in.uv * vec2<f32>(7.0, 5.5)));
    let n = value_noise(in.uv * 18.0 + local * 3.0);
    let normal = normalize(vec2<f32>(local.x, local.y) + vec2<f32>(n - 0.5, 0.5 - n) * 0.35);
    let warp = normal * (OVERLAY_WARP_SCALE * params.warp * amount);
    let uv = clamp(in.uv + warp, vec2<f32>(0.0), vec2<f32>(1.0));
    let fringe = OVERLAY_FRINGE_SCALE * amount * params.warp;
    let sample = textureSample(map_texture, map_sampler, uv);
    let r = textureSample(map_texture, map_sampler, uv + vec2<f32>(fringe, 0.0)).r;
    let g = sample.g;
    let b = textureSample(map_texture, map_sampler, uv - vec2<f32>(fringe, 0.0)).b;
    var color = mix(sample.xyz, vec3<f32>(r, g, b), amount);
    let pane = glass_tint(seed);
    color = mix(color, mix(color, pane, OVERLAY_PANE_MIX), amount * params.glass);
    color = mix(color, vec3<f32>(0.06, 0.04, 0.05), came * amount * OVERLAY_CAME_MIX);
    let lead = 0.22 + 0.18 * n;
    let alpha = mix(
        sample.a,
        sample.a * OVERLAY_ALPHA_SAMPLE + OVERLAY_ALPHA_BIAS + came * lead * OVERLAY_CAME_ALPHA,
        amount,
    );
    return vec4<f32>(color, alpha);
}
