import bevy_pbr::render::forward_io::{Vertex,decompress_vertex};
import bevy_pbr::render::mesh_functions;
import bevy_pbr::render::view_transformations::position_world_to_clip;
struct Environment{map:mat4x4<f32>,normal:mat4x4<f32>,colliders:array<vec4<f32>,12>,settings:vec4<f32>}
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> environment:Environment;
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> cutoff:vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> seed:vec4<u32>;
// Existing material ABI: W=1 selects wave controls in the cutoff slot.
// Landing keeps its original threshold and hash seed, with identity deformation.
fn controls()->vec4<f32>{return select(vec4(0.0),cutoff,seed.w==1u);}

// Delivered local map, before instance scale/rotation/translation.
fn deform(p:vec3<f32>)->vec3<f32>{
 let a=(p.x-0.40)/0.65;let b=(p.x+0.55)/0.60;let z=p.z/0.85;
 let f=exp(-a*a-z*z)-0.6*exp(-b*b-z*z);
 let deformation=controls();
 let y=p.y*exp(deformation.x*f);let hy=y/1.25;let qy=(y-0.75)/0.65;
 let h=y*exp(-0.5*hy*hy);let q=exp(-qy*qy);
 return vec3(p.x+deformation.y*h,y,p.z*exp(deformation.z*q)+deformation.w*h);
}
fn analytic_normal(p:vec3<f32>,n:vec3<f32>)->vec3<f32>{
 let d=controls();let a=(p.x-0.40)/0.65;let b=(p.x+0.55)/0.60;let z=p.z/0.85;
 let f1=exp(-a*a-z*z);let f2=exp(-b*b-z*z);let f=f1-0.6*f2;
 let fx=-2.0*a/0.65*f1+1.2*b/0.60*f2;let fz=-2.0*p.z/(0.85*0.85)*f;let ey=exp(d.x*f);let y=p.y*ey;let dy=vec3(y*d.x*fx,ey,y*d.x*fz);
 let hy=y/1.25;let hp=exp(-0.5*hy*hy)*(1.0-hy*hy);let qy=(y-0.75)/0.65;let q=exp(-qy*qy);let qp=q*(-2.0*(y-0.75)/(0.65*0.65));let ez=exp(d.z*q);let k=p.z*ez*d.z*qp+d.w*hp;
 let gx=vec3(1.0,0.0,0.0)+dy*(d.y*hp);let gz=vec3(0.0,0.0,ez)+dy*k;
 let a0=vec3(gx.x,dy.x,gz.x);let b0=vec3(gx.y,dy.y,gz.y);let c0=vec3(gx.z,dy.z,gz.z);
 return normalize(mat3x3(cross(b0,c0),cross(c0,a0),cross(a0,b0))*n/dot(a0,cross(b0,c0)));
}
fn deformed_normal(p:vec3<f32>,n:vec3<f32>)->vec3<f32>{
 if all(controls()==vec4(0.0)){return n;}
 if seed.z==2u{return analytic_normal(p,n);}
 let e=0.0005;
 let a=(deform(p+vec3(e,0.0,0.0))-deform(p-vec3(e,0.0,0.0)))/(2.0*e);
 let b=(deform(p+vec3(0.0,e,0.0))-deform(p-vec3(0.0,e,0.0)))/(2.0*e);
 let c=(deform(p+vec3(0.0,0.0,e))-deform(p-vec3(0.0,0.0,e)))/(2.0*e);
 let determinant=dot(a,cross(b,c));
 return normalize(mat3x3(cross(b,c),cross(c,a),cross(a,b))*n/determinant);
}
struct Out { @builtin(position) position:vec4<f32>, @location(0) rest:vec3<f32>, @location(1) normal:vec3<f32>, @location(2) world:vec3<f32> }
@vertex fn vertex(packed:Vertex)->Out {
 let i=decompress_vertex(packed,packed.instance_index);var o:Out;
 let m=mesh_functions::get_world_from_local(i.instance_index);
 o.world=(environment.map*(m*vec4(deform(i.position),1.0))).xyz;
 o.position=position_world_to_clip(o.world);
 o.rest=i.position;
 @if(VERTEX_NORMALS) o.normal=mesh_functions::mesh_normal_local_to_world(deformed_normal(i.position,i.normal),i.instance_index);
 @if(!VERTEX_NORMALS) o.normal=vec3(0.0,1.0,0.0);
 o.normal=normalize((environment.normal*vec4(o.normal,0.0)).xyz);
 return o;
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

@fragment fn fragment(i:Out)->@location(0) vec4<f32>{
 for(var c=0u;c<u32(environment.settings.x);c+=1u){let base=c*4u;let w=vec4(i.world,1.0);let q=vec3(dot(environment.colliders[base],w),dot(environment.colliders[base+1u],w),dot(environment.colliders[base+2u],w));let shape=environment.colliders[base+3u];var d=0.0;if shape.w<0.5{d=length(q-vec3(0.0,clamp(q.y,-shape.y,shape.y),0.0))-shape.x;}else{let v=abs(q)-shape.xyz;d=length(max(v,vec3(0.0)))+min(max(v.x,max(v.y,v.z)),0.0);}if d<0.0{discard;}}
 if seed.w!=1u {if dissolve_noise(i.rest,seed.x)<=cutoff.x {discard;}}
 let d=dot(normalize(i.normal),normalize(vec3(-3.8,5.5,3.0)));
 var c=vec3(0.34,0.19,0.085);if d>0.04{c=vec3(0.58,0.37,0.17);}if d>0.62{c=vec3(0.84,0.59,0.31);}
 return vec4(c,1.0);
}
