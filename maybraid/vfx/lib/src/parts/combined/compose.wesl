// Rebuild the CURRENT composed density from the seven unchanged rest fields.
@group(0) @binding(0) var shoulder:texture_3d<f32>;
@group(0) @binding(1) var column:texture_3d<f32>;
@group(0) @binding(2) var swept:texture_3d<f32>;
@group(0) @binding(3) var billow:texture_3d<f32>;
@group(0) @binding(4) var curl:texture_3d<f32>;
@group(0) @binding(5) var trail:texture_3d<f32>;
@group(0) @binding(6) var cluster:texture_3d<f32>;
@group(0) @binding(7) var linear_sampler:sampler;
@group(0) @binding(8) var<storage,read> scene:array<vec4<f32>>;
@group(0) @binding(9) var field:texture_storage_3d<rgba16float,write>;
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
fn inverse_deform(p:vec3<f32>,c:vec4<f32>)->vec3<f32>{
 let h0=p.y/1.25;let h=p.y*exp(-0.5*h0*h0);let q0=(p.y-0.75)/0.65;let q=exp(-q0*q0);
 let x=p.x-c.y*h;let z=(p.z-c.w*h)/exp(c.z*q);
 let a=(x-0.40)/0.65;let b=(x+0.55)/0.60;let zz=z/0.85;
 let f=exp(-a*a-zz*zz)-0.6*exp(-b*b-zz*zz);return vec3(x,p.y/exp(c.x*f),z);
}
fn texture_density(index:u32,p:vec3<f32>)->f32{

 let info=scene[7u+2u*index];let uv=(p-info.xyz)*scene[8u+2u*index].xyz;
 if any(uv<vec3(0.0)) || any(uv>vec3(1.0)){return 0.0;}
 var d=0.0;
 switch index {
 case 0u:{d=textureSampleLevel(shoulder,linear_sampler,uv,0.0).r;}
 case 1u:{d=textureSampleLevel(column,linear_sampler,uv,0.0).r;}
 case 2u:{d=textureSampleLevel(swept,linear_sampler,uv,0.0).r;}
 case 3u:{d=textureSampleLevel(billow,linear_sampler,uv,0.0).r;}
 case 4u:{d=textureSampleLevel(curl,linear_sampler,uv,0.0).r;}
 case 5u:{d=textureSampleLevel(trail,linear_sampler,uv,0.0).r;}
 default:{d=textureSampleLevel(cluster,linear_sampler,uv,0.0).r;}
 }return max(0.0,d)*info.w;
}
fn bin_at(p:vec3<f32>)->u32{
 let dims=vec3<u32>(scene[3].xyz);let uv=clamp((p-scene[1].xyz)/(scene[2].xyz-scene[1].xyz),vec3(0.0),vec3(0.999999));let c=vec3<u32>(floor(uv*scene[3].xyz));return c.x+dims.x*(c.y+dims.y*c.z);
}
fn hash3(c:vec3<i32>,seed:u32)->f32 {
 let q=bitcast<vec3<u32>>(c);
 var h=(q.x*0x8da6b343u)^(q.y*0xd8163841u)^(q.z*0xcb1ab31fu)^seed;
 h=h^(h>>16u);h=h*0x7feb352du;h=h^(h>>15u);h=h*0x846ca68bu;h=h^(h>>16u);
 return f32(h&0x00ffffffu)/16777215.0;
}
fn value_noise(p:vec3<f32>,seed:u32)->f32 {
 let c=vec3<i32>(floor(p));let t=fract(p);let f=t*t*t*(t*(t*6.0-15.0)+10.0);
 var out=0.0;
 for(var z=0;z<2;z+=1){for(var y=0;y<2;y+=1){for(var x=0;x<2;x+=1){
 let w=select(1.0-f,f,vec3(x,y,z)==vec3(1));out+=hash3(c+vec3(x,y,z),seed)*w.x*w.y*w.z;
 }}}
 return out;
}
fn dissolve_noise(p:vec3<f32>,seed:u32)->f32 {
 return 0.7*value_noise(2.2*p,seed)+0.3*value_noise(4.4*p+vec3(19.3,-7.1,11.7),seed^0x9e3779b9u);
}

fn density(p:vec3<f32>)->vec2<f32>{
 if any(p<scene[1].xyz) || any(p>scene[2].xyz){return vec2(0.0);}
 let solid=occupancy(p);
 let bin=bin_at(p);let offset=u32(scene[0].z);let begin=word(offset,bin);let end=word(u32(scene[3].w),bin);var sum=0.0;var reference=0.0;
 for(var j=begin;j<end;j+=1u){let id=word(u32(scene[0].w),j);let parameters=row(id,4u);
  if parameters.w>1.5 {continue;}
  if any(p<row(id,5u).xyz) || any(p>row(id,6u).xyz){continue;}
  var q=local_point(id,p);if parameters.w>=0.0 && parameters.w<0.5 {q=inverse_deform(q,row(id,3u));}
  let mask=select(1.0,solid,row(id,5u).w>0.5);
  let raw=texture_density(u32(parameters.x),q);
  if raw<=0.0{continue;}
  var contribution=0.0;
  if parameters.w<0.5 {
   let phase=parameters.y;
   let n=0.5+0.22*sin(dot(q,vec3(2.1,1.7,0.9))+phase)+0.16*sin(dot(q,vec3(-1.4,3.2,2.6))+phase*1.71)+0.10*sin(dot(q,vec3(4.7,-0.8,3.1))+phase*0.63);
   let u=clamp(raw/scene[5].z,0.0,1.0);contribution=u*u*(3.0-2.0*u)*(1.0+scene[5].x*(2.0*n-1.0))*parameters.z*mask;
  }else{if p.y>=0.0 {contribution=raw*parameters.z*mask;}}
  if parameters.w< -0.5 && row(id,3u).x>0.0 {let d=row(id,3u);let seed=u32(d.y)|(u32(d.z)<<16u);contribution*=smoothstep(d.x-0.015,d.x+0.015,dissolve_noise(q,seed));}
  sum+=contribution;if row(id,5u).w<0.5 {reference+=contribution;}
 }
 let rho=scene[5].y*(1.0-exp(-max(0.0,sum)/scene[5].y));
 return vec2(rho,rho*reference/max(sum,0.00000001));
}

@compute @workgroup_size(4,4,4)
fn compose(@builtin(global_invocation_id) id:vec3<u32>){
 let dims=vec3<u32>(scene[6].yzw);if any(id>=dims){return;}
 let p=scene[1].xyz+(vec3<f32>(id)+0.5)/vec3<f32>(dims)*(scene[2].xyz-scene[1].xyz);
 textureStore(field,vec3<i32>(id),vec4(density(p),0.0,0.0));
}
