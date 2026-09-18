//---------------------------------------------------------
// Cosmos backdrop. Opaque Cosimo field behind the blue dome.
// Day does not crush this — the dome's alpha does.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
}

struct SkyFieldParams {
    zenith: vec4<f32>,
    horizon: vec4<f32>,
    nadir: vec4<f32>,
    // x day_weight, y swirl_gain, z star_gain, w phase
    style: vec4<f32>,
    sun_dir: vec4<f32>,
    moon_dir: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: SkyFieldParams;

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var vertex = vertex_no_morph;
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index,
    );
#endif
#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.position = position_world_to_clip(out.world_position.xyz);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index,
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index,
        world_from_local[3],
    );
#endif
    return out;
}

fn hash13(p: vec3<f32>) -> f32 {
    let q = fract(p * 0.1031);
    let d = q + vec3<f32>(dot(q, q.yzx + 33.33));
    return fract((d.x + d.y) * d.z);
}

fn value_noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * f0 * (f0 * (f0 * 6.0 - 15.0) + 10.0);
    let n000 = hash13(i);
    let n100 = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash13(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash13(i + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = hash13(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash13(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash13(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash13(i + vec3<f32>(1.0, 1.0, 1.0));
    let x00 = mix(n000, n100, f.x);
    let x10 = mix(n010, n110, f.x);
    let x01 = mix(n001, n101, f.x);
    let x11 = mix(n011, n111, f.x);
    return mix(mix(x00, x10, f.y), mix(x01, x11, f.y), f.z);
}

fn fbm3(p: vec3<f32>) -> f32 {
    return value_noise3(p) * 0.50
        + value_noise3(p * 1.87) * 0.28
        + value_noise3(p * 3.41) * 0.15
        + value_noise3(p * 6.13) * 0.07;
}

fn poster(n: f32, steps: f32) -> f32 {
    return floor(saturate(n) * steps + 0.5) / steps;
}

fn flatten(n: f32) -> f32 {
    return mix(n, poster(n, 6.0), 0.45);
}

fn celestial_dots(dir: vec3<f32>, t: f32) -> vec3<f32> {
    let drift = vec3<f32>(t * 0.055, -t * 0.042, t * 0.038);
    let s = value_noise3(dir * 72.0 + drift);
    let s2 = value_noise3(dir * 80.0 + vec3<f32>(3.1, 1.4, 7.2) + drift * 0.7);
    let star = pow(saturate(s * s2), 10.0);
    let p = value_noise3(dir * 11.0 + vec3<f32>(2.2, 5.1, 0.4) + drift * 0.35);
    let p2 = value_noise3(dir * 12.0 + vec3<f32>(8.0, 0.2, 4.0) + drift * 0.3);
    let planet = pow(saturate(p), 14.0) * step(0.55, p2);
    var rgb = mix(vec3<f32>(0.96, 0.92, 1.0), vec3<f32>(0.72, 0.94, 1.0), step(0.5, s2)) * star;
    rgb += mix(vec3<f32>(1.0, 0.68, 0.36), vec3<f32>(0.60, 0.78, 1.0), step(0.5, p2)) * planet * 0.9;
    return rgb;
}

fn paper_body(
    dir: vec3<f32>,
    body: vec3<f32>,
    core: f32,
    mid: f32,
    rim: f32,
    core_c: vec3<f32>,
    mid_c: vec3<f32>,
    rim_c: vec3<f32>,
) -> vec3<f32> {
    let d = saturate(dot(dir, body));
    let w = 0.0025;
    var rgb = rim_c * smoothstep(rim - w, rim + w, d);
    rgb = mix(rgb, mid_c, smoothstep(mid - w, mid + w, d));
    rgb = mix(rgb, core_c, smoothstep(core - w, core + w, d));
    return rgb;
}

fn sun_frame(dir: vec3<f32>, sun: vec3<f32>) -> vec2<f32> {
    let d = clamp(dot(dir, sun), -1.0, 1.0);
    var up = vec3<f32>(0.0, 1.0, 0.0);
    if abs(sun.y) > 0.92 {
        up = vec3<f32>(1.0, 0.0, 0.0);
    }
    let tangent = normalize(cross(up, sun));
    let bitan = cross(sun, tangent);
    let radial = dir - sun * d;
    return vec2<f32>(atan2(dot(radial, bitan), dot(radial, tangent)), acos(d));
}

fn logo_shard(id: f32, layer: f32, phi: f32, ang: f32, n: f32, disk: f32) -> f32 {
    let seed = id + layer * 17.3;
    let h = fract(sin(seed * 127.1) * 43758.55);
    let h2 = fract(sin(seed * 269.5 + 1.7) * 23421.63);
    let h3 = fract(sin(seed * 91.3 + 4.2) * 19283.21);
    let gap = mix(0.012, 0.028, h2) + layer * 0.016;
    let thick = mix(0.020, 0.052, h3) * mix(1.0, 0.62, layer);
    let r_mid = disk + gap + thick * 0.5;
    let sector = 6.2831853 / n;
    let center = (id + 0.5 + layer * 0.5) * sector;
    var dphi = phi - center;
    dphi = dphi - 6.2831853 * round(dphi / 6.2831853);
    let x = dphi * r_mid;
    let y = ang - r_mid;
    let twist = mix(-0.22, 0.22, h2);
    let cs = cos(twist);
    let sn = sin(twist);
    let lx = x * cs - y * sn;
    let ly = x * sn + y * cs;
    let point = select(-1.0, 1.0, h > 0.45);
    let sector_half = sector * r_mid * 0.5;
    let half_len = mix(0.72, 0.96, h3) * sector_half * mix(1.0, 0.42, layer);
    let half_wid = mix(0.011, 0.028, h) * mix(1.0, 0.7, layer);
    let along = saturate((lx * point + half_len) / max(2.0 * half_len, 1e-4));
    let halfw = mix(half_wid, 0.0, along);
    let in_x = 1.0 - smoothstep(half_len, half_len + 0.005, abs(lx));
    let in_y = 1.0 - smoothstep(halfw, halfw + 0.004, abs(ly));
    let off_disk = step(disk + 0.006, ang);
    return in_x * in_y * off_disk;
}

fn logo_blotches(dir: vec3<f32>, sun: vec3<f32>, t: f32) -> vec3<f32> {
    let frame = sun_frame(dir, sun);
    let phi = frame.x + t * 0.02;
    let ang = frame.y;
    let disk = 0.168;
    let n = 14.0;
    let base = floor(phi / 6.2831853 * n);
    var fill = 0.0;
    for (var i = -1; i <= 1; i += 1) {
        let id = base + f32(i);
        fill = max(fill, logo_shard(id, 0.0, phi, ang, n, disk));
        fill = max(fill, logo_shard(id, 1.0, phi, ang, n, disk));
    }
    return vec3<f32>(1.0, 0.94, 0.62) * fill;
}

fn shade_cosmos(dir: vec3<f32>) -> vec3<f32> {
    let t = globals.time;
    let swirl_g = material.style.y;
    let star_g = material.style.z;
    let phase = material.style.w;
    let drift = vec3<f32>(t * 0.052, -t * 0.044, t * 0.038) + vec3<f32>(phase * 0.35, 0.0, 0.0);

    let n = flatten(fbm3(dir * 3.4 + drift));
    let n2 = flatten(fbm3(dir * 8.2 + vec3<f32>(-t * 0.048, t * 0.041, 0.4) + drift * 0.6));

    let void_c = vec3<f32>(0.05, 0.02, 0.12);
    let nebula = vec3<f32>(0.28, 0.06, 0.48);
    let bloom = vec3<f32>(0.52, 0.16, 0.72);
    var color = mix(void_c, nebula, n);
    color = mix(color, bloom, smoothstep(0.50, 0.78, n2) * 0.38 * swirl_g);

    let band = smoothstep(0.72, 0.88, 0.5 + 0.5 * sin(dir.x * 2.2 + dir.y * 3.1 + t * 0.35));
    color += vec3<f32>(0.62, 0.22, 0.78) * band * 0.12 * swirl_g;

    color += celestial_dots(dir, t) * star_g;

    let sun = normalize(material.sun_dir.xyz + vec3<f32>(1e-5, 0.0, 0.0));
    let sun_vis = smoothstep(-0.08, 0.04, sun.y);
    let body = paper_body(
        dir,
        sun,
        0.9982,
        0.9945,
        0.9860,
        vec3<f32>(1.0, 0.96, 0.72),
        vec3<f32>(1.0, 0.82, 0.38),
        vec3<f32>(1.0, 0.62, 0.22),
    );
    color += mix(logo_blotches(dir, sun, t), body, step(0.02, max(body.r, max(body.g, body.b)))) * sun_vis;

    let moon = normalize(material.moon_dir.xyz + vec3<f32>(1e-5, 0.0, 0.0));
    color += paper_body(
        dir,
        moon,
        0.9992,
        0.9980,
        0.9955,
        vec3<f32>(0.92, 0.94, 1.0),
        vec3<f32>(0.72, 0.80, 0.95),
        vec3<f32>(0.48, 0.58, 0.82),
    );

    return color;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz - view.world_position.xyz);
    return vec4<f32>(shade_cosmos(dir), 1.0);
}
