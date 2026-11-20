import bevy_pbr::render::forward_io::{Vertex,decompress_vertex};
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;
struct Surface{centroid:vec4<f32>,opening:vec4<f32>,controls:vec4<f32>,effect_from_world:mat4x4<f32>,tint:vec4<f32>}
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<storage,read> surfaces:array<Surface>;
struct Out{@builtin(position) position:vec4<f32>,@location(0) world:vec3<f32>,@location(1) normal:vec3<f32>,@location(2) color:vec4<f32>,@location(3) @interpolate(flat) tag:u32}
@vertex fn vertex(packed:Vertex)->Out{
 let v=decompress_vertex(packed,packed.instance_index);var o:Out;let m=mesh_functions::get_world_from_local(v.instance_index);
 o.world=(m*vec4(v.position,1.0)).xyz;o.position=position_world_to_clip(o.world);
 @if(VERTEX_NORMALS) o.normal=mesh_functions::mesh_normal_local_to_world(v.normal,v.instance_index);
 @if(!VERTEX_NORMALS) o.normal=vec3(0.0,1.0,0.0);@if(VERTEX_COLORS) o.color=v.color;@if(!VERTEX_COLORS)o.color=vec4(1.0);o.tag=mesh_functions::get_tag(v.instance_index);return o;
}
fn linear(c:vec3<f32>)->vec3<f32>{return select(c/12.92,pow((c+vec3(0.055))/1.055,vec3(2.4)),c>vec3(0.04045));}
@fragment fn fragment(i:Out)->@location(0) vec4<f32>{let data=surfaces[i.tag];
 if data.controls.w>1.5{return vec4(data.tint.xyz*(0.45+0.55*max(0.0,dot(normalize(i.normal),normalize(vec3(-3.8,5.5,3.0))))),1.0);}
 let spike=data.controls.w>0.5;if spike && i.color.g<data.controls.x{discard;}
 let n0=normalize(i.normal);var macro_normal=n0;let delta=i.world-data.centroid.xyz;if !spike && dot(delta,delta)>0.00000001{macro_normal=normalize(delta);}
 let n=normalize(mix(n0,macro_normal,0.4));let key=normalize(vec3(-3.8,5.5,3.0));let m=dot(normalize(mix(n0,macro_normal,0.8)),key);let d=dot(n,key);
 var band=0;if m>0.0{band=1;if d>0.72{band=2;}}
 let cavity=(1.0-data.opening.w)*clamp(-dot(n0,data.opening.xyz),0.0,1.0);if cavity>0.08{band=0;}
 if i.color.r<0.65{band=max(0,band-1);}
 let h=(data.effect_from_world*vec4(i.world,1.0)).y/data.controls.z;
 if h<0.1{band=0;}else if h<0.24{band=min(band,1);}if spike{band=min(band,1);}
 let initial=array<vec3<f32>,3>(linear(vec3(63.0,56.0,51.0)/255.0),linear(vec3(87.0,77.0,70.0)/255.0),linear(vec3(119.0,106.0,97.0)/255.0));
 let settled=array<vec3<f32>,3>(linear(vec3(42.0,36.0,32.0)/255.0),linear(vec3(61.0,51.0,45.0)/255.0),linear(vec3(90.0,76.0,68.0)/255.0));
 return vec4(mix(initial[band],settled[band],data.controls.y)*data.tint.xyz,1.0);
}
