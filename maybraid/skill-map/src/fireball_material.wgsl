//---------------------------------------------------------
// Discover fireball: cheap capsule, vertex aft-bleed, discarded rim.
// Flight is object +Y; the tail pulls along -Y.
// Phase = globals.time + seed + time_offset so two maps do not boil alike.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    forward_io::Vertex,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}

struct FireballUniform {
    seed: f32,
    intensity: f32,
    speed: f32,
    time_offset: f32,
    radius: f32,
    tail: f32,
    _pad: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: FireballUniform;

struct FireballVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec3<f32>,
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
}

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33, 33.33, 33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn value_noise_3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (vec3<f32>(3.0, 3.0, 3.0) - 2.0 * f0);

    let n000 = hash13(i + vec3<f32>(0.0, 0.0, 0.0));
    let n100 = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash13(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash13(i + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = hash13(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash13(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash13(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash13(i + vec3<f32>(1.0, 1.0, 1.0));

    let nx00 = mix(n000, n100, f.x);
    let nx10 = mix(n010, n110, f.x);
    let nx01 = mix(n001, n101, f.x);
    let nx11 = mix(n011, n111, f.x);
    return mix(mix(nx00, nx10, f.y), mix(nx01, nx11, f.y), f.z);
}

fn phase() -> f32 {
    return globals.time + material.time_offset + material.seed * 0.0013;
}

fn displace_local(local: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let radius = max(material.radius, 1e-3);
    let t = phase();
    let boil = value_noise_3d(local * 2.2 + vec3<f32>(material.seed * 0.01, t * 1.7, material.seed * 0.02));
    let surface = (boil - 0.5) * 0.16 * material.intensity;
    var p = local + normal * surface;

    // Compress the unused forward cap so the hit sphere still reads as the head.
    if p.y > 0.0 {
        let r = length(p);
        if r > 1e-4 {
            p = mix(p, normalize(p) * radius, 0.72);
        }
    }

    // Rear hemisphere bleeds aft along -Y. Faster shots stretch a bit more.
    let aft = saturate(-p.y / radius);
    let speed_k = saturate(material.speed / 30.0);
    let stretch = aft * aft * material.tail * (0.55 + 0.45 * speed_k) * material.intensity;
    p.y -= stretch;
    let pinch = 1.0 - aft * 0.42;
    p.x *= pinch;
    p.z *= pinch;
    return p;
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> FireballVertexOutput {
    var out: FireballVertexOutput;
    let local = displace_local(vertex_no_morph.position, vertex_no_morph.normal);
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(local, 1.0),
    );
    out.position = position_world_to_clip(world_position.xyz);
    out.local_pos = local;
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif
    return out;
}

@fragment
fn fragment(in: FireballVertexOutput) -> @location(0) vec4<f32> {
    let radius = max(material.radius, 1e-3);
    let t = phase();
    let seed = material.seed * 0.01;
    var p = in.local_pos / radius;
    // Domain-warp so the boil slides instead of swimming in place.
    let warp = value_noise_3d(p * 1.7 + vec3<f32>(t * 0.55, seed, -t * 0.4));
    p += vec3<f32>(warp - 0.5) * 0.55;
    let n1 = value_noise_3d(p * 2.05 + vec3<f32>(t * 1.15, seed, -t * 0.7));
    let n2 = value_noise_3d(p * 4.4 - vec3<f32>(t * 1.85, 0.2, seed));
    let n = saturate(n1 * 0.62 + n2 * 0.38);

    // Elongate the radial test so the stretched tail stays thinner than the head.
    let radial = length(vec3<f32>(in.local_pos.x, in.local_pos.y * 0.52, in.local_pos.z)) / radius;
    let rim = n * (1.18 - radial);
    if rim < 0.36 {
        discard;
    }

    let heat = saturate((1.0 - radial) * 1.25 + n * 0.28);
    let core = vec3<f32>(1.0, 0.96, 0.72);
    let mid = vec3<f32>(1.0, 0.36, 0.05);
    let edge = vec3<f32>(0.32, 0.04, 0.01);
    var color = mix(edge, mid, saturate(heat * 1.45));
    color = mix(color, core, saturate(heat * heat * 1.7));
    color *= material.intensity * (1.35 + n * 0.7);
    return vec4<f32>(color, 1.0);
}
