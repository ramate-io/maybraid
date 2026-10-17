//---------------------------------------------------------
// Sustained field looks: descending heavenly columns,
// ground-contact bloom, and propagating shock rings.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> params: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> envelope: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> tint: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var<uniform> extras: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(4)
var<uniform> color_hot: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(5)
var<uniform> color_mid: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(6)
var<uniform> color_cool: vec4<f32>;

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn hash31(p: vec3<f32>) -> f32 {
    return hash21(p.xy + vec2<f32>(p.z * 1.37, p.z * 0.61));
}

fn value_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let n000 = hash31(i);
    let n100 = hash31(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash31(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash31(i + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = hash31(i + vec3<f32>(1.0, 0.0, 1.0));
    let n101 = hash31(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash31(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash31(i + vec3<f32>(1.0, 1.0, 1.0));
    let x00 = mix(n000, n100, u.x);
    let x10 = mix(n010, n110, u.x);
    let x01 = mix(n001, n101, u.x);
    let x11 = mix(n011, n111, u.x);
    return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    var n = 0.0;
    var a = 0.5;
    var q = p;
    for (var i = 0; i < 4; i++) {
        n += value_noise(q) * a;
        q = q * 2.03 + vec3<f32>(11.2, 3.7, 7.1);
        a *= 0.5;
    }
    return n;
}

fn soft_band(v: f32, steps: f32) -> f32 {
    let s = max(steps, 2.0);
    let x = saturate(v) * s;
    let q = floor(x);
    let f = fract(x);
    return (q + smoothstep(0.32, 0.68, f)) / s;
}

fn object_pos(vertex: Vertex) -> vec3<f32> {
#ifdef VERTEX_COLORS
    return vertex.color.xyz;
#else
    return vertex.position;
#endif
}

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
    out.color = vec4<f32>(object_pos(vertex), 1.0);
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

fn column_shade(local: vec3<f32>, n: vec3<f32>, view_dir: vec3<f32>, sheath: bool) -> vec4<f32> {
    let height = saturate(local.y);
    let front = 1.0 - envelope.y;
    let reveal = smoothstep(front - 0.05, front + 0.03, height);
    let radial = saturate(length(local.xz) * 2.0);
    let angle = atan2(local.z, local.x);
    let scroll = params.x * extras.y + params.z;
    let streaks = fbm(vec3<f32>(angle * 1.8, height * 7.5 - scroll, params.z * 3.1));
    let facing = saturate(dot(n, view_dir));
    let rim = mix(0.35, 1.0, pow(1.0 - facing, 1.6));
    var heat = mix(0.25, 1.05, streaks);
    if sheath {
        heat *= mix(0.35, 0.9, radial);
    } else {
        heat *= mix(1.0, 0.55, radial);
    }
    var rgb = mix(color_cool.xyz, mix(color_mid.xyz, color_hot.xyz, soft_band(heat, extras.w)), heat);
    rgb *= tint.xyz * extras.x * envelope.x;
    let alpha = reveal * envelope.x * mix(0.35, 1.0, streaks) * rim;
    return vec4<f32>(rgb, alpha * (0.55 + extras.z));
}

fn contact_shade(local: vec3<f32>, n: vec3<f32>, view_dir: vec3<f32>) -> vec4<f32> {
    let radial = saturate(length(local.xz) * 2.0);
    let ring = 1.0 - smoothstep(0.28, 1.0, radial);
    let hole = smoothstep(0.0, 0.18, radial);
    let pulse = 0.72 + 0.28 * sin(params.x * extras.y * 6.2 + params.z * 8.0);
    let blotch = fbm(vec3<f32>(local.xz * 4.4, params.z + params.x * 0.35));
    let facing = saturate(dot(n, view_dir));
    var rgb = mix(color_mid.xyz, color_hot.xyz, mix(0.4, 1.0, blotch));
    rgb *= tint.xyz * extras.x * envelope.z * pulse;
    let alpha = envelope.z * envelope.x * ring * hole * mix(0.45, 1.0, blotch) * (0.55 + 0.45 * facing);
    return vec4<f32>(rgb, alpha);
}

fn shock_shade(local: vec3<f32>, n: vec3<f32>, view_dir: vec3<f32>) -> vec4<f32> {
    let radial = saturate(length(local.xy) * 2.0);
    let rim = 1.0 - smoothstep(0.72, 1.0, radial);
    let inner = smoothstep(0.48, 0.82, radial);
    let crest = fbm(vec3<f32>(local.xy * 6.0, params.z + params.x * extras.y));
    let facing = saturate(abs(dot(n, view_dir)));
    var rgb = mix(color_cool.xyz, mix(color_mid.xyz, color_hot.xyz, crest), rim);
    rgb *= tint.xyz * extras.x * envelope.x;
    let alpha = envelope.x * rim * inner * mix(0.4, 1.0, crest) * (0.45 + 0.55 * facing);
    return vec4<f32>(rgb, alpha);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(view.world_position.xyz - mesh.world_position.xyz);
    let n = normalize(mesh.world_normal);
#ifdef VERTEX_COLORS
    let local = mesh.color.xyz;
#else
    let local = vec3<f32>(0.0);
#endif
    var shaded: vec4<f32>;
    if params.w > 2.5 {
        shaded = shock_shade(local, n, view_dir);
    } else if params.w > 1.5 {
        shaded = contact_shade(local, n, view_dir);
    } else if params.w > 0.5 {
        shaded = column_shade(local, n, view_dir, true);
    } else {
        shaded = column_shade(local, n, view_dir, false);
    }
    let mapped = tone_mapping(vec4<f32>(shaded.xyz, 1.0), view.color_grading);
    return vec4<f32>(mapped.rgb, shaded.w);
}
