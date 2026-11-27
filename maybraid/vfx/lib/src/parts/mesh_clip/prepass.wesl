import bevy_pbr::prepass::io::{Vertex,decompress_vertex};
import bevy_pbr::render::{mesh_functions,mesh_view_bindings::view};
import super::common::{retained,clip_position};
@if(PREPASS_FRAGMENT)
import bevy_pbr::prepass::io::FragmentOutput;
@if(MOTION_VECTOR_PREPASS)
import bevy_pbr::prepass::bindings::previous_view_uniforms;
struct Out {
 @builtin(position) position:vec4<f32>,
 @location(0) rest:vec3<f32>,
 @if(NORMAL_PREPASS) @location(1) normal:vec3<f32>,
 @if(MOTION_VECTOR_PREPASS) @location(2) world:vec4<f32>,
 @if(MOTION_VECTOR_PREPASS) @location(3) previous_world:vec4<f32>,
 @if(UNCLIPPED_DEPTH_ORTHO_EMULATION) @location(4) unclipped_depth:f32,
}
@vertex fn vertex(packed:Vertex)->Out {
 let i=decompress_vertex(packed,packed.instance_index);var o:Out;
 o.position=clip_position(i.position,i.instance_index);
 o.rest=i.position;
 @if(NORMAL_PREPASS && VERTEX_NORMALS) o.normal=mesh_functions::mesh_normal_local_to_world(i.normal,i.instance_index);
 @if(NORMAL_PREPASS && !VERTEX_NORMALS) o.normal=vec3(0.0,1.0,0.0);
 @if(MOTION_VECTOR_PREPASS) {
  o.world=mesh_functions::get_world_from_local(i.instance_index)*vec4(i.position,1.0);
  o.previous_world=mesh_functions::get_previous_world_from_local(i.instance_index)*vec4(i.position,1.0);
 }
 @if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {o.unclipped_depth=o.position.z;o.position.z=min(o.position.z,1.0);}
 return o;
}
@if(PREPASS_FRAGMENT)
@fragment fn fragment(i:Out)->FragmentOutput {
 if !retained(i.rest) {discard;}
 var o:FragmentOutput;
 @if(NORMAL_PREPASS) o.normal=vec4(normalize(i.normal)*0.5+vec3(0.5),1.0);
 @if(UNCLIPPED_DEPTH_ORTHO_EMULATION) o.frag_depth=i.unclipped_depth;
 @if(MOTION_VECTOR_PREPASS) {
  let a=view.unjittered_clip_from_world*i.world;
  let b=previous_view_uniforms.unjittered_clip_from_world*i.previous_world;
  o.motion_vector=(a.xy/a.w-b.xy/b.w)*vec2(0.5,-0.5);
 }
 return o;
}
@else
@fragment fn fragment(i:Out) {if !retained(i.rest) {discard;}}
