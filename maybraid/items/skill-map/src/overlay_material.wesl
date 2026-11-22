//---------------------------------------------------------
// Skill-map overlay compositor: stained-glass warp of the map
// camera image so the world shows through coloured, refracted panes.
//---------------------------------------------------------
import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct OverlayParams {
    amount: f32,
    warp: f32,
    glass: f32,
    lens_radius: f32,
    curl: f32,
    flash: f32,
    film: f32,
    braid_turns: f32,
    flare: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
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
const OVERLAY_PANE_MIX: f32 = 0.50;
const OVERLAY_CAME_MIX: f32 = 0.28;
// Keep paint / came / mark constants in sync with overlay.rs OVERLAY_*.
const OVERLAY_CAME_ALPHA: f32 = 0.084;
const OVERLAY_PAINT_FLOOR: f32 = 0.16;
const OVERLAY_PAINT_GAIN: f32 = 0.36;
const OVERLAY_PAINT_CHROMA: f32 = 2.6;
const OVERLAY_PAINT_LUMA: f32 = 0.45;
const OVERLAY_PAINT_RECOVER: f32 = 0.08;
const OVERLAY_PAINT_SATURATION: f32 = 1.45;
const OVERLAY_MARK_ALPHA_LO: f32 = 0.60;
const OVERLAY_MARK_ALPHA_HI: f32 = 0.90;
// Keep in sync with lens.rs. Amp is apa-kan TILE_BRAID_AMP / 22.
const BRAID_AMP_FRAC: f32 = 0.07272727;
const BRAID_STROKE_FRAC: f32 = 0.038;
const LENS_RING_THICKNESS_FRAC: f32 = 0.055;
const LENS_EDGE_FRAC: f32 = 0.012;
const BRAID_STRANDS: i32 = 3;

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

fn unpremultiply(color: vec3<f32>, alpha: f32) -> vec3<f32> {
    return color / max(alpha, OVERLAY_PAINT_RECOVER);
}

fn paint_ink(pigment: vec3<f32>) -> f32 {
    let luma = dot(pigment, vec3<f32>(0.30, 0.59, 0.11));
    let chroma = length(pigment - vec3<f32>(luma));
    return saturate(chroma * OVERLAY_PAINT_CHROMA + luma * OVERLAY_PAINT_LUMA);
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

fn braid_strand_color(i: i32) -> vec3<f32> {
    if i == 0 {
        return vec3<f32>(0.82, 0.22, 0.18);
    } else if i == 1 {
        return vec3<f32>(0.86, 0.68, 0.18);
    }
    return vec3<f32>(0.22, 0.52, 0.86);
}

fn pixel_from_center(uv: vec2<f32>, size: vec2<f32>) -> vec2<f32> {
    return (uv - vec2<f32>(0.5, 0.5)) * max(size, vec2<f32>(1.0, 1.0));
}

fn lens_radius_px(size: vec2<f32>) -> f32 {
    return params.lens_radius * min(max(size.x, 1.0), max(size.y, 1.0));
}

fn lens_mask(uv: vec2<f32>, size: vec2<f32>) -> f32 {
    let r = length(pixel_from_center(uv, size));
    let radius = lens_radius_px(size);
    let edge = max(radius * LENS_EDGE_FRAC, 1e-4);
    return 1.0 - smoothstep(radius - edge, radius, r);
}

fn sphere_ring(uv: vec2<f32>, size: vec2<f32>, amount: f32) -> vec4<f32> {
    let p = pixel_from_center(uv, size);
    let r = length(p);
    let radius = lens_radius_px(size);
    let thick = max(radius * LENS_RING_THICKNESS_FRAC, 1.0);
    let d = abs(r - radius);
    let band = 1.0 - smoothstep(thick * 0.35, thick, d);
    let x = clamp((r - radius) / thick, -1.0, 1.0);
    let nz = sqrt(max(0.0, 1.0 - x * x));
    let nxy = select(vec2<f32>(0.0, 0.0), normalize(p) * x, r > 1e-4);
    let light = saturate(dot(vec3<f32>(nxy.x, nxy.y, nz), normalize(vec3<f32>(-0.35, 0.70, 0.55))));
    let spec = pow(light, 24.0);
    var col = mix(vec3<f32>(0.14, 0.11, 0.16), vec3<f32>(0.78, 0.86, 0.95), light * 0.70);
    col += vec3<f32>(0.95, 0.97, 1.0) * spec * 0.50;
    return vec4<f32>(col, band * amount * 0.94);
}

fn wrap_pi(a: f32) -> f32 {
    return atan2(sin(a), cos(a));
}

/// 12 o'clock in overlay pixel space (Y grows down).
fn braid_break() -> f32 {
    return -1.57079632679;
}

fn braid_strand_radial(theta: f32, i: i32, radius: f32, amp: f32, braid_radius: f32) -> f32 {
    let offset = 6.28318530718 * f32(i) / f32(BRAID_STRANDS);
    let phase = params.braid_turns * theta + offset;
    let woven = amp * sin(phase);
    let park = (f32(i) - 1.0) * amp * 2.35;
    let loosen = 1.0 - saturate(params.curl);
    let from_break = abs(wrap_pi(theta - braid_break()));
    let peel_width = loosen * 3.14159265359 + 1e-4;
    let peel = pow(saturate(1.0 - from_break / peel_width), 1.35) * loosen;
    let base = mix(woven, park, peel);
    let flare = peel * (park * 2.0 + radius * (0.08 + 0.18 * loosen)) * params.flare;
    return braid_radius + base + flare;
}

fn braid_color(uv: vec2<f32>, size: vec2<f32>, amount: f32) -> vec4<f32> {
    if amount <= 0.0 {
        return vec4<f32>(0.0);
    }
    let p = pixel_from_center(uv, size);
    let r = length(p);
    let theta = atan2(p.y, p.x);
    let radius = lens_radius_px(size);
    let ring_thick = max(radius * LENS_RING_THICKNESS_FRAC, 1.0);
    let amp = radius * BRAID_AMP_FRAC;
    let stroke = max(radius * BRAID_STROKE_FRAC, 1.0);
    let braid_radius = radius + ring_thick * 0.65;
    var alphas = array<f32, 3>(0.0, 0.0, 0.0);
    var colors = array<vec3<f32>, 3>(vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
    var depths = array<f32, 3>(-2.0, -2.0, -2.0);
    for (var i = 0; i < BRAID_STRANDS; i++) {
        let offset = 6.28318530718 * f32(i) / f32(BRAID_STRANDS);
        let phase = params.braid_turns * theta + offset;
        let radial = braid_strand_radial(theta, i, radius, amp, braid_radius);
        let dist = abs(r - radial);
        let body = 1.0 - smoothstep(stroke * 0.42, stroke, dist);
        let tube = 0.62 + 0.38 * (1.0 - saturate(dist / max(stroke, 1e-4)));
        let alpha = body * amount;
        if alpha > 0.01 {
            alphas[i] = alpha;
            colors[i] = braid_strand_color(i) * tube * (1.0 + params.flash * 1.55);
            depths[i] = cos(phase);
        }
    }
    // Back-to-front so each strand's over/under (cos phase) actually stacks.
    for (var a = 0; a < 2; a++) {
        for (var b = 0; b < 2 - a; b++) {
            if depths[b] > depths[b + 1] {
                let da = depths[b];
                depths[b] = depths[b + 1];
                depths[b + 1] = da;
                let aa = alphas[b];
                alphas[b] = alphas[b + 1];
                alphas[b + 1] = aa;
                let ca = colors[b];
                colors[b] = colors[b + 1];
                colors[b + 1] = ca;
            }
        }
    }
    var out_rgb = vec3<f32>(0.0);
    var out_a = 0.0;
    for (var i = 0; i < BRAID_STRANDS; i++) {
        let src_a = alphas[i];
        if src_a > 0.01 {
            out_rgb = mix(out_rgb, colors[i], src_a);
            out_a = src_a + out_a * (1.0 - src_a);
        }
    }
    return vec4<f32>(out_rgb, out_a);
}

fn film_sheen(uv: vec2<f32>, size: vec2<f32>, amount: f32) -> f32 {
    if params.film <= 0.0 {
        return 0.0;
    }
    let lens = lens_mask(uv, size);
    return params.film * amount * lens * 0.62;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let amount = saturate(params.amount);
    let size = in.size;
    let lens = lens_mask(in.uv, size);
    let cell = glass_cell(in.uv);
    let came = cell.z;
    let local = cell.xy;
    let seed = hash12(floor(in.uv * vec2<f32>(7.0, 5.5)));
    let n = value_noise(in.uv * 18.0 + local * 3.0);
    let normal = normalize(vec2<f32>(local.x, local.y) + vec2<f32>(n - 0.5, 0.5 - n) * 0.35);
    let warp = normal * (OVERLAY_WARP_SCALE * params.warp * amount * (1.0 - lens));
    let uv = clamp(in.uv + warp, vec2<f32>(0.0), vec2<f32>(1.0));
    let fringe = OVERLAY_FRINGE_SCALE * amount * params.warp * (1.0 - lens);
    let sample = textureSample(map_texture, map_sampler, uv);
    let r = textureSample(map_texture, map_sampler, uv + vec2<f32>(fringe, 0.0)).r;
    let g = sample.g;
    let b = textureSample(map_texture, map_sampler, uv - vec2<f32>(fringe, 0.0)).b;
    var color = mix(sample.xyz, vec3<f32>(r, g, b), amount * (1.0 - lens));
    let pigment = unpremultiply(color, sample.a);
    let luma = dot(pigment, vec3<f32>(0.30, 0.59, 0.11));
    color = mix(vec3<f32>(luma), pigment, OVERLAY_PAINT_SATURATION);
    let pane = glass_tint(seed);
    let glass_amt = amount * params.glass * (1.0 - lens);
    color = mix(color, mix(color, pane, OVERLAY_PANE_MIX), glass_amt);
    color = mix(color, vec3<f32>(0.06, 0.04, 0.05), came * glass_amt * OVERLAY_CAME_MIX);
    let lead = 0.22 + 0.18 * n;
    let paint = OVERLAY_PAINT_FLOOR
        + paint_ink(pigment) * OVERLAY_PAINT_GAIN
        + came * lead * OVERLAY_CAME_ALPHA;
    let keep = smoothstep(OVERLAY_MARK_ALPHA_LO, OVERLAY_MARK_ALPHA_HI, sample.a);
    let treated = mix(paint, sample.a, keep);
    let field = mix(sample.a, treated, amount * (1.0 - lens));
    // Interior is a hole: the 3D world shows through. Map marks stay outside.
    var alpha = field * (1.0 - lens);
    let ring = sphere_ring(in.uv, size, amount);
    color = mix(color, ring.xyz, ring.a);
    alpha = max(alpha, ring.a);
    let braid = braid_color(in.uv, size, amount);
    color = mix(color, braid.xyz, braid.a);
    alpha = max(alpha, braid.a);
    let film = film_sheen(in.uv, size, amount);
    color = mix(color, vec3<f32>(1.0, 0.82, 0.45), film);
    alpha = max(alpha, film * 0.40);
    return vec4<f32>(color, alpha);
}
