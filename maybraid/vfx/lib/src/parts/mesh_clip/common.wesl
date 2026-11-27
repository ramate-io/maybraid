@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> cutoff:vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> seed:vec4<u32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> light:vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> dark:vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var<uniform> mid:vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var<uniform> bright:vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var<uniform> thresholds:vec4<f32>;
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

fn retained(rest:vec3<f32>)->bool{return dissolve_noise(rest,seed.x)>cutoff.x;}
fn clip_position(rest:vec3<f32>,index:u32)->vec4<f32>{
 return bevy_pbr::render::view_transformations::position_world_to_clip((bevy_pbr::render::mesh_functions::get_world_from_local(index)*vec4(rest,1.0)).xyz);
}
