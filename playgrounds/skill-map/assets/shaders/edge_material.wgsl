//---------------------------------------------------------
// Required imports for a PBR fragment shader in Bevy 0.17
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


//---------------------------------------------------------
// Fragment Shader
//---------------------------------------------------------
@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput
) -> @location(0) vec4<f32> {

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
    // 3. Strong edge detection using world normals
    //-----------------------------------------------------
    let n = normalize(mesh.world_normal);
    let dN = fwidth3(n);
    let edge_val = length(dN);

    // strong edges
    let edge = smoothstep(0.0001, 0.05, edge_val);

    // invert: 1 → interior, 0 → edge
    let intensity = 1.0 - edge;

    //-----------------------------------------------------
    // 4. Mix: apply edges on top of PBR lighting
    //-----------------------------------------------------
    let shaded = lit_color.rgb * intensity;


    //-----------------------------------------------------
    // 5. Apply tonemapping, color grading, exposure
    //-----------------------------------------------------
    let output = tone_mapping(vec4<f32>(shaded, 1.0), view.color_grading);


    return output;
}
