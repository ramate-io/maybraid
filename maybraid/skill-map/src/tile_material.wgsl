//---------------------------------------------------------
// Skill-map tiles: subdivided quads, world-space noise, thematic marks.
// Kind: 0 land, 1 water, 2 fireball, 3 dumbwave.
//---------------------------------------------------------

#import bevy_sprite::{
    mesh2d_functions as mesh_functions,
    mesh2d_vertex_output::VertexOutput,
    mesh2d_view_bindings::{view, globals},
}

struct TileParams {
    // rgb + unused
    tint: vec4<f32>,
    // x kind, y intensity, z seed, w unused
    style: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: TileParams;

const KIND_LAND: f32 = 0.0;
const KIND_WATER: f32 = 1.0;
const KIND_FIRE: f32 = 2.0;
const KIND_WAVE: f32 = 3.0;

fn hash12(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = p3 + vec3<f32>(dot(p3, p3.yzx + 33.33));
    return fract((d.x + d.y) * d.z);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (3.0 - 2.0 * f0);
    let a = hash12(i);
    let b = hash12(i + vec2<f32>(1.0, 0.0));
    let c = hash12(i + vec2<f32>(0.0, 1.0));
    let d = hash12(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    return value_noise(p) * 0.55 + value_noise(p * 2.13) * 0.3 + value_noise(p * 4.27) * 0.15;
}

fn tile_kind() -> f32 { return material.style.x; }
fn tile_seed() -> f32 { return material.style.z; }

struct Vertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
#endif
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_POSITIONS
    var local = vertex.position;
#ifdef VERTEX_UVS
    let uv = vertex.uv;
#else
    let uv = vec2<f32>(0.5, 0.5);
#endif
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let origin = world_from_local[3].xy;
    let t = globals.time + tile_seed() * 0.01;
    let n = fbm(origin * 0.07 + local.xy * 0.18 + vec2<f32>(t * 0.11, -t * 0.08));
    let edge = saturate(length(uv - vec2<f32>(0.5, 0.5)) * 2.0);
    let kind = tile_kind();
    if kind < 0.5 {
        local.x += (n - 0.5) * 0.55 * edge;
        local.y += (value_noise(origin * 0.09 + local.xy * 0.2) - 0.5) * 0.45 * edge;
    } else if kind < 1.5 {
        local.x += sin(t * 1.7 + origin.y * 0.2 + uv.y * 6.0) * 0.42;
        local.y += cos(t * 1.3 + origin.x * 0.18 + uv.x * 5.0) * 0.38;
        local.z += (n - 0.5) * 0.2;
    } else if kind < 3.5 {
        let pulse = 0.5 + 0.5 * sin(t * 3.1 + origin.x * 0.15);
        let scale = 1.0 + (pulse - 0.5) * 0.06 * (1.0 - edge);
        let wobble = (n - 0.5) * 0.28 * edge;
        local.x = local.x * scale + wobble;
        local.y = local.y * scale + wobble;
    } else {
        let pulse = 0.5 + 0.5 * sin(t * 5.2);
        let scale = 1.0 + (pulse - 0.5) * 0.08;
        local.x = local.x * scale;
        local.y = local.y * scale;
    }
    out.world_position = mesh_functions::mesh2d_position_local_to_world(
        world_from_local,
        vec4<f32>(local, 1.0),
    );
    out.position = mesh_functions::mesh2d_position_world_to_clip(out.world_position);
#endif
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh2d_normal_local_to_world(
        vertex.normal,
        vertex.instance_index,
    );
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh2d_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
    return out;
}

fn shade_land(uv: vec2<f32>, world: vec2<f32>) -> vec3<f32> {
    let n = fbm(world * 0.11 + uv * 1.6);
    let n2 = value_noise(world * 0.31 + uv * 3.4);
    var dirt = vec3<f32>(0.38, 0.24, 0.13);
    let dry = vec3<f32>(0.58, 0.42, 0.24);
    let moss = vec3<f32>(0.22, 0.28, 0.14);
    var color = mix(dirt, dry, n);
    color = mix(color, moss, smoothstep(0.62, 0.82, n2) * 0.35);
    let rim = saturate(length(uv - vec2<f32>(0.5, 0.5)) * 1.85);
    color *= 1.08 - rim * 0.38;
    return color;
}

fn shade_water(uv: vec2<f32>, world: vec2<f32>) -> vec3<f32> {
    let t = globals.time;
    let n = fbm(world * 0.09 + uv * 2.2 + vec2<f32>(t * 0.18, -t * 0.14));
    let band = 0.5 + 0.5 * sin(world.x * 0.35 + world.y * 0.22 + t * 1.6 + n * 4.0);
    let deep = vec3<f32>(0.05, 0.12, 0.28);
    let shallow = vec3<f32>(0.14, 0.38, 0.58);
    let foam = vec3<f32>(0.62, 0.82, 0.88);
    var color = mix(deep, shallow, n);
    color = mix(color, foam, smoothstep(0.78, 0.94, band) * 0.45);
    let rim = saturate(length(uv - vec2<f32>(0.5, 0.5)) * 1.7);
    color *= 1.05 - rim * 0.28;
    return color;
}

fn shade_fire(uv: vec2<f32>, world: vec2<f32>) -> vec3<f32> {
    var color = shade_land(uv, world);
    let q = uv - vec2<f32>(0.5, 0.5);
    let t = globals.time + tile_seed() * 0.02;
    let warp = fbm(world * 0.2 + q * 4.0 + vec2<f32>(t * 0.4, -t * 0.3));
    let r = length(q + vec2<f32>(warp - 0.5) * 0.18);
    let ember = 1.0 - smoothstep(0.16, 0.38, r);
    let core = 1.0 - smoothstep(0.0, 0.16, r);
    let coal = vec3<f32>(0.18, 0.05, 0.02);
    let mid = vec3<f32>(0.95, 0.32, 0.06);
    let hot = vec3<f32>(1.0, 0.86, 0.45);
    color = mix(color, coal, ember * 0.85);
    color = mix(color, mid, ember);
    color = mix(color, hot, core * core);
    return color * (1.0 + core * 0.35);
}

fn shade_wave(uv: vec2<f32>, world: vec2<f32>) -> vec3<f32> {
    var color = shade_land(uv, world) * vec3<f32>(0.72, 0.8, 0.9);
    let q = uv - vec2<f32>(0.5, 0.5);
    let t = globals.time * 1.4 + tile_seed() * 0.03;
    let r = length(q);
    let rings = 0.5 + 0.5 * sin(r * 28.0 - t * 4.2);
    let disk = 1.0 - smoothstep(0.18, 0.46, r);
    let glow = vec3<f32>(0.28, 0.82, 0.9);
    let mist = vec3<f32>(0.72, 0.9, 0.95);
    color = mix(color, glow, disk * 0.55);
    color = mix(color, mist, smoothstep(0.55, 0.9, rings) * disk);
    return color;
}

fn shade_cursor(uv: vec2<f32>) -> vec3<f32> {
    let q = uv - vec2<f32>(0.5, 0.5);
    let r = length(q);
    let core = 1.0 - smoothstep(0.0, 0.34, r);
    let ring = smoothstep(0.42, 0.5, r) * (1.0 - smoothstep(0.54, 0.7, r));
    let ivory = vec3<f32>(1.0, 0.93, 0.78);
    let gold = vec3<f32>(1.0, 0.62, 0.18);
    return ivory * (0.35 + core * 0.9) + gold * ring;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let kind = tile_kind();
    let world = in.world_position.xy;
    var color = shade_land(in.uv, world);
    if kind > 0.5 && kind < 1.5 {
        color = shade_water(in.uv, world);
    } else if kind > 1.5 && kind < 2.5 {
        color = shade_fire(in.uv, world);
    } else if kind > 2.5 && kind < 3.5 {
        color = shade_wave(in.uv, world);
    } else if kind > 3.5 {
        color = shade_cursor(in.uv);
    }
    color *= material.tint.xyz * material.style.y;
    return vec4<f32>(color, 1.0);
}
