import bevy_pbr::render::forward_io::{UncompressedVertex, VertexOutput};
import bevy_pbr::render::view_transformations::position_world_to_clip;

struct PackedTransform {
    @location(5) model0: vec4<f32>,
    @location(6) model1: vec4<f32>,
    @location(7) model2: vec4<f32>,
    @location(8) model3: vec4<f32>,
    @location(9) normal0: vec4<f32>,
    @location(10) normal1: vec4<f32>,
    @location(11) normal2: vec4<f32>,
    @location(12) root0: vec4<f32>,
    @location(13) root1: vec4<f32>,
    @location(14) root2: vec4<f32>,
    @location(15) plant_origin: vec4<f32>,
}

@vertex
fn packed_vertex(vertex: UncompressedVertex, transform: PackedTransform) -> VertexOutput {
    var out: VertexOutput;
    let model = mat4x4<f32>(transform.model0, transform.model1, transform.model2, transform.model3);
    let normal = mat3x3<f32>(transform.normal0.xyz, transform.normal1.xyz, transform.normal2.xyz);
    out.world_position = model * vec4<f32>(vertex.position, 1.0);
    out.position = position_world_to_clip(out.world_position.xyz);
    @if(VERTEX_NORMALS)
    out.world_normal = normalize(normal * vertex.normal);
    @if(VERTEX_UVS_A)
    out.uv = vertex.uv;
    @if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;
    @if(VERTEX_TANGENTS)
    out.world_tangent = vec4<f32>(normalize((model * vec4<f32>(vertex.tangent.xyz, 0.0)).xyz), vertex.tangent.w);
    @if(VERTEX_COLORS)
    out.color = vertex.color;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = 0u;
    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = 0i;
    return out;
}
