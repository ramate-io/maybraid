import bevy_pbr::render::forward_io::{Vertex,decompress_vertex};
import bevy_pbr::render::mesh_view_bindings::view;
import bevy_pbr::render::view_transformations::{position_ndc_to_world,frag_coord_to_ndc};
@if(DEPTH_PREPASS) import bevy_pbr::prepass::utils::prepass_depth;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var field:texture_3d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var samp:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<storage,read> scene:array<vec4<f32>>;
struct Out{@builtin(position) position:vec4<f32>}
@vertex fn vertex(packed:Vertex)->Out{let v=decompress_vertex(packed,packed.instance_index);var o:Out;o.position=vec4(v.position.xy*2.0,0.0,1.0);return o;}
fn transform(p:vec3<f32>,w:f32)->vec3<f32>{let v=vec4(p,w);return vec3(dot(scene[4],v),dot(scene[5],v),dot(scene[6],v));}
fn sample(p:vec3<f32>)->vec2<f32>{if any(p<scene[0].xyz)||any(p>scene[1].xyz){return vec2(0.0);}let uv=(p-scene[0].xyz)/(scene[1].xyz-scene[0].xyz)*scene[2].xyz/256.0;return textureSampleLevel(field,samp,clamp(uv,vec3(0.5)/256.0,(scene[2].xyz-0.5)/256.0),0.0).rg;}
@fragment fn fragment(i:Out)->@location(0) vec4<f32>{
 let ndc=frag_coord_to_ndc(i.position);let worldro=view.world_position.xyz;let worldrd=normalize(position_ndc_to_world(vec3(ndc.xy,0.5))-worldro);let ro=transform(worldro,1.0);let raw=transform(worldrd,0.0);let scale=length(raw);let rd=raw/scale;
 let safe=select(select(vec3(-0.00000001),vec3(0.00000001),rd>=vec3(0.0)),rd,abs(rd)>vec3(0.00000001));let a=(scene[0].xyz-ro)/safe;let b=(scene[1].xyz-ro)/safe;let low=min(a,b);let high=max(a,b);let start=max(0.0,max(low.x,max(low.y,low.z)));var end=min(high.x,min(high.y,high.z));
 @if(DEPTH_PREPASS){let depth=prepass_depth(i.position,0u);if depth>0.0{let p=position_ndc_to_world(vec3(ndc.xy,depth));end=min(end,max(0.0,dot(p-worldro,worldrd))*scale);}}
 if end<=start{discard;}let tone=smoothstep(0.1,1.2,scene[0].w);let source=mix(vec3(0.179919195,0.140569070,0.116586304),vec3(0.099716768,0.070487023,0.056377866),tone)*scene[10].xyz;
 var color=vec3(0.0);var trans=1.0;var t=start;let step=scene[2].w;
 for(var k=0u;k<1024u;k+=1u){if t>=end||trans<0.001{break;}let ds=min(step,end-t);let value=sample(ro+rd*(t+0.5*ds));let alpha=1.0-exp(-3.0*value.x*ds);color+=trans*alpha*source*value.y;trans*=1.0-alpha;t+=ds;}
 return vec4(color,1.0-trans);
}
