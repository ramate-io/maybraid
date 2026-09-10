//---------------------------------------------------------
// Fireball fill: default vertex, fire color, no discard.
// Facing term gives a hotter core without a custom interpolator.
//---------------------------------------------------------

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> base_color: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(mesh.world_normal);
    let v = normalize(view.world_position.xyz - mesh.world_position.xyz);
    let facing = saturate(dot(n, v));
    let core = vec3<f32>(1.0, 0.95, 0.62);
    let color = mix(base_color.xyz, core, facing * facing);
    return tone_mapping(vec4<f32>(color * 1.55, 1.0), view.color_grading);
}
