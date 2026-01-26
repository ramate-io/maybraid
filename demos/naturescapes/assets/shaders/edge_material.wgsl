//---------------------------------------------------------
// Required imports for a PBR fragment shader in Bevy 0.17
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


//---------------------------------------------------------
// Material uniform (simple color)
//---------------------------------------------------------
@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> base_color: vec4<f32>;


//---------------------------------------------------------
// Edge utilities
//---------------------------------------------------------
fn fwidth3(v: vec3<f32>) -> vec3<f32> {
    return abs(dpdx(v)) + abs(dpdy(v));
}

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn depth_at(pos: vec4<f32>) -> f32 {
    return prepass_depth(pos, 0);
}

fn sanitize_depth(d: f32) -> f32 {
    // If depth is basically 0, assume sky/far plane.
    // (Tweak 1e-6 if needed)
    if d < 0.00001 {
        return 1e9;
    }
    return d;
}

// Edge strength from depth *curvature* (second derivative), not slope.
fn depth_edge_laplacian(pos: vec4<f32>, strength: f32) -> f32 {
    let d0 = depth_at(pos);

    let dR = depth_at(pos + vec4<f32>(1.0,  0.0, 0.0, 0.0));
    let dL = depth_at(pos + vec4<f32>(-1.0,  0.0, 0.0, 0.0));
    let dU = depth_at(pos + vec4<f32>( 0.0,  1.0, 0.0, 0.0));
    let dD = depth_at(pos + vec4<f32>( 0.0, -1.0, 0.0, 0.0));

    // Discontinuity measure (2nd derivative)
    let lap = abs((dR + dL + dU + dD) - (4.0 * d0));

    // Normalize by distance so it doesn't vanish far away,
    // and doesn't explode up close.
    let scale = max(0.05, abs(1000.0 * d0));

    // Strength is your artistic knob
    return saturate((lap / scale) * strength);
}


//---------------------------------------------------------
// Fragment Shader
//---------------------------------------------------------
@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput
) -> @location(0) vec4<f32> {

    let visual_depth = prepass_depth(mesh.position, 0);

    //-----------------------------------------------------
    // 1. Build PBR input (same way StandardMaterial does)
    //-----------------------------------------------------
    var pbr_input: PbrInput = pbr_input_new();

    // basic material
    pbr_input.material.base_color = base_color;

    let double_sided = (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

    // basic PBR required fields
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
    // 2. Compute PBR lighting (includes shadows)
    //-----------------------------------------------------
    let lit_color = fns::apply_pbr_lighting(pbr_input);


    //-----------------------------------------------------
    // 3. Depth discontinuity edge detection
    //-----------------------------------------------------
    // Current pixel depth from prepass (view-space depth)
    let edge = depth_edge_laplacian(mesh.position, 2.0); 

    // Invert for intensity multiplier (1 interior, <1 on edges)
    let intensity = 1.0 - edge * 1000.0;

    //-----------------------------------------------------
    // 4. Mix: apply edges on top of PBR lighting
    //-----------------------------------------------------
    let shaded = lit_color.rgb * intensity;
    let final_color = shaded * 0.5 + base_color.rgb * 0.5;

    //-----------------------------------------------------
    // 5. Apply tonemapping, color grading, exposure
    let output = tone_mapping(vec4<f32>(final_color, 1.0), view.color_grading);


    return output;
}
