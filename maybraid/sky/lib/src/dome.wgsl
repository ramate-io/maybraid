//---------------------------------------------------------
// Blue dome: 4D blue / haze patches with alpha holes.
// Cosmos sits behind this shell.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
}

struct SkyDomeParams {
    zenith: vec4<f32>,
    horizon: vec4<f32>,
    nadir: vec4<f32>,
    // x day_weight, y peak_alpha, z star_gain, w phase
    style: vec4<f32>,
    sun_dir: vec4<f32>,
    moon_dir: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: SkyDomeParams;

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

fn fbm2(p: vec2<f32>) -> f32 {
    return fbm3(vec3<f32>(p.x, p.y, 1.7));
}

fn haze_cover(t: f32, phase: f32) -> f32 {
    let a = fbm2(vec2<f32>(t * 0.006 + phase * 3.0, 2.17));
    let b = fbm2(vec2<f32>(-t * 0.004 + phase, 8.41));
    return mix(0.28, 0.78, smoothstep(0.32, 0.62, a * 0.6 + b * 0.4));
}

fn haze_island(dir: vec3<f32>, t: f32) -> f32 {
    let p = dir * 1.45 + vec3<f32>(t * 0.012, -t * 0.01, t * 0.008);
    let warp = fbm3(p);
    let n = fbm3(p * 1.1 + vec3<f32>(warp * 0.55, t * 0.009, -warp));
    let n2 = fbm3(p * 1.45 + vec3<f32>(-t * 0.014, warp, t * 0.007));
    return saturate(n * 0.62 + n2 * 0.38);
}

fn cosmos_hole(dir: vec3<f32>, t: f32) -> f32 {
    let p = dir * 1.35 + vec3<f32>(3.7, -1.4, t * 0.011);
    let n = fbm3(p + vec3<f32>(t * 0.01, -t * 0.008, 0.6));
    let n2 = fbm3(p * 1.35 + vec3<f32>(-t * 0.007, t * 0.012, 2.2));
    return saturate(n * 0.58 + n2 * 0.42);
}

/// Sparse pinpoints. High freq + high threshold so they stay dots, not blotches.
fn celestial_dots(dir: vec3<f32>) -> vec4<f32> {
    let s = value_noise3(dir * 72.0);
    let s2 = value_noise3(dir * 80.0 + vec3<f32>(3.1, 1.4, 7.2));
    let star = pow(saturate(s * s2), 10.0);
    let p = value_noise3(dir * 11.0 + vec3<f32>(2.2, 5.1, 0.4));
    let p2 = value_noise3(dir * 12.0 + vec3<f32>(8.0, 0.2, 4.0));
    let planet = pow(saturate(p), 14.0) * step(0.55, p2);
    var rgb = mix(vec3<f32>(0.96, 0.92, 1.0), vec3<f32>(0.72, 0.94, 1.0), step(0.5, s2)) * star;
    rgb += mix(vec3<f32>(1.0, 0.68, 0.36), vec3<f32>(0.60, 0.78, 1.0), step(0.5, p2)) * planet * 0.9;
    let mask = step(0.35, max(star, planet));
    return vec4<f32>(rgb, mask);
}

/// Flat paper sun: two rings + a disk. Narrow smoothstep so it is not a photo glow.
fn paper_body(
    dir: vec3<f32>,
    body: vec3<f32>,
    core: f32,
    mid: f32,
    rim: f32,
    core_c: vec3<f32>,
    mid_c: vec3<f32>,
    rim_c: vec3<f32>,
) -> vec4<f32> {
    let d = saturate(dot(dir, body));
    let w = 0.0025;
    var rgb = rim_c * smoothstep(rim - w, rim + w, d);
    rgb = mix(rgb, mid_c, smoothstep(mid - w, mid + w, d));
    rgb = mix(rgb, core_c, smoothstep(core - w, core + w, d));
    let mask = smoothstep(rim - w, rim + w, d);
    return vec4<f32>(rgb, mask);
}

fn shade_dome(dir: vec3<f32>) -> vec4<f32> {
    let elev = dir.y;
    let t = globals.time;
    let day = material.style.x;
    let peak = material.style.y;
    let star_g = material.style.z;
    let phase = material.style.w;

    let sun = normalize(material.sun_dir.xyz + vec3<f32>(1e-5, 0.0, 0.0));
    let sun_body = paper_body(
        dir,
        sun,
        0.9982,
        0.9945,
        0.9860,
        vec3<f32>(1.0, 0.96, 0.72),
        vec3<f32>(1.0, 0.82, 0.38),
        vec3<f32>(1.0, 0.62, 0.22),
    );
    let sun_vis = smoothstep(-0.08, 0.04, sun.y);
    if sun_body.a * sun_vis > 0.04 {
        return vec4<f32>(sun_body.rgb, saturate(sun_body.a * sun_vis));
    }

    let moon = normalize(material.moon_dir.xyz + vec3<f32>(1e-5, 0.0, 0.0));
    let moon_body = paper_body(
        dir,
        moon,
        0.9992,
        0.9980,
        0.9955,
        vec3<f32>(0.92, 0.94, 1.0),
        vec3<f32>(0.72, 0.80, 0.95),
        vec3<f32>(0.48, 0.58, 0.82),
    );
    if moon_body.a > 0.04 {
        return vec4<f32>(moon_body.rgb, moon_body.a);
    }

    let dots = celestial_dots(dir);
    if dots.a * star_g > 0.5 {
        return vec4<f32>(dots.rgb, saturate(max(dots.r, max(dots.g, dots.b))));
    }

    var haze_col = material.horizon.xyz;
    if elev < 0.0 {
        haze_col = mix(material.horizon.xyz, material.nadir.xyz, saturate(-elev * 1.4));
    }
    let blue = material.zenith.xyz;
    let island = haze_island(dir, t);
    let haze = smoothstep(0.40, 0.64, island);
    var rgb = mix(blue, haze_col, haze);
    let stripe = smoothstep(0.18, 0.04, elev) * smoothstep(-0.10, 0.00, elev);
    rgb = mix(rgb, haze_col, stripe * 0.55);

    let cover = haze_cover(t, phase);
    let hole = cosmos_hole(dir, t);
    let presence = mix(0.10, 0.70, day) * mix(0.48, 1.0, cover);
    let open = smoothstep(mix(0.38, 0.55, day), mix(0.62, 0.80, day), hole);
    var alpha = presence * (1.0 - open * mix(0.88, 0.52, day));
    let rim = smoothstep(0.22, -0.06, elev);
    alpha = max(alpha, rim * mix(0.20, 0.48, day));
    alpha = saturate(alpha * peak);

    return vec4<f32>(rgb, alpha);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz - view.world_position.xyz);
    return shade_dome(dir);
}
