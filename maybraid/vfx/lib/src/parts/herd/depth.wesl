// Depth-only prepass for the opaque herd meshes. Dissolution/contact are geometry,
// so this pass never discards a different set of fragments than forward rendering.
import bevy_pbr::prepass::io::{Vertex,VertexOutput,decompress_vertex};
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;
struct Environment{map:mat4x4<f32>,normal:mat4x4<f32>,colliders:array<vec4<f32>,12>,settings:vec4<f32>}
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> environment:Environment;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> cutoff:vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> seed:vec4<u32>;
fn controls()->vec4<f32>{return select(vec4(0.0),cutoff,seed.w==1u);}
fn deform(p:vec3<f32>)->vec3<f32>{
 let a=(p.x-0.40)/0.65;let b=(p.x+0.55)/0.60;let z=p.z/0.85;
 let f=exp(-a*a-z*z)-0.6*exp(-b*b-z*z);
 let deformation=controls();
 let y=p.y*exp(deformation.x*f);let hy=y/1.25;let qy=(y-0.75)/0.65;
 let h=y*exp(-0.5*hy*hy);let q=exp(-qy*qy);
 return vec3(p.x+deformation.y*h,y,p.z*exp(deformation.z*q)+deformation.w*h);
}
@vertex fn vertex(packed:Vertex)->VertexOutput {
 let i=decompress_vertex(packed,packed.instance_index);var o:VertexOutput;
 let m=mesh_functions::get_world_from_local(i.instance_index);
 o.world_position=environment.map*(m*vec4(deform(i.position),1.0));
 o.position=position_world_to_clip(o.world_position.xyz);
 @if(VERTEX_OUTPUT_INSTANCE_INDEX) o.instance_index=i.instance_index;
 return o;
}
