// Filled body + dynamic3D elements + opaque analytic grain geometry, one optical pass.
import bevy_pbr::render::forward_io::{Vertex,decompress_vertex};
import bevy_pbr::render::mesh_view_bindings::view;
import bevy_pbr::render::view_transformations::{position_ndc_to_world,frag_coord_to_ndc};
import bevy_core_pipeline::tonemapping::tone_mapping;
@if(DEPTH_PREPASS) import bevy_pbr::prepass::utils::prepass_depth;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var shoulder:texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var column:texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var swept:texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var billow:texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var curl:texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var trail:texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var cluster:texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(7) var linear_sampler:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(8) var<storage,read> scene:array<vec4<f32>>;
@group(constants::MATERIAL_BIND_GROUP) @binding(9) var composed:texture_3d<f32>;
struct Out{@builtin(position) position:vec4<f32>}
@vertex fn vertex(packed:Vertex)->Out {let v=decompress_vertex(packed,packed.instance_index);var o:Out;o.position=vec4(v.position.xy*2.0,0.0,1.0);return o;}
// Same capsule/box signed distances as interaction::Interactor::query.
fn terrain_occupancy(p:vec3<f32>)->f32 {
 let base=u32(scene[21].w);if base==0u{return 1.0;}
 let info=scene[base];let nx=u32(info.w);let nz=u32(scene[base+1u].x);let q=(p.xz-info.xy)/info.z;
 if any(q<vec2(0.0)) || q.x>f32(nx-1u) || q.y>f32(nz-1u){return 1.0;}
 let c=min(vec2<u32>(floor(q)),vec2(nx-2u,nz-2u));let f=q-vec2<f32>(c);
 let i=base+2u+c.x+c.y*nx;let a=scene[i];let b=scene[i+1u];let d=scene[i+nx];let e=scene[i+nx+1u];
 if min(min(a.y,b.y),min(d.y,e.y))<0.5{return 1.0;}
 let h=mix(mix(a.x,b.x,f.x),mix(d.x,e.x,f.x),f.y);
 return smoothstep(0.0,scene[21].z,p.y-h);
}
fn occupancy(p:vec3<f32>)->f32 {
 var mask=terrain_occupancy(p);
 for(var i=0u;i<u32(scene[21].x);i+=1u){
  let base=u32(scene[21].y)+4u*i;let v=vec4(p,1.0);
  let q=vec3(dot(scene[base],v),dot(scene[base+1u],v),dot(scene[base+2u],v));let shape=scene[base+3u];var d=0.0;
  if shape.w<0.5 {let delta=q-vec3(0.0,clamp(q.y,-shape.y,shape.y),0.0);d=length(delta)-shape.x;}
  else {let delta=abs(q)-shape.xyz;d=length(max(delta,vec3(0.0)))+min(max(delta.x,max(delta.y,delta.z)),0.0);}
  mask=min(mask,smoothstep(0.0,scene[21].z,d));
 } return mask;
}
fn word(base:u32,index:u32)->u32 {return u32(scene[base+index/4u][index%4u]);}
fn row(index:u32,offset:u32)->vec4<f32>{return scene[u32(scene[0].y)+7u*index+offset];}
fn local_point(index:u32,p:vec3<f32>)->vec3<f32>{let q=vec4(p,1.0);return vec3(dot(row(index,0u),q),dot(row(index,1u),q),dot(row(index,2u),q));}
fn local_direction(index:u32,p:vec3<f32>)->vec3<f32>{let q=vec4(p,0.0);return vec3(dot(row(index,0u),q),dot(row(index,1u),q),dot(row(index,2u),q));}
fn bin_at(p:vec3<f32>)->u32{
 let dims=vec3<u32>(scene[3].xyz);let uv=clamp((p-scene[1].xyz)/(scene[2].xyz-scene[1].xyz),vec3(0.0),vec3(0.999999));let c=vec3<u32>(floor(uv*scene[3].xyz));return c.x+dims.x*(c.y+dims.y*c.z);
}
fn density(p:vec3<f32>)->f32 {
 if any(p<scene[1].xyz) || any(p>scene[2].xyz){return 0.0;}
 let physical=vec3<f32>(textureDimensions(composed));let used_dims=scene[6].yzw;
 let uv=(p-scene[1].xyz)/(scene[2].xyz-scene[1].xyz)*used_dims/physical;
 let rho=textureSampleLevel(composed,linear_sampler,clamp(uv,vec3(0.5)/physical,(used_dims-0.5)/physical),0.0).rg;
 // Reapply the exact solid mask to interactive density after trilinear sampling:
 // interpolation must not leak media into a collider. Reference contribution stays intact.
 if scene[21].x<0.5 && scene[21].w<0.5 {return rho.x;}
 return rho.y+max(0.0,rho.x-rho.y)*occupancy(p);
}
fn safe_direction(rd:vec3<f32>)->vec3<f32>{return select(select(vec3(-0.00000001),vec3(0.00000001),rd>=vec3(0.0)),rd,abs(rd)>vec3(0.00000001));}
fn interval(ro:vec3<f32>,rd:vec3<f32>,lo:vec3<f32>,hi:vec3<f32>)->vec2<f32>{let a=(lo-ro)/safe_direction(rd);let b=(hi-ro)/safe_direction(rd);let l=min(a,b);let h=max(a,b);return vec2(max(l.x,max(l.y,l.z)),min(h.x,min(h.y,h.z)));}
fn cell_exit(p:vec3<f32>,rd:vec3<f32>)->f32{
 let size=(scene[2].xyz-scene[1].xyz)/scene[3].xyz;let c=floor((p-scene[1].xyz)/size);let edge=scene[1].xyz+(c+select(vec3(0.0),vec3(1.0),rd>=vec3(0.0)))*size;
 let t=(edge-p)/safe_direction(rd);return max(0.00001,min(t.x,min(t.y,t.z)));
}
struct GrainHit{distance:f32,color:vec3<f32>}
fn grains(ro:vec3<f32>,rd:vec3<f32>,start:f32,limit:f32)->GrainHit{
 var hit:GrainHit;hit.distance=limit;hit.color=vec3(0.0);var t=start;
 for(var k=0u;k<64u;k+=1u){
  if t>=hit.distance{break;}
  let p=ro+rd*(t+0.00001);let bin=bin_at(p);let base=u32(scene[0].z);let begin=word(u32(scene[3].w),bin);let end=word(base,bin+1u);
  for(var j=begin;j<end;j+=1u){let id=word(u32(scene[0].w),j);let parameters=row(id,4u);if parameters.w<1.5{continue;}
   // Exact opaque ellipsoid intersection. Dust in front attenuates it; no media
   // behind an opaque retained grain is visible. No translucent-grain claim.
   let o=local_point(id,ro);let d=local_direction(id,rd);let a=dot(d,d);let b=dot(o,d);let c=dot(o,o)-1.0;let delta=b*b-a*c;
   if delta<0.0 {continue;}
   let entry=(-b-sqrt(delta))/a;let exit=(-b+sqrt(delta))/a;let at=select(entry,exit,entry<start);
   if at>=start && at<hit.distance{
    if row(id,5u).w>0.5 && occupancy(ro+rd*at)<0.5 {continue;}
    let n=normalize(o+d*at);let x=row(id,0u).xyz;let y=row(id,1u).xyz;let z=row(id,2u).xyz;let world_n=normalize(vec3(x.x*n.x+y.x*n.y+z.x*n.z,x.y*n.x+y.y*n.y+z.y*n.z,x.z*n.x+y.z*n.y+z.z*n.z));
    hit.distance=at;hit.color=parameters.xyz*(0.45+0.55*max(0.0,dot(world_n,normalize(vec3(-3.8,5.5,3.0)))));
   }
  }
  t+=cell_exit(p,rd)+0.00002;
 }
 return hit;
}
fn lighting(p:vec3<f32>)->f32{
 let key=normalize(vec3(-3.8,5.5,3.0));var tau=density(p+key*0.18)*0.45;
 if scene[2].w>1.5 {tau=density(p+key*0.06)*0.12+density(p+key*0.18)*0.24+density(p+key*0.40)*0.35;}
 // Per-sample short directional optical probes. Neither whole-ray guide
 // averaging nor offline Mantra transport / physical multiple scattering.
 return 0.28+0.90*exp(-scene[5].w*tau*scene[4].w);
}
@fragment fn fragment(i:Out)->@location(0) vec4<f32>{
 if scene[0].x<0.5{discard;}
 let ro=view.world_position.xyz;let ndc=frag_coord_to_ndc(i.position);let rd=normalize(position_ndc_to_world(vec3(ndc.xy,0.5))-ro);
 let bounds=interval(ro,rd,scene[1].xyz,scene[2].xyz);let start=max(0.0,bounds.x);var end=bounds.y;
 if end<=start{discard;}
 @if(DEPTH_PREPASS) {
  let opaque=prepass_depth(i.position,0u);
  if opaque>0.0 {let world=position_ndc_to_world(vec3(ndc.xy,opaque));end=min(end,max(0.0,dot(world-ro,rd)));}
 }
 if end<=start{discard;}
 let grain=grains(ro,rd,start,end);let grain_visible=grain.distance<end;end=min(end,grain.distance);
 let step=max(scene[1].w,(end-start)/scene[6].x);var t=start;var transmittance=1.0;var color=vec3(0.0);
 for(var k=0u;k<512u;k+=1u){
  if t>=end || transmittance<0.005{break;}
  let ds=min(step,end-t);let sample=ro+rd*(t+0.5*ds);let rho=density(sample);
  if rho>0.00001 {let alpha=1.0-exp(-rho*scene[4].w*ds);color+=transmittance*alpha*scene[4].xyz*lighting(sample);transmittance*=1.0-alpha;}
  t+=ds;
 }
 if grain_visible {color+=transmittance*grain.color;transmittance=0.0;}
 let alpha=1.0-transmittance;if alpha<0.00001{discard;}
 var straight=color/alpha;
 @if(!HDR) straight=tone_mapping(vec4(straight,1.0),view.color_grading).rgb;
 return vec4(straight,alpha);
}
