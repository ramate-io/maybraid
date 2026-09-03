//---------------------------------------------------------
// Richmond urban surfaces: palette tint + named recipe kind.
//
// Kind: 0 stucco, 1 terracotta, 2 wood, 3 hay, 4 iron.
//---------------------------------------------------------

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_types::{PbrInput, pbr_input_new},
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct UrbanSurfaceUniform {
    colors: array<vec4<f32>, 8>,
    noise: vec4<f32>,
    scalars: array<vec4<f32>, 8>,
    rasters: array<array<vec4<f32>, 3>, 8>,
    kind: u32,
    _pad: vec3<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: UrbanSurfaceUniform;

const KIND_STUCCO: u32 = 0u;
const KIND_TERRACOTTA: u32 = 1u;
const KIND_WOOD: u32 = 2u;
const KIND_HAY: u32 = 3u;
const KIND_IRON: u32 = 4u;

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
    for (var i = 0; i < 4; i++) {
        s += a * value_noise_3d(q);
        q *= 2.03;
        a *= 0.5;
    }
    return s;
}

fn look_coord(mesh: VertexOutput) -> vec3<f32> {
    let freq = max(material.noise.x, 1e-4);
    return mesh.world_position.xyz * freq;
}

fn palette_base() -> vec3<f32> {
    return material.colors[0].xyz;
}

fn palette_accent() -> vec3<f32> {
    let accent = material.colors[1].xyz;
    if (dot(accent, accent) < 1e-6) {
        return palette_base() * 0.72;
    }
    return accent;
}

fn scalar0() -> f32 {
    return material.scalars[0].x;
}

fn scalar1() -> f32 {
    return material.scalars[0].y;
}

fn scalar2() -> f32 {
    return material.scalars[0].z;
}

/// rgb + roughness. `w` is perceptual roughness.
fn stucco_look(p: vec3<f32>) -> vec4<f32> {
    let base = palette_base();
    let accent = palette_accent();
    let scale = mix(1.1, 2.4, saturate(scalar1()));
    let mottling = fbm(p * scale);
    let pits = value_noise_3d(p * scale * 4.2);
    let wear = saturate(scalar2());
    var tint = mix(base, accent, mottling * 0.35 + wear * 0.2);
    tint *= 0.88 + 0.18 * pits;
    let roughness = mix(0.86, 0.98, mottling);
    return vec4<f32>(tint, mix(roughness, saturate(scalar0()), step(1e-4, scalar0())));
}

fn terracotta_look(p: vec3<f32>) -> vec4<f32> {
    let base = palette_base();
    let accent = palette_accent();
    let scale = mix(1.4, 3.2, saturate(scalar1()));
    let tile = vec2<f32>(p.x, p.z) * scale * 1.6;
    let grout = max(
        abs(fract(tile.x) - 0.5),
        abs(fract(tile.y) - 0.5),
    );
    let line = smoothstep(0.44, 0.5, grout);
    let clay = fbm(p * scale);
    var tint = mix(base, accent, clay * 0.4);
    tint = mix(tint, tint * 0.55, line);
    let roughness = mix(0.72, 0.9, clay);
    return vec4<f32>(tint, mix(roughness, saturate(scalar0()), step(1e-4, scalar0())));
}

fn wood_look(p: vec3<f32>) -> vec4<f32> {
    let base = palette_base();
    let accent = palette_accent();
    let scale = mix(1.6, 3.6, saturate(scalar1()));
    let along = p.y * scale * 2.8 + p.x * 0.35;
    let grain = sin(along + fbm(p * scale) * 3.2) * 0.5 + 0.5;
    let pore = value_noise_3d(vec3<f32>(p.x * scale * 6.0, p.y * scale * 0.4, p.z * scale * 6.0));
    var tint = mix(base, accent, grain * 0.55);
    tint *= 0.9 + 0.16 * pore;
    let roughness = mix(0.62, 0.82, grain);
    return vec4<f32>(tint, mix(roughness, saturate(scalar0()), step(1e-4, scalar0())));
}

fn hay_look(p: vec3<f32>) -> vec4<f32> {
    let base = palette_base();
    let accent = palette_accent();
    let scale = mix(2.2, 4.4, saturate(scalar1()));
    let strand_id = floor(p.x * scale * 7.0 + p.z * scale * 1.4);
    let strand = fract(p.x * scale * 7.0 + fbm(vec3<f32>(strand_id, p.y, p.z)) * 0.4);
    let ridge = pow(1.0 - abs(strand * 2.0 - 1.0), 0.55);
    let clump = fbm(p * scale * 0.7);
    var tint = mix(base, accent, clump * 0.5);
    tint *= mix(0.72, 1.12, ridge);
    let roughness = mix(0.9, 0.99, 1.0 - ridge);
    return vec4<f32>(tint, mix(roughness, saturate(scalar0()), step(1e-4, scalar0())));
}

fn iron_look(p: vec3<f32>) -> vec4<f32> {
    let base = palette_base();
    let rust = palette_accent();
    let scale = mix(1.2, 2.8, saturate(scalar1()));
    let blot = fbm(p * scale);
    let speckle = value_noise_3d(p * scale * 5.5);
    let rust_amt = smoothstep(0.42, 0.78, blot) * mix(0.25, 0.7, saturate(scalar2()));
    var tint = mix(base * (0.78 + 0.22 * speckle), rust, rust_amt);
    let roughness = mix(0.28, 0.72, rust_amt);
    return vec4<f32>(tint, mix(roughness, saturate(scalar0()), step(1e-4, scalar0())));
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();
    let p = look_coord(mesh);
    var look = vec4<f32>(palette_base(), 0.8);
    var metallic = 0.0;

    switch material.kind {
        case KIND_TERRACOTTA: {
            look = terracotta_look(p);
            metallic = 0.02;
        }
        case KIND_WOOD: {
            look = wood_look(p);
            metallic = 0.0;
        }
        case KIND_HAY: {
            look = hay_look(p);
            metallic = 0.0;
        }
        case KIND_IRON: {
            look = iron_look(p);
            metallic = mix(0.72, 0.18, saturate((look.w - 0.28) / 0.5));
        }
        default: {
            look = stucco_look(p);
            metallic = 0.0;
        }
    }

    pbr_input.material.base_color = vec4<f32>(look.xyz, 1.0);
    pbr_input.material.perceptual_roughness = look.w;
    pbr_input.material.metallic = metallic;
    pbr_input.material.reflectance = vec3<f32>(0.18, 0.18, 0.18);

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    let prepared_normal = fns::prepare_world_normal(mesh.world_normal, false, is_front);
    let n = normalize(prepared_normal);
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit_color.rgb, 1.0), view.color_grading);
}
