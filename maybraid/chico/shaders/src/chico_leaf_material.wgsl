//---------------------------------------------------------
// Chico canopy leaf: opaque world-space volumetric canopy
// breakup + fake surface bumping + PBR lighting.
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

// --------------------------------------------------------
// Hash / noise
// --------------------------------------------------------

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33, 33.33, 33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn value_noise_3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (vec3<f32>(3.0, 3.0, 3.0) - 2.0 * f0);

    let n000 = hash13(i + vec3<f32>(0.0, 0.0, 0.0));
    let n100 = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash13(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash13(i + vec3<f32>(1.0, 1.0, 0.0));

    let n001 = hash13(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash13(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash13(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash13(i + vec3<f32>(1.0, 1.0, 1.0));

    let nx00 = mix(n000, n100, f.x);
    let nx10 = mix(n010, n110, f.x);
    let nx01 = mix(n001, n101, f.x);
    let nx11 = mix(n011, n111, f.x);

    let nxy0 = mix(nx00, nx10, f.y);
    let nxy1 = mix(nx01, nx11, f.y);

    return mix(nxy0, nxy1, f.z);
}

fn fbm_3d(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var norm = 0.0;

    for (var i = 0; i < 5; i++) {
        value += value_noise_3d(p * frequency) * amplitude;
        norm += amplitude;
        frequency *= 2.03;
        amplitude *= 0.5;
    }

    return value / norm;
}

fn ridged_3d(p: vec3<f32>) -> f32 {
    let n = fbm_3d(p);
    return 1.0 - abs(n * 2.0 - 1.0);
}

// --------------------------------------------------------
// Fragment
// --------------------------------------------------------

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();

    let world_pos = vec3<f32>(
        mesh.world_position.x,
        mesh.world_position.y,
        mesh.world_position.z,
    );

    // ----------------------------------------------------
    // World-space canopy silhouette breakup
    // ----------------------------------------------------

    let coarse = fbm_3d(world_pos * 0.85);
    let medium = fbm_3d(world_pos * 2.25);
    let fine = fbm_3d(world_pos * 7.50);
    let ridge = ridged_3d(world_pos * 3.25);

    let canopy_field =
        coarse * 0.48 +
        medium * 0.27 +
        fine * 0.15 +
        ridge * 0.10;

    let threshold = 0.58;

    if (canopy_field < threshold) {
        discard;
    }

    // ----------------------------------------------------
    // Color variation
    // ----------------------------------------------------

    let tint_noise = fbm_3d(world_pos * 1.75);
    let speckle = fbm_3d(world_pos * 12.0);

    let warm_cool = mix(
        vec3<f32>(0.82, 0.95, 0.72),
        vec3<f32>(1.12, 1.04, 0.78),
        tint_noise,
    );

    let brightness = mix(0.78, 1.18, speckle);

    let base_rgb = vec3<f32>(
        base_color.x,
        base_color.y,
        base_color.z,
    );

    pbr_input.material.base_color = vec4<f32>(
        base_rgb * warm_cool * brightness,
        1.0,
    );

    // ----------------------------------------------------
    // PBR setup
    // ----------------------------------------------------

    let double_sided =
        (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    let prepared_normal_raw = fns::prepare_world_normal(
        mesh.world_normal,
        double_sided,
        is_front,
    );

    let prepared_normal = vec3<f32>(
        prepared_normal_raw.x,
        prepared_normal_raw.y,
        prepared_normal_raw.z,
    );

    // ----------------------------------------------------
    // Fake world-space bump
    // ----------------------------------------------------

    let bump_scale = 5.0;
    let eps = 0.055;

    let bp = world_pos * bump_scale;
    let b0 = fbm_3d(bp);
    let bx = fbm_3d(bp + vec3<f32>(eps, 0.0, 0.0));
    let by = fbm_3d(bp + vec3<f32>(0.0, eps, 0.0));
    let bz = fbm_3d(bp + vec3<f32>(0.0, 0.0, eps));

    let bump_gradient = normalize(vec3<f32>(
        bx - b0,
        by - b0,
        bz - b0,
    ));

    let bump_strength = 0.16;
    let bumped_normal = normalize(prepared_normal + bump_gradient * bump_strength);

    pbr_input.world_normal = bumped_normal;
    pbr_input.N = bumped_normal;

    let lit_color = fns::apply_pbr_lighting(pbr_input);

    return tone_mapping(vec4<f32>(lit_color.rgb, 1.0), view.color_grading);
}
