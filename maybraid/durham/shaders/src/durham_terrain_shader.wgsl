//---------------------------------------------------------
// Durham terrain: PBR with world-space palette noise.
//---------------------------------------------------------
#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    prepass_utils::prepass_depth,
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
    pbr_bindings,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> base_color: vec4<f32>;

// x = seed, y = regional scale, z = detail scale, w = value strength.
@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> noise_params: vec4<f32>;

// x = palette strength, y = edge strength, z = edge darkness, w = lit mix.
@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> style_params: vec4<f32>;

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn hash21(p: vec2<f32>) -> f32 {
    let q = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    );
    return fract(sin(q.x + q.y) * 43758.5453123);
}

fn value_noise(p: vec2<f32>, seed: f32) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let s = vec2<f32>(seed * 17.13, seed * 31.71);

    let a = hash21(i + vec2<f32>(0.0, 0.0) + s);
    let b = hash21(i + vec2<f32>(1.0, 0.0) + s);
    let c = hash21(i + vec2<f32>(0.0, 1.0) + s);
    let d = hash21(i + vec2<f32>(1.0, 1.0) + s);

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p: vec2<f32>, seed: f32) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var freq = 1.0;

    for (var octave = 0; octave < 5; octave = octave + 1) {
        sum += value_noise(p * freq, seed + f32(octave) * 19.17) * amp;
        freq *= 2.03;
        amp *= 0.5;
    }

    return saturate(sum);
}

fn palette_sample(t: f32) -> vec3<f32> {
    let brown = vec3<f32>(0.36, 0.28, 0.20);
    let gray = vec3<f32>(0.42, 0.38, 0.32);
    let red_brown = vec3<f32>(0.45, 0.30, 0.22);
    let yellow_brown = vec3<f32>(0.48, 0.44, 0.26);
    let dark_mineral = vec3<f32>(0.20, 0.18, 0.16);

    let x = saturate(t) * 4.0;
    let band = floor(x);
    let f = smoothstep(0.0, 1.0, fract(x));

    if band < 1.0 {
        return mix(brown, gray, f);
    }
    if band < 2.0 {
        return mix(gray, red_brown, f);
    }
    if band < 3.0 {
        return mix(red_brown, yellow_brown, f);
    }
    return mix(yellow_brown, dark_mineral, f);
}

fn ground_color(world_position: vec3<f32>) -> vec3<f32> {
    let world_xz = world_position.xz;
    let seed = noise_params.x;
    let regional = fbm(world_xz * noise_params.y, seed);
    let detail = fbm(world_xz * noise_params.z + vec2<f32>(11.7, -5.3), seed + 101.0);
    let broad_shadow = fbm(world_xz * noise_params.y * 0.33 + vec2<f32>(41.0, 7.0), seed + 211.0);

    let palette_color = palette_sample(regional);
    let value = mix(1.0 - noise_params.w, 1.0 + noise_params.w, detail);
    let mineral_drift = mix(0.88, 1.12, broad_shadow);
    let noisy_color = palette_color * value * mineral_drift;

    return mix(base_color.rgb, noisy_color, saturate(style_params.x));
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
