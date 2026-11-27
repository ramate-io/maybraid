import bevy_pbr::render::forward_io::{Vertex,decompress_vertex};
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;
import super::common::{retained,clip_position,light,dark,mid,bright,thresholds};
struct Out { @builtin(position) position:vec4<f32>, @location(0) rest:vec3<f32>, @location(1) normal:vec3<f32> }
@vertex fn vertex(packed:Vertex)->Out {
 let i=decompress_vertex(packed,packed.instance_index);var o:Out;
 o.position=clip_position(i.position,i.instance_index);
 o.rest=i.position;
 @if(VERTEX_NORMALS) o.normal=mesh_functions::mesh_normal_local_to_world(i.normal,i.instance_index);
 @if(!VERTEX_NORMALS) o.normal=vec3(0.0,1.0,0.0);
 return o;
}
@fragment fn fragment(i:Out)->@location(0) vec4<f32>{
 if !retained(i.rest) {discard;}
 let d=dot(normalize(i.normal),normalize(light.xyz));
 var c=dark.xyz;if d>thresholds.x{c=mid.xyz;}if d>thresholds.y{c=bright.xyz;}
 return vec4(c,1.0);
}
