//---------------------------------------------------------
// Furniture surfaces: BotW-warm wood / cloth, marble, ornate inlay.
//
// Kind: 0 wood, 1 cloth, 2 soft, 3 marble, 4 ornate.
//---------------------------------------------------------

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_fragment::pbr_input_from_vertex_output,
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct FurnitureSurfaceUniform {
    colors: array<vec4<f32>, 8>,
    noise: vec4<f32>,
    scalars: array<vec4<f32>, 8>,
    rasters: array<array<vec4<f32>, 3>, 8>,
    kind: u32,
    _pad: vec3<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: FurnitureSurfaceUniform;

const KIND_WOOD: u32 = 0u;
const KIND_CLOTH: u32 = 1u;
const KIND_SOFT: u32 = 2u;
const KIND_MARBLE: u32 = 3u;
const KIND_ORNATE: u32 = 4u;

struct Look {
    rgb: vec3<f32>,
    roughness: f32,
    metallic: f32,
    reflectance: f32,
}

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn value_noise_3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (3.0 - 2.0 * f0);

    let n000 = hash13(i);
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

fn fbm(p: vec3<f32>) -> f32 {
    var a = 0.5;
    var s = 0.0;
    var q = p;
    for (var i = 0; i < 5; i++) {
        s += a * value_noise_3d(q);
        q *= 2.11;
        a *= 0.5;
    }
    return s;
}

fn look_coord(mesh: VertexOutput) -> vec3<f32> {
    let freq = max(material.noise.x, 1e-4);
    return mesh.world_position.xyz * freq;
}

fn palette(i: u32) -> vec3<f32> {
    return material.colors[i].xyz;
}

fn palette_or(i: u32, fallback: vec3<f32>) -> vec3<f32> {
    let c = palette(i);
    if (dot(c, c) < 1e-6) {
        return fallback;
    }
    return c;
}

/// Honey-hour lift: keep furniture out of the urban-mud range.
fn warm_bias(rgb: vec3<f32>) -> vec3<f32> {
    return rgb * vec3<f32>(1.10, 1.03, 0.90) + vec3<f32>(0.04, 0.02, 0.0);
}

fn warm_rim(n: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    let v = normalize(view.world_position.xyz - world_pos);
    let rim = pow(1.0 - saturate(abs(dot(normalize(n), v))), 2.4);
    return vec3<f32>(1.0, 0.78, 0.42) * rim * 0.18;
}

fn hemi(n: vec3<f32>) -> vec3<f32> {
    let t = saturate(normalize(n).y * 0.5 + 0.5);
    return mix(vec3<f32>(0.62, 0.42, 0.28), vec3<f32>(1.04, 0.96, 0.82), t);
}

fn finish(rgb: vec3<f32>, n: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    var tint = warm_bias(rgb);
    tint *= mix(vec3<f32>(1.0), hemi(n), 0.16);
    tint += warm_rim(n, world_pos);
    return tint;
}

fn wood_look(p: vec3<f32>) -> Look {
    let base = palette(0u);
    let accent = palette_or(1u, base * vec3<f32>(1.25, 0.85, 0.45));
    let scale = mix(1.4, 2.8, saturate(material.scalars[0].y));
    let mottling = fbm(p * scale);
    let radial = length(p.xz);
    let ring = abs(sin(radial * scale * 16.0 + mottling * 5.2));
    let latewood = pow(ring, 10.0);
    var tint = mix(base, accent, mottling * 0.42);
    tint = mix(tint, tint * vec3<f32>(0.62, 0.48, 0.32), latewood * 0.55);
    tint += vec3<f32>(0.16, 0.09, 0.02) * (1.0 - latewood) * 0.45;
    let roughness = mix(0.36, 0.58, mottling);
    return Look(tint, roughness, 0.0, 0.26);
}

fn cloth_look(p: vec3<f32>) -> Look {
    let base = palette(0u);
    let accent = palette_or(1u, base * vec3<f32>(1.15, 0.7, 1.05));
    let scale = mix(3.2, 5.6, saturate(material.scalars[0].y));
    let weave = abs(sin(p.x * scale * 28.0)) * abs(sin(p.z * scale * 28.0));
    let pile = fbm(p * scale * 0.55);
    var tint = mix(base, accent, pile * 0.55 + weave * 0.2);
    tint *= mix(0.88, 1.16, weave);
    return Look(tint, mix(0.68, 0.88, 1.0 - weave), 0.0, 0.14);
}

fn soft_look(p: vec3<f32>) -> Look {
    let base = palette(0u);
    let accent = palette_or(1u, base * vec3<f32>(1.02, 0.96, 0.88));
    let cotton = fbm(p * 4.2);
    let speckle = value_noise_3d(p * 14.0);
    var tint = mix(base, accent, cotton * 0.28);
    tint *= 0.94 + 0.08 * speckle;
    return Look(tint, mix(0.86, 0.96, cotton), 0.0, 0.10);
}

fn marble_look(p: vec3<f32>) -> Look {
    let field = palette(0u);
    let vein = palette_or(1u, vec3<f32>(0.72, 0.52, 0.28));
    let spark = palette_or(2u, vec3<f32>(0.98, 0.92, 0.78));
    let scale = mix(1.6, 3.0, saturate(material.scalars[0].y));
    let q = p * scale;
    let warp = vec3<f32>(fbm(q), fbm(q + 19.2), fbm(q + 4.8));
    let body = fbm(q * 0.65 + warp * 1.7);
    let hair = fbm(q * 2.6 + warp * 0.55);
    let major = smoothstep(0.44, 0.52, body) * (1.0 - smoothstep(0.52, 0.74, body));
    let minor = smoothstep(0.48, 0.56, hair) * 0.45;
    var tint = mix(field, vein, saturate(major + minor));
    tint = mix(tint, spark, major * 0.22);
    let roughness = mix(0.12, 0.28, 1.0 - major);
    return Look(tint, roughness, 0.04, 0.42);
}

fn ornate_look(p: vec3<f32>) -> Look {
    let wood = wood_look(p);
    let gold = palette_or(1u, vec3<f32>(0.95, 0.72, 0.22));
    let gem = palette_or(2u, vec3<f32>(0.22, 0.48, 0.62));
    let uv = fract(vec2<f32>(p.x, p.z) * 0.85) - vec2<f32>(0.5);
    let diamond = abs(uv.x) + abs(uv.y);
    let panel = 1.0 - smoothstep(0.26, 0.33, max(abs(uv.x), abs(uv.y)));
    let frame = saturate(
        (1.0 - smoothstep(0.30, 0.37, max(abs(uv.x), abs(uv.y)))) - panel
    );
    let lattice = smoothstep(0.045, 0.018, abs(fract((uv.x + uv.y) * 7.0) - 0.5))
        * smoothstep(0.40, 0.22, diamond);
    let hy = fract(p.y * 1.65 + 0.12);
    let strap = 1.0 - smoothstep(0.07, 0.13, abs(hy - 0.28));
    let rivet = smoothstep(0.08, 0.03, length(fract(vec2<f32>(p.x, p.y) * 4.4) - vec2<f32>(0.5)));
    let metal = saturate(strap * 0.95 + frame + lattice * 0.75 + rivet * strap);
    var tint = mix(wood.rgb, gem * 1.18, panel * 0.88);
    tint = mix(tint, gold * 1.22, metal);
    let roughness = mix(wood.roughness, 0.18, metal);
    return Look(tint, roughness, metal * 0.82, mix(wood.reflectance, 0.46, metal));
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input = pbr_input_from_vertex_output(mesh, is_front, false);
    let p = look_coord(mesh);
    var look = Look(palette(0u), 0.55, 0.0, 0.22);

    switch material.kind {
        case KIND_CLOTH: { look = cloth_look(p); }
        case KIND_SOFT: { look = soft_look(p); }
        case KIND_MARBLE: { look = marble_look(p); }
        case KIND_ORNATE: { look = ornate_look(p); }
        default: { look = wood_look(p); }
    }

    let rgb = finish(look.rgb, mesh.world_normal, mesh.world_position.xyz);
    pbr_input.material.base_color = vec4<f32>(rgb, 1.0);
    pbr_input.material.perceptual_roughness = look.roughness;
    pbr_input.material.metallic = look.metallic;
    pbr_input.material.reflectance = vec3<f32>(look.reflectance);

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit_color.rgb, 1.0), view.color_grading);
}
