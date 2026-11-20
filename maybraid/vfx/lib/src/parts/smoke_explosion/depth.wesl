// Bevy's prepass attributes differ from forward_io (COLOR_0 is at location 7).
import bevy_pbr::prepass::io::{Vertex,decompress_vertex};
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;
struct Surface{centroid:vec4<f32>,opening:vec4<f32>,controls:vec4<f32>,effect_from_world:mat4x4<f32>,tint:vec4<f32>}
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<storage,read> surfaces:array<Surface>;
struct Out{@builtin(position) position:vec4<f32>,@location(0) color:vec4<f32>,@location(1) @interpolate(flat) tag:u32}
@vertex fn vertex(packed:Vertex)->Out{let v=decompress_vertex(packed,packed.instance_index);var o:Out;let m=mesh_functions::get_world_from_local(v.instance_index);o.position=position_world_to_clip((m*vec4(v.position,1.0)).xyz);o.tag=mesh_functions::get_tag(v.instance_index);@if(VERTEX_COLORS)o.color=v.color;@if(!VERTEX_COLORS)o.color=vec4(1.0);return o;}
@fragment fn fragment(i:Out){let data=surfaces[i.tag];if data.controls.w>0.5 && data.controls.w<1.5 && i.color.g<data.controls.x{discard;}}
