//---------------------------------------------------------
// Nintendo-ish Water (Bevy 0.17 fragment shader)
// - UNLIT (no apply_pbr_lighting)
// - NO tonemapping
// - NO derivatives
// - NO noise/hash
// - NO pow
// - Camera-relative UVs (stable far from origin)
// - Simple animated bands + ink rim + soft foam tint
//---------------------------------------------------------

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
}


//---------------------------------------------------------
// Material uniforms (UNCHANGED)
//---------------------------------------------------------
@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> water_color: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> mix_color_a: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> mix_color_b: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<uniform> swirl_params: vec4<f32>; // x: scale, y: speed, z: mix_strength, w: foam_strength

@group(#{MATERIAL_BIND_GROUP}) @binding(4)
var<uniform> foam_params: vec4<f32>;  // unused (kept for layout)

@group(#{MATERIAL_BIND_GROUP}) @binding(5)
var<uniform> time: f32;


//---------------------------------------------------------
// Helpers
//---------------------------------------------------------
fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}


//---------------------------------------------------------
// Fragment Shader
//---------------------------------------------------------
@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput
) -> @location(0) vec4<f32> {

    // We only use PBR structs to get consistent N/V (cheap)
    var pbr: PbrInput = pbr_input_new();

    let double_sided =
        (pbr.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

    pbr.world_position = mesh.world_position;
    pbr.world_normal = fns::prepare_world_normal(mesh.world_normal, double_sided, is_front);
    pbr.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr.N = normalize(pbr.world_normal);
    pbr.V = fns::calculate_view(mesh.world_position, pbr.is_orthographic);

    // Base flat color
    var col = water_color.rgb;

    // Camera-relative UVs (prevents huge-coordinate precision weirdness)
    let rel = mesh.world_position.xyz - view.world_position;
    let uv = rel.xz;

    // Controls
    let scale = max(swirl_params.x, 0.0001);
    let speed = swirl_params.y;
    let mix_strength = saturate(swirl_params.z);
    let foam_strength = saturate(swirl_params.w);

    let t = time * speed;

    // Two animated band layers (super cheap)
    let a = sin((uv.x * 0.18 + uv.y * 0.08) * scale + t);
    let b = sin((uv.x * -0.10 + uv.y * 0.16) * (scale * 0.9) - t * 0.7);

    // Combine into 0..1
    let bands = saturate(0.5 + 0.5 * (a * 0.75 + b * 0.55));

    // Color variation (like painted layers)
    let tint = mix(mix_color_a.rgb, mix_color_b.rgb, bands);
    col = mix(col, col * tint, mix_strength * 0.75);

    // Ink rim darkening (linear)
    let NoV = saturate(dot(pbr.N, pbr.V));
    let rim = saturate((1.0 - NoV) * 1.1);
    col *= (1.0 - rim * 0.35);

    // Foam-ish highlights: brighten band crests + a touch on rim
    let crest = smoothstep(0.72, 0.96, bands);
    let foam = saturate(crest * 0.85 + rim * 0.20) * foam_strength;
    col = mix(col, vec3<f32>(1.0, 1.0, 1.0), foam * 0.55);

    // Slight posterization for "gamey" look
    let steps = 6.0;
    col = floor(col * steps) / steps;

    // Opacity: stable constant (still translucent)
    let alpha = saturate(water_color.a);

    return vec4<f32>(col, alpha);
}
