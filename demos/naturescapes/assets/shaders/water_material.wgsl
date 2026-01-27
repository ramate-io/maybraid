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

  return water_color;
}
