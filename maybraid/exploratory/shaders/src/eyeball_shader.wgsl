#import bevy_pbr::forward_io::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> uv_center: vec2<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> uv_scale: vec2<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var albedo_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var albedo_sampler: sampler;

fn map_uv(raw_uv: vec2<f32>) -> vec2<f32> {
    return (raw_uv - uv_center) * uv_scale + uv_center;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
#ifdef VERTEX_UVS_A
    let raw_uv = mesh.uv;
#else
    let raw_uv = mesh.world_position.xz * 0.25;
#endif
    return textureSample(albedo_texture, albedo_sampler, map_uv(raw_uv));
}
