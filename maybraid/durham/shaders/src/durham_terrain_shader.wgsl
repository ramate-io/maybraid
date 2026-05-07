//---------------------------------------------------------
// Durham terrain: PBR with world-space palette noise.
//---------------------------------------------------------
#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    prepass_utils::prepass_depth,
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct DurhamSwatch {
    left: vec4<f32>,
    right: vec4<f32>,
    swatch_meta: vec4<f32>,
}

struct DurhamTerrainBand {
    config: vec4<f32>,
    band_scale: vec4<f32>,
    swatches: array<DurhamSwatch, 8>,
}

struct DurhamTerrainNoise {
    regional_blend: vec4<f32>,
    global_seed: vec4<f32>,
    bands: array<DurhamTerrainBand, 4>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> terrain_noise: DurhamTerrainNoise;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> style_params: vec4<f32>;

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn spread_noise(t: f32, amount: f32) -> f32 {
    let x = saturate(t);
    return saturate(0.5 + (x - 0.5) * amount);
}

fn hash31(p: vec3<f32>) -> f32 {
    let q = vec3<f32>(
        dot(p, vec3<f32>(127.1, 311.7, 269.5)),
        dot(p, vec3<f32>(269.5, 183.3, 127.1)),
        dot(p, vec3<f32>(183.3, 127.1, 311.7))
    );
    return fract(sin(q.x + q.y + q.z) * 43758.5453123);
}

fn value_noise(p: vec3<f32>, seed: f32) -> f32 {
    let i = floor(p);
    let nf = fract(p);
    let u = nf * nf * (3.0 - 2.0 * nf);
    let s = vec3<f32>(seed * 17.13, seed * 31.71, seed * 12.34);

    let a = hash31(i + vec3<f32>(0.0, 0.0, 0.0) + s);
    let b = hash31(i + vec3<f32>(1.0, 0.0, 0.0) + s);
    let c = hash31(i + vec3<f32>(0.0, 1.0, 0.0) + s);
    let d = hash31(i + vec3<f32>(1.0, 1.0, 0.0) + s);
    let e = hash31(i + vec3<f32>(0.0, 0.0, 1.0) + s);
    let f = hash31(i + vec3<f32>(1.0, 0.0, 1.0) + s);
    let g = hash31(i + vec3<f32>(0.0, 1.0, 1.0) + s);
    let h = hash31(i + vec3<f32>(1.0, 1.0, 1.0) + s);

    return mix(
        mix(mix(a, b, u.x), mix(c, d, u.x), u.y),
        mix(mix(e, f, u.x), mix(g, h, u.x), u.y),
        u.z
    );
}

fn fbm(p: vec3<f32>, seed: f32, amp: f32, freq: f32) -> f32 {
    var sum = 0.0;
    var a = amp;
    var f = freq;

    for (var octave = 0; octave < 4; octave = octave + 1) {
        sum += value_noise(p * f, seed + f32(octave) * 19.17) * a;
        f *= 2.03;
        a *= 0.5;
    }

    return saturate(sum);
}

fn domain_warp_offset(p: vec3<f32>, seed: f32, amp: f32, freq: f32) -> vec3<f32> {
    let qx = fbm(p + vec3<f32>(17.1, 3.7, 1.0), seed + 10.0, amp, freq * 0.35);
    let qy = fbm(p + vec3<f32>(8.3, 29.4, 1.0), seed + 20.0, amp, freq * 0.35);
    let qz = fbm(p + vec3<f32>(2.9, 61.7, 13.0), seed + 30.0, amp, freq * 0.35);

    return (vec3<f32>(qx, qy, qz) - vec3<f32>(0.5)) * 260.0;
}

fn palette_noise(p: vec3<f32>, seed: f32, amp: f32, freq: f32) -> f32 {
    let warp = domain_warp_offset(p, seed, amp, freq);
    let n = fbm(p + warp, seed + 3.0, amp, freq);

    return spread_noise(n, 1.75);
}

fn swatch_linear(sw: DurhamSwatch, along: f32) -> vec3<f32> {
    return mix(sw.left.xyz, sw.right.xyz, saturate(along));
}

/// Half-width in **continuous** `u = t*8` around each integer seam for joint-only blending.
const SWATCH_INDEX_SOFTNESS: f32 = 0.12;

fn swatch_sample(t_noise: f32, band: DurhamTerrainBand) -> vec3<f32> {
    let u = min(saturate(t_noise) * 8.0, 8.0 - 1e-4);
    let e = SWATCH_INDEX_SOFTNESS;

    // `floor(u)` / `fract(u)` jump at swatch seams; per-cell `f` made left/right limits disagree.
    // In `|u - s| <= e` around seam **s**, blend only **sw[s-1](1)** → **sw[s](0)** in **u** (same
    // expression from both sides of **s**).
    for (var s = 1; s < 8; s = s + 1) {
        let du = u - f32(s);
        if (abs(du) <= e) {
            let t = smoothstep(0.0, 1.0, (du + e) / (2.0 * e));
            return mix(
                swatch_linear(band.swatches[s - 1], 1.0),
                swatch_linear(band.swatches[s], 0.0),
                t
            );
        }
    }

    let i = min(i32(floor(u)), 7);
    return swatch_linear(band.swatches[i], fract(u));
}

fn band_color(p: vec3<f32>, band: DurhamTerrainBand) -> vec3<f32> {
    let seed = band.config.x;
    let t = palette_noise(p, seed, band.band_scale.y, band.band_scale.x);
    return swatch_sample(t, band);
}

fn ground_color(world_position: vec3<f32>) -> vec3<f32> {
    let p = world_position + vec3<f32>(1298.0, 18229.0, 0.0);

    var acc = vec3<f32>(0.0);
    var wsum = 0.0;

    for (var bi = 0; bi < 4; bi = bi + 1) {
        let band = terrain_noise.bands[bi];
        let w = spread_noise(max(band.band_scale.z, 0.0), 1.35);

        acc += band_color(p, band) * w;
        wsum += w;
    }

    return acc / max(wsum, 1e-6);
}

fn depth_at(pos: vec4<f32>) -> f32 {
    return prepass_depth(pos, 0);
}

fn depth_edge_laplacian(pos: vec4<f32>, strength: f32) -> f32 {
    let d0 = depth_at(pos);
    let dR = depth_at(pos + vec4<f32>( 1.0,  0.0, 0.0, 0.0));
    let dL = depth_at(pos + vec4<f32>(-1.0,  0.0, 0.0, 0.0));
    let dU = depth_at(pos + vec4<f32>( 0.0,  1.0, 0.0, 0.0));
    let dD = depth_at(pos + vec4<f32>( 0.0, -1.0, 0.0, 0.0));

    let lap = abs((dR + dL + dU + dD) - (4.0 * d0));
    let scale = max(0.05, abs(1000.0 * d0));

    return saturate((lap / scale) * strength);
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();
    let ground = ground_color(mesh.world_position.xyz);

    pbr_input.material.base_color = vec4<f32>(ground, 1.0);
    pbr_input.material.metallic = 0.0;
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;

    let double_sided =
        (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

    pbr_input.world_normal = fns::prepare_world_normal(
        mesh.world_normal,
        double_sided,
        is_front,
    );

    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.N = normalize(pbr_input.world_normal);
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    let lit_color = fns::apply_pbr_lighting(pbr_input);

    let edge = depth_edge_laplacian(mesh.position, style_params.y);
    let edge_ink = saturate(edge * 1000.0);
    let edge_intensity = mix(1.0, saturate(style_params.z), edge_ink);

    let shaded = lit_color.rgb * edge_intensity;
    let toned = tone_mapping(vec4<f32>(shaded, 1.0), view.color_grading);
    let lit_mix = saturate(style_params.w);
    let final_color = mix(ground * edge_intensity, toned.rgb, lit_mix);

    return vec4<f32>(final_color, 1.0);
}
