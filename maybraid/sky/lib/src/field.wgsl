//---------------------------------------------------------
// Discovery sky field: Cosimo-like nebula / star glints on a
// camera-follow dome. Samples view direction so it does not
// parallax. Day weight keeps afternoon readable.
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

fn hash12(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = p3 + vec3<f32>(dot(p3, p3.yzx + 33.33));
    return fract((d.x + d.y) * d.z);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * f0 * (f0 * (f0 * 6.0 - 15.0) + 10.0);
    let a = hash12(i);
    let b = hash12(i + vec2<f32>(1.0, 0.0));
    let c = hash12(i + vec2<f32>(0.0, 1.0));
    let d = hash12(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    return value_noise(p) * 0.5
        + value_noise(p * 1.87) * 0.28
        + value_noise(p * 3.41) * 0.15
        + value_noise(p * 6.13) * 0.07;
}

fn shade_field(dir: vec3<f32>) -> vec3<f32> {
    let elev = dir.y;
    var base: vec3<f32>;
    if elev >= 0.0 {
        let t = elev * elev;
        base = mix(material.horizon.xyz, material.zenith.xyz, t);
    } else {
        base = mix(material.horizon.xyz, material.nadir.xyz, saturate(-elev));
    }

    let t = globals.time;
    let day = material.style.x;
    let swirl_g = material.style.y;
    let star_g = material.style.z;
    let u = atan2(dir.z, dir.x);
    let v = elev;

    // Slow chroma breathe so a paused golden pose is not a dusty still.
    let breathe = 0.5 + 0.5 * sin(t * 0.11 + material.style.w * 6.2831853);
    let clearer = vec3<f32>(0.30, 0.52, 0.82);
    let rose = vec3<f32>(0.80, 0.44, 0.34);
    base = mix(base, clearer, breathe * 0.20 * day);
    base = mix(base, rose, (1.0 - breathe) * 0.12 * day);

    let p = vec2<f32>(u, v) * 2.15;
    let warp = fbm(p + vec2<f32>(t * 0.012, -t * 0.01));
    let n = fbm(p + vec2<f32>(warp, warp * 0.7) * 0.85);
    let n2 = fbm(p * 2.05 + vec2<f32>(-t * 0.016, t * 0.014));

    let swirl_col = mix(vec3<f32>(0.40, 0.20, 0.60), vec3<f32>(0.56, 0.36, 0.74), n2);
    let swirl_w = swirl_g * mix(0.62, 0.14, day) * smoothstep(-0.02, 0.42, elev);
    base = mix(base, swirl_col, n * swirl_w);

    let band = 0.5 + 0.5 * sin(u * 2.3 + v * 3.0 + t * 0.18 + n * 2.0);
    let meridian = 0.5 + 0.5 * sin(u * 0.9 - v * 1.4 + t * 0.09);
    base += vec3<f32>(0.48, 0.28, 0.66) * smoothstep(0.76, 1.0, band) * swirl_w * 0.32;
    base += vec3<f32>(0.32, 0.42, 0.70) * smoothstep(0.86, 1.0, meridian) * swirl_w * 0.16;

    let glint = pow(saturate(n2), 13.0) * (0.5 + 0.5 * sin(t * 1.35 + n * 6.0));
    let glint2 = pow(saturate(n), 16.0);
    let star_w = star_g * mix(1.0, 0.16, day) * smoothstep(0.12, 0.52, elev);
    base += vec3<f32>(0.96, 0.88, 1.0) * glint * star_w;
    base += vec3<f32>(0.72, 0.90, 1.0) * glint2 * star_w * 0.42;

    let sun = normalize(material.sun_dir.xyz + vec3<f32>(1e-5, 0.0, 0.0));
    let sun_d = saturate(dot(dir, sun));
    let sun_vis = smoothstep(-0.10, 0.04, sun.y);
    base += vec3<f32>(1.0, 0.93, 0.68) * pow(sun_d, 720.0) * 2.4 * sun_vis;
    base += vec3<f32>(1.0, 0.70, 0.32) * pow(sun_d, 28.0) * 0.58 * sun_vis;

    let moon = normalize(material.moon_dir.xyz + vec3<f32>(1e-5, 0.0, 0.0));
    let moon_d = saturate(dot(dir, moon));
    let moon_vis = mix(1.0, 0.22, day);
    base += vec3<f32>(0.78, 0.84, 0.96) * pow(moon_d, 260.0) * 1.35 * moon_vis;
    base += vec3<f32>(0.42, 0.50, 0.70) * pow(moon_d, 16.0) * 0.18 * moon_vis;

    return base;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz - view.world_position.xyz);
    return vec4<f32>(shade_field(dir), 1.0);
}
