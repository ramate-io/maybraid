#import bevy_pbr::forward_io::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> base_color: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var albedo_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var albedo_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(albedo_texture, albedo_sampler, mesh.uv);
    return base_color * albedo;
}
