@group(0) @binding(0) var field0:texture_3d<f32>;
@group(0) @binding(1) var field1:texture_3d<f32>;
@group(0) @binding(2) var field2:texture_3d<f32>;
@group(0) @binding(3) var field3:texture_3d<f32>;
@group(0) @binding(4) var field4:texture_3d<f32>;
@group(0) @binding(5) var field5:texture_3d<f32>;
@group(0) @binding(6) var field6:texture_3d<f32>;
@group(0) @binding(7) var field7:texture_3d<f32>;
@group(0) @binding(8) var field8:texture_3d<f32>;
@group(0) @binding(9) var field9:texture_3d<f32>;
@group(0) @binding(10) var field10:texture_3d<f32>;
@group(0) @binding(11) var field11:texture_3d<f32>;
@group(0) @binding(12) var field12:texture_3d<f32>;
@group(0) @binding(13) var field13:texture_3d<f32>;
@group(0) @binding(14) var field14:texture_3d<f32>;
@group(0) @binding(15) var field15:texture_3d<f32>;
@group(0) @binding(16) var samp:sampler;
@group(0) @binding(17) var<storage,read> scene:array<vec4<f32>>;
@group(0) @binding(18) var output:texture_storage_3d<r16float,write>;
fn S(x:f32)->f32{let v=clamp(x,0.0,1.0);return v*v*(3.0-2.0*v);}
fn noise(p:vec3<f32>,seed:f32)->f32{let phi=seed*0.000219;return 0.5+0.22*sin(dot(p,vec3(2.1,1.7,0.9))+phi)+0.16*sin(dot(p,vec3(-1.4,3.2,2.6))+1.71*phi)+0.10*sin(dot(p,vec3(4.7,-0.8,3.1))+0.63*phi);}
fn rest(index:u32,p:vec3<f32>)->f32{let lo=scene[12u+2u*index].xyz;let hi=scene[13u+2u*index].xyz;if any(p<lo)||any(p>hi){return 0.0;}let uv=(p-lo)/(hi-lo);switch index{
case 0u:{return textureSampleLevel(field0,samp,uv,0.0).r;}
case 1u:{return textureSampleLevel(field1,samp,uv,0.0).r;}
case 2u:{return textureSampleLevel(field2,samp,uv,0.0).r;}
case 3u:{return textureSampleLevel(field3,samp,uv,0.0).r;}
case 4u:{return textureSampleLevel(field4,samp,uv,0.0).r;}
case 5u:{return textureSampleLevel(field5,samp,uv,0.0).r;}
case 6u:{return textureSampleLevel(field6,samp,uv,0.0).r;}
case 7u:{return textureSampleLevel(field7,samp,uv,0.0).r;}
case 8u:{return textureSampleLevel(field8,samp,uv,0.0).r;}
case 9u:{return textureSampleLevel(field9,samp,uv,0.0).r;}
case 10u:{return textureSampleLevel(field10,samp,uv,0.0).r;}
case 11u:{return textureSampleLevel(field11,samp,uv,0.0).r;}
case 12u:{return textureSampleLevel(field12,samp,uv,0.0).r;}
case 13u:{return textureSampleLevel(field13,samp,uv,0.0).r;}
case 14u:{return textureSampleLevel(field14,samp,uv,0.0).r;}
case 15u:{return textureSampleLevel(field15,samp,uv,0.0).r;}
default:{return 0.0;}}}
fn local(b:u32,o:u32,p:vec3<f32>)->vec3<f32>{let q=vec4(p,1.0);return vec3(dot(scene[b+o],q),dot(scene[b+o+1u],q),dot(scene[b+o+2u],q));}
fn primitive(id:u32,p:vec3<f32>)->vec3<f32>{let b=44u+10u*id;let kind=u32(scene[b+9u].w);let r=local(b,0u,p);
 if kind==0u{let a=scene[b+5u];let value=S(rest(u32(a.x),r)/0.18)*(1.0+0.32*(2.0*noise(scene[b+3u].xyz+scene[b+4u].xyz*r,a.y)-1.0))*a.z;return vec3(value,0.0,0.0);}
 if kind==1u{let center=scene[b+6u];let anchor=scene[b+7u];let parent=local(b,3u,p);let v=max(0.0,1.0-dot(r,r));let value=center.w*v*v*pow(0.35+0.65*noise((parent-center.xyz+anchor.xyz)*2.8,anchor.w),2.0)*S(max(p.y,0.0)/0.035);return vec3(0.0,value,0.0);}
 let y=r.y;if y<0.0||y>1.0{return vec3(0.0);}let hw=0.03*pow(1.0-y,0.9);if hw<=0.00000001{return vec3(0.0);}let rn=length(vec2(r.x/hw,r.z/(0.75*hw)));let value=7.0*S(1.0-rn)*S((1.0-scene[b+6u].x-y)/0.08);return vec3(value,0.0,value);
}
@compute @workgroup_size(4,4,4) fn compose(@builtin(global_invocation_id) id:vec3<u32>){let dims=vec3<u32>(scene[2].xyz);if any(id>=dims){return;}let p=scene[0].xyz+(vec3<f32>(id)+0.5)*scene[2].w;let uv=clamp((p-scene[0].xyz)/(scene[1].xyz-scene[0].xyz),vec3(0.0),vec3(0.999999));let cell=vec3<u32>(uv*16.0);let bin=u32(scene[3].y)+cell.x+16u*(cell.y+16u*cell.z);let range=scene[bin].xy;var sums=vec3(0.0);
 for(var i=u32(range.x);i<u32(range.y);i+=1u){let id=u32(scene[u32(scene[3].z)+i/4u][i%4u]);sums+=primitive(id,p);}
 var rho=2.2*(1.0-exp(-(sums.x+sums.y)/2.2));let mode=u32(scene[1].w);if mode==1u{rho=2.2*(1.0-exp(-sums.x/2.2));}if mode==2u{rho=sums.y;}if mode==3u{rho=2.2*(1.0-exp(-sums.z/2.2));}if mode==4u&&p.z>0.0{rho=0.0;}textureStore(output,vec3<i32>(id),vec4(rho,0.0,0.0,0.0));}
