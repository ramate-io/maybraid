//---------------------------------------------------------
// Watercolor: soft half-Lambert lighting, value bands,
// cool shadow bleeding, and large paper/noise variation.
//---------------------------------------------------------
#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::{view, lights},
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct WatercolorLighting {
    band_count: f32,
    band_mix: f32,
    light_smooth_min: f32,
    light_smooth_max: f32,
    diffuse_scale: f32,
    diffuse_bias: f32,
    fallback_light: f32,
}

struct WatercolorShadow {
    tint_r: f32,
    tint_g: f32,
    tint_b: f32,
    _pad: f32,
}

struct WatercolorPaper {
    noise_scale: f32,
    noise_strength: f32,
    brightness_base: f32,
    seed: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> base_color: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> lighting: WatercolorLighting;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> shadow: WatercolorShadow;

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<uniform> paper: WatercolorPaper;

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(p.xyx * vec3<f32>(127.1, 311.7, 74.7));
    return fract(sin(dot(p3, vec3<f32>(12.9898, 78.233, 45.164))) * 43758.5453);
}

fn value_noise_2d(p: vec2<f32>, seed: f32) -> f32 {
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

fn paper_noise(world_pos: vec3<f32>) -> f32 {
    let scale = max(paper.noise_scale, 0.001);
    let uv = world_pos.xz * scale + world_pos.y * 0.17;
    return value_noise_2d(uv, paper.seed);
}

fn watercolor_light(normal: vec3<f32>) -> f32 {
    var light = 0.0;
    let count = min(lights.n_directional_lights, 4u);

    for (var i = 0u; i < count; i = i + 1u) {
        let light_dir = normalize(lights.directional_lights[i].direction_to_light);
        let ndotl = dot(normal, light_dir) * lighting.diffuse_scale + lighting.diffuse_bias;
        light = max(light, ndotl);
    }

    if (count == 0u) {
        light = lighting.fallback_light;
    }

    light = smoothstep(lighting.light_smooth_min, lighting.light_smooth_max, light);

    let band_count = max(lighting.band_count, 1.0);
    let bands = floor(light * band_count) / band_count;
    return mix(light, bands, saturate(lighting.band_mix));
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    let double_sided = false;
    let prepared_normal = fns::prepare_world_normal(
        mesh.world_normal,
        double_sided,
        is_front,
    );
    let normal = normalize(prepared_normal);

    let light = watercolor_light(normal);

    let base_rgb = base_color.rgb;
    let shadow_tint = vec3<f32>(shadow.tint_r, shadow.tint_g, shadow.tint_b);
    let lit_rgb = base_rgb;
    var color = mix(shadow_tint * base_rgb, lit_rgb, light);

    let noise = paper_noise(mesh.world_position.xyz);
    color *= paper.brightness_base + noise * paper.noise_strength;

    return tone_mapping(vec4<f32>(color, base_color.a), view.color_grading);
}
