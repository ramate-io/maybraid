@group(0) @binding(0) var density:texture_3d<f32>;
@group(0) @binding(1) var samp:sampler;
@group(0) @binding(2) var<storage,read> scene:array<vec4<f32>>;
@group(0) @binding(3) var output:texture_storage_3d<rgba16float,write>;
fn rho(p:vec3<f32>)->f32{if any(p<scene[0].xyz)||any(p>scene[1].xyz){return 0.0;}let uv=(p-scene[0].xyz)/(scene[1].xyz-scene[0].xyz)*scene[2].xyz/256.0;return textureSampleLevel(density,samp,clamp(uv,vec3(0.5)/256.0,(scene[2].xyz-0.5)/256.0),0.0).r;}
@compute @workgroup_size(4,4,4) fn light(@builtin(global_invocation_id) id:vec3<u32>){let dims=vec3<u32>(scene[2].xyz);if any(id>=dims){return;}let p=scene[0].xyz+(vec3<f32>(id)+0.5)*scene[2].w;let value=textureLoad(density,vec3<i32>(id),0).r;if value<0.000001{textureStore(output,vec3<i32>(id),vec4(0.0));return;}
 let world_key=normalize(vec3(-3.8,5.5,3.0));let key=normalize(vec3(dot(scene[4].xyz,world_key),dot(scene[5].xyz,world_key),dot(scene[6].xyz,world_key)));let ds=scene[2].w;var tau=0.0;var pos=p+key*ds*0.5;
 for(var i=0u;i<512u;i+=1u){if any(pos<scene[0].xyz)||any(pos>scene[1].xyz){break;}tau+=rho(pos)*3.0*ds;pos+=key*ds;}
 let ex=vec3(ds,0.0,0.0);let ey=vec3(0.0,ds,0.0);let ez=vec3(0.0,0.0,ds);let grad=vec3(rho(p+ex)-rho(p-ex),rho(p+ey)-rho(p-ey),rho(p+ez)-rho(p-ez))/(2.0*ds);let lengthgrad=length(grad);var facing=0.0;if lengthgrad>0.000001{facing=max(0.0,dot(-grad/lengthgrad,key));}let edge=smoothstep(0.0,1.0,lengthgrad/7.0);let form=(1.0-0.3*edge)+0.3*edge*(0.3+0.7*facing);let illumination=(0.28+0.90*exp(-0.9*tau))*form;
 textureStore(output,vec3<i32>(id),vec4(value,illumination,0.0,0.0));}
