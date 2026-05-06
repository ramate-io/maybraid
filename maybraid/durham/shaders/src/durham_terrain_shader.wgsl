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

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> base_color: vec4<f32>;

struct DurhamTerrainNoise {
    // x = seed, y = procedural vs base_color mix [0,1], zw unused
    config: vec4<f32>,
    // xyzw = frequency, amplitude, blend_weight, unused (per band)
    band0: vec4<f32>,
    band1: vec4<f32>,
    band2: vec4<f32>,
    band3: vec4<f32>,
    // rgb = color, w = segment weight
    palette0: vec4<f32>,
    palette1: vec4<f32>,
    palette2: vec4<f32>,
    palette3: vec4<f32>,
    palette4: vec4<f32>,
    palette5: vec4<f32>,
    palette6: vec4<f32>,
    palette7: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> terrain_noise: DurhamTerrainNoise;

// x unused, y edge strength, z edge darkness, w lit mix
@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> style_params: vec4<f32>;

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
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

    return mix(mix(mix(a, b, u.x), mix(c, d, u.x), u.y), mix(mix(e, f, u.x), mix(g, h, u.x), u.y), u.z);
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

fn fbm_continuous_scaled(p: vec3<f32>, seed: f32, amp: f32, freq: f32) -> f32 {
    let c = fbm(p * 0.06, seed + 91.0, amp, freq);
    let local_scale = exp2(mix(-1.5, 2.0, c));
    return fbm(p * local_scale, seed + 12.0, amp, freq);
}

// Two low FBMs warp XY only (cheaper than a third octave on Z).
fn domain_warp_offset(p: vec3<f32>, seed: f32, amp: f32, freq: f32) -> vec3<f32> {
    let qx = fbm(p * 0.35 + vec3<f32>(17.1, 3.7, 1.0), seed + 10.0, amp, freq);
    let qy = fbm(p * 0.35 + vec3<f32>(8.3, 29.4, 1.0), seed + 20.0, amp, freq);
    return vec3<f32>((qx - 0.5) * 4.0, (qy - 0.5) * 4.0, 0.0);
}

fn warped_scaled_fbm(p: vec3<f32>, seed: f32, amp: f32, freq: f32) -> f32 {
    let warp = domain_warp_offset(p, seed, amp, freq);
    return fbm_continuous_scaled(p + warp, seed + 3.0, amp, freq);
}

fn chaotic_periodic(t: f32, seed: f32) -> f32 {
    var x = t;
    x = x + 0.18 * sin(6.28318 * (x * 2.0 + seed * 0.13));
    x = x + 0.10 * sin(6.28318 * (x * 5.0 + seed * 0.31));
    return saturate(x);
}

fn palette_rgba(i: i32) -> vec4<f32> {
    switch i {
        case 0: { return terrain_noise.palette0; }
        case 1: { return terrain_noise.palette1; }
        case 2: { return terrain_noise.palette2; }
        case 3: { return terrain_noise.palette3; }
        case 4: { return terrain_noise.palette4; }
        case 5: { return terrain_noise.palette5; }
        case 6: { return terrain_noise.palette6; }
        case 7: { return terrain_noise.palette7; }
        default: { return vec4<f32>(0.0); }
    }
}

fn palette_rgb_at(t: f32) -> vec3<f32> {
    var w_sum = 0.0;
    for (var i = 0; i < 8; i = i + 1) {
        w_sum += max(palette_rgba(i).w, 1e-6);
    }
    let u = saturate(t) * w_sum;
    var acc = 0.0;
    for (var i = 0; i < 8; i = i + 1) {
        let e = palette_rgba(i);
        let wi = max(e.w, 1e-6);
        if (u <= acc + wi) {
            let f = saturate((u - acc) / wi);
            let c0 = e.xyz;
            let c1 = palette_rgba(min(i + 1, 7)).xyz;
            return mix(c0, c1, smoothstep(0.0, 1.0, f));
        }
        acc += wi;
    }
    return palette_rgba(7).xyz;
}

fn band_color(p: vec3<f32>, seed: f32, band: vec4<f32>) -> vec3<f32> {
    let composed = warped_scaled_fbm(p, seed, band.y, band.x);
    let t = chaotic_periodic(composed, seed);
    return palette_rgb_at(t);
}

fn ground_color(world_position: vec3<f32>) -> vec3<f32> {
    let cfg = terrain_noise.config;
    let seed = cfg.x;
    let p = world_position;

    var acc = vec3<f32>(0.0);
    var wsum = 0.0;

    let w0 = max(terrain_noise.band0.z, 0.0);
    acc += band_color(p, seed, terrain_noise.band0) * w0;
    wsum += w0;

    let w1 = max(terrain_noise.band1.z, 0.0);
    acc += band_color(p, seed, terrain_noise.band1) * w1;
    wsum += w1;

    let w2 = max(terrain_noise.band2.z, 0.0);
    acc += band_color(p, seed, terrain_noise.band2) * w2;
    wsum += w2;

    let w3 = max(terrain_noise.band3.z, 0.0);
    acc += band_color(p, seed, terrain_noise.band3) * w3;
    wsum += w3;

    let mixed = acc / max(wsum, 1e-6);
    return mix(base_color.rgb, mixed, saturate(cfg.y));
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
    let world_ground_color = ground_color(mesh.world_position.xyz);

    pbr_input.material.base_color = vec4<f32>(world_ground_color, base_color.a);
    pbr_input.material.metallic = 0.0;
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;

    let double_sided = (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;
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
    let final_color = mix(world_ground_color * edge_intensity, toned.rgb, lit_mix);

    return vec4<f32>(final_color, 1.0);
}
