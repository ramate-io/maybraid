//---------------------------------------------------------
// Chico canopy leaf: stylized UV noise silhouettes + PBR lighting.
// Adapted from `playgrounds/objects/assets/shaders/leaf_material.wgsl`.
//---------------------------------------------------------
#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
    pbr_bindings,
}
#import bevy_core_pipeline::tonemapping::tone_mapping


@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> base_color: vec4<f32>;


fn hash22(p: vec2<f32>) -> vec2<f32> {
    let p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let dot_val = dot(p3, p3 + 33.33);
    let p3_xy = vec2<f32>(p3.x, p3.y);
    let p3_yz = vec2<f32>(p3.y, p3.z);
    return fract((p3_xy + p3_yz) * vec2<f32>(dot_val, dot_val * 1.618));
}

fn grad(p: vec2<f32>) -> vec2<f32> {
    let h = hash22(p);
    let angle = h.x * 6.28318;
    return vec2<f32>(cos(angle), sin(angle));
}

fn perlin_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    var f = fract(p);

    f = f * f * (3.0 - 2.0 * f);

    let g00 = grad(i);
    let g10 = grad(i + vec2<f32>(1.0, 0.0));
    let g01 = grad(i + vec2<f32>(0.0, 1.0));
    let g11 = grad(i + vec2<f32>(1.0, 1.0));

    let d00 = f;
    let d10 = f - vec2<f32>(1.0, 0.0);
    let d01 = f - vec2<f32>(0.0, 1.0);
    let d11 = f - vec2<f32>(1.0, 1.0);

    let n00 = dot(g00, d00);
    let n10 = dot(g10, d10);
    let n01 = dot(g01, d01);
    let n11 = dot(g11, d11);

    let nx0 = mix(n00, n10, f.x);
    let nx1 = mix(n01, n11, f.x);
    return mix(nx0, nx1, f.y);
}

fn fractal_noise(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 10.0;

    for (var i = 0; i < 5; i++) {
        value += perlin_noise(p * frequency) * amplitude;
        frequency *= 2.0;
        amplitude *= 0.5;
    }

    return value * 0.5 + 0.5;
}


@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput
) -> @location(0) vec4<f32> {

    let noise_scale = 6.0;
    let noise_value = fractal_noise(mesh.world_position.xz * noise_scale);

    let threshold = 0.54;

    let alpha = step(threshold, noise_value);

    if (alpha < 0.001) {
        discard;
    }

    var pbr_input: PbrInput = pbr_input_new();

    pbr_input.material.base_color = base_color;

    let double_sided = (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = fns::prepare_world_normal(
        mesh.world_normal,
        double_sided,
        is_front,
    );
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.N = normalize(pbr_input.world_normal);
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    let lit_color = fns::apply_pbr_lighting(pbr_input);

    let output_color = vec4<f32>(lit_color.rgb, base_color.a * alpha);

    return tone_mapping(output_color, view.color_grading);
}
