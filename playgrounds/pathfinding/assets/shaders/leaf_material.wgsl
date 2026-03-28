//---------------------------------------------------------
// Stylized Leaf Material Shader
// 
// This shader creates a stylized, abstract leaf appearance
// suitable for balls of intersecting planes. It uses UV-based
// alpha shaping to create leaf silhouettes from simple planes.
// Uses simple directional lighting with rim lighting.
//---------------------------------------------------------

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
    pbr_bindings,
}
#import bevy_core_pipeline::tonemapping::tone_mapping


//---------------------------------------------------------
// Material Uniforms
//---------------------------------------------------------
@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> base_color: vec4<f32>;


//---------------------------------------------------------
// Perlin Noise
//---------------------------------------------------------
// Classic Perlin noise using gradient vectors at grid points
fn hash22(p: vec2<f32>) -> vec2<f32> {
    let p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let dot_val = dot(p3, p3 + 33.33);
    let p3_xy = vec2<f32>(p3.x, p3.y);
    let p3_yz = vec2<f32>(p3.y, p3.z);
    return fract((p3_xy + p3_yz) * vec2<f32>(dot_val, dot_val * 1.618));
}

// Get gradient vector at grid point
fn grad(p: vec2<f32>) -> vec2<f32> {
    let h = hash22(p);
    // Map to gradient vectors - use angle from hash
    let angle = h.x * 6.28318; // 2 * PI
    return vec2<f32>(cos(angle), sin(angle));
}

fn perlin_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    var f = fract(p);
    
    // Smooth interpolation (smoothstep)
    f = f * f * (3.0 - 2.0 * f);
    
    // Get gradients at the four corners
    let g00 = grad(i);
    let g10 = grad(i + vec2<f32>(1.0, 0.0));
    let g01 = grad(i + vec2<f32>(0.0, 1.0));
    let g11 = grad(i + vec2<f32>(1.0, 1.0));
    
    // Distance vectors from grid points
    let d00 = f;
    let d10 = f - vec2<f32>(1.0, 0.0);
    let d01 = f - vec2<f32>(0.0, 1.0);
    let d11 = f - vec2<f32>(1.0, 1.0);
    
    // Dot products
    let n00 = dot(g00, d00);
    let n10 = dot(g10, d10);
    let n01 = dot(g01, d01);
    let n11 = dot(g11, d11);
    
    // Bilinear interpolation
    let nx0 = mix(n00, n10, f.x);
    let nx1 = mix(n01, n11, f.x);
    return mix(nx0, nx1, f.y);
}

// Fractal Perlin noise (multi-octave)
fn fractal_noise(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 10.0;
    
    // Multiple octaves for fractal detail
    for (var i = 0; i < 5; i++) {
        value += perlin_noise(p * frequency) * amplitude;
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    
    // Perlin noise is in range [-1, 1], normalize to [0, 1]
    return value * 0.5 + 0.5;
}


//---------------------------------------------------------
// Fragment Shader
//---------------------------------------------------------
@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput
) -> @location(0) vec4<f32> {

    //-----------------------------------------------------
    // 1. Calculate leaf shape alpha from noise
    //-----------------------------------------------------
    // Sample noise at UV coordinates
    let noise_scale = 6.0;
    let noise_value = fractal_noise(mesh.uv * noise_scale);
    
    // Threshold: above = visible, below = transparent
    let threshold = 0.5;
    
    // If noise is above threshold, visible, otherwise transparent
    let alpha = step(threshold, noise_value);
    
    // Early exit optimization: if fully transparent, skip lighting
    if (alpha < 0.001) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }


    //-----------------------------------------------------
    // 2. Build PBR input (same way StandardMaterial does)
    //-----------------------------------------------------
    var pbr_input: PbrInput = pbr_input_new();

    // Set base color
    pbr_input.material.base_color = base_color;

    let double_sided = (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

    // Basic PBR required fields
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


    //-----------------------------------------------------
    // 3. Compute PBR lighting (includes shadows)
    //-----------------------------------------------------
    let lit_color = fns::apply_pbr_lighting(pbr_input);


    //-----------------------------------------------------
    // 4. Apply alpha and output
    //-----------------------------------------------------
    let output_color = vec4<f32>(lit_color.rgb, base_color.a * alpha);

    // Apply tonemapping, color grading, exposure
    return tone_mapping(output_color, view.color_grading);
}

