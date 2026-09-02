//---------------------------------------------------------
// Crozon clothing: palette tint, look kind, tiny hem sway.
//
// Kind: 0 cloth, 1 space suit, 2 tattered, 3 hawaiian,
//       4 wizard veins, 5 glitter, 6 scales.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::{morph_position, morph_normal, morph_tangent},
    forward_io::Vertex,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct ClothingMaterialUniform {
    base_color: vec4<f32>,
    kind: u32,
    _pad: vec3<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: ClothingMaterialUniform;

const KIND_CLOTH: u32 = 0u;
const KIND_SPACE_SUIT: u32 = 1u;
const KIND_TATTERED: u32 = 2u;
const KIND_HAWAIIAN: u32 = 3u;
const KIND_WIZARDS_VEINS: u32 = 4u;
const KIND_GLITTER: u32 = 5u;
const KIND_SCALES: u32 = 6u;

struct ClothingVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
    @location(8) local_pos: vec3<f32>,
}

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn hash21(p: vec2<f32>) -> f32 {
    return hash13(vec3<f32>(p, 17.13));
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

fn clothing_sway(local_pos: vec3<f32>, world_normal: vec3<f32>) -> vec3<f32> {
    let hem = saturate(-local_pos.y * 0.85 + 0.15);
    let t = globals.time;
    let phase = dot(local_pos, vec3<f32>(2.7, 1.1, 3.4));
    let gust = sin(t * 1.25 + phase) * 0.006 + sin(t * 2.1 + phase * 1.7) * 0.0025;
    let flutter = sin(t * 3.4 + local_pos.x * 8.0) * 0.0015;
    var offset = vec3<f32>(0.92, 0.05, 0.38) * (gust * (0.25 + 0.75 * hem));
    offset += world_normal * (flutter * (0.4 + 0.6 * hem));
    return offset;
}

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex, instance_index: u32) -> Vertex {
    var vertex = vertex_in;
    let first_vertex = mesh[instance_index].first_vertex_index;
    let vertex_index = vertex.index - first_vertex;

    let weight_count = bevy_pbr::morph::layer_count(instance_index);
    for (var i: u32 = 0u; i < weight_count; i++) {
        let weight = bevy_pbr::morph::weight_at(i, instance_index);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph_position(vertex_index, i, instance_index);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph_normal(vertex_index, i, instance_index);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * morph_tangent(vertex_index, i, instance_index), 0.0);
#endif
    }
    return vertex;
}
#endif

@vertex
fn vertex(vertex_no_morph: Vertex) -> ClothingVertexOutput {
    var out: ClothingVertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph, vertex_no_morph.instance_index);
#else
    var vertex = vertex_no_morph;
#endif

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

#ifdef SKINNED
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_no_morph.instance_index
    );
#else
    var world_from_local = mesh_world_from_local;
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index
    );
#endif
#endif

    out.local_pos = vertex.position;
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.world_position += vec4<f32>(clothing_sway(vertex.position, out.world_normal), 0.0);
    out.position = position_world_to_clip(out.world_position.xyz);

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index
    );
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif

    return out;
}

fn look_coord(mesh: ClothingVertexOutput) -> vec2<f32> {
#ifdef VERTEX_UVS_A
    return mesh.uv;
#else
    return mesh.world_position.xz * 0.55 + mesh.local_pos.xy * 0.35;
#endif
}

/// Basket weave: warp and weft ridges with fiber noise along each thread.
fn cloth_threads(uv: vec2<f32>) -> vec3<f32> {
    let count = 168.0;
    let warp_id = floor(uv.x * count);
    let weft_id = floor(uv.y * count);
    let warp_f = fract(uv.x * count);
    let weft_f = fract(uv.y * count);
    let warp_ridge = pow(1.0 - abs(warp_f * 2.0 - 1.0), 0.45);
    let weft_ridge = pow(1.0 - abs(weft_f * 2.0 - 1.0), 0.45);
    let warp_on_top = step(0.5, fract((warp_id + weft_id) * 0.5));
    let ridge = mix(weft_ridge, warp_ridge, warp_on_top);
    let along = mix(uv.x, uv.y, warp_on_top);
    let fiber = value_noise_3d(vec3<f32>(along * 420.0, ridge * 18.0, warp_id * 0.13 + weft_id * 0.07));
    let slub = 0.82 + 0.28 * value_noise_3d(vec3<f32>(uv * 48.0, 3.1));
    return vec3<f32>(ridge, fiber, slub);
}

fn cloth_look(base: vec3<f32>, uv: vec2<f32>) -> vec4<f32> {
    let t = cloth_threads(uv);
    let groove = mix(0.52, 1.16, t.x);
    let fiber = mix(0.88, 1.08, t.y);
    let tint = base * groove * fiber * t.z;
    return vec4<f32>(tint, mix(0.82, 0.58, t.x));
}

fn space_suit_look(base: vec3<f32>, uv: vec2<f32>, n: vec3<f32>) -> vec4<f32> {
    let panel = abs(fract(uv.x * 6.0) - 0.5) * abs(fract(uv.y * 8.0) - 0.5);
    let seam = smoothstep(0.42, 0.48, max(
        abs(fract(uv.x * 6.0) - 0.5),
        abs(fract(uv.y * 8.0) - 0.5),
    ));
    let iridescence = 0.5 + 0.5 * n.y;
    let sheen = mix(vec3<f32>(0.55, 0.72, 0.92), vec3<f32>(0.95, 0.88, 0.72), iridescence);
    let tint = mix(base * 0.55 + sheen * 0.35, base * 0.22, seam) * (0.92 + 0.16 * panel);
    return vec4<f32>(tint, 0.18);
}

/// Worn cloth: same thread pattern, stained, with discarded holes.
fn tattered_look(base: vec3<f32>, uv: vec2<f32>, local_pos: vec3<f32>) -> vec4<f32> {
    let cloth = cloth_look(base, uv);
    let wear = fbm(vec3<f32>(uv * 5.2, local_pos.y * 2.2));
    let ragged = value_noise_3d(vec3<f32>(uv * 22.0, 4.4));
    let stain = value_noise_3d(vec3<f32>(uv * 3.6, 2.7));
    let hole = step(0.76, wear) * step(0.42, ragged);
    let fray_speck = step(0.70, wear) * step(0.93, hash21(floor(uv * 190.0)));
    if (hole + fray_speck > 0.5) {
        discard;
    }
    let dirt = mix(cloth.xyz, cloth.xyz * vec3<f32>(0.28, 0.22, 0.16), stain * 0.65);
    let fray = smoothstep(0.58, 0.78, wear);
    let tint = mix(dirt, dirt * 0.55, fray);
    return vec4<f32>(tint, mix(cloth.w, 0.92, fray));
}

fn hawaiian_look(base: vec3<f32>, uv: vec2<f32>) -> vec4<f32> {
    let p = uv * vec2<f32>(4.0, 5.2);
    let cell = floor(p);
    let f = fract(p) - 0.5;
    let petal = 0.18 + 0.04 * sin(hash21(cell) * 40.0);
    let flower = smoothstep(petal, petal * 0.35, length(f));
    let leaf = smoothstep(0.22, 0.05, abs(f.x * 1.6 + sin(f.y * 9.0) * 0.08));
    let blossom = vec3<f32>(0.95, 0.35, 0.22);
    let tropic = vec3<f32>(0.18, 0.62, 0.34);
    let ground = mix(base * 0.92, base * 1.15, value_noise_3d(vec3<f32>(uv * 3.0, 1.0)));
    var tint = mix(ground, tropic, leaf * 0.55);
    tint = mix(tint, mix(blossom, vec3<f32>(1.0, 0.85, 0.2), hash21(cell + 3.1)), flower);
    return vec4<f32>(tint, 0.78);
}

fn wizards_veins_look(base: vec3<f32>, uv: vec2<f32>, local_pos: vec3<f32>) -> vec4<f32> {
    let n1 = fbm(vec3<f32>(uv * 5.5, local_pos.y * 2.0 + globals.time * 0.12));
    let ridge = 1.0 - abs(n1 * 2.0 - 1.0);
    let vein = pow(saturate(ridge * 1.15), 7.0);
    let pulse = 0.55 + 0.45 * sin(globals.time * 2.4 + n1 * 18.0);
    let glow = vec3<f32>(0.35, 0.82, 1.0) * (0.35 + 0.65 * pulse);
    let cloth = base * (0.18 + 0.22 * value_noise_3d(vec3<f32>(uv * 12.0, 0.2)));
    let tint = mix(cloth, glow, vein);
    return vec4<f32>(tint, 0.55);
}

fn glitter_look(base: vec3<f32>, uv: vec2<f32>, n: vec3<f32>) -> vec4<f32> {
    let cell = floor(uv * 92.0);
    let spark_id = hash21(cell);
    let twinkle = step(0.82, spark_id) * pow(
        saturate(sin(globals.time * (6.0 + spark_id * 9.0) + spark_id * 40.0) * 0.5 + 0.5),
        8.0,
    );
    let flake = hash21(cell + 11.3);
    let spec = pow(saturate(n.y * 0.5 + 0.5), 3.0);
    let sparkle = vec3<f32>(1.0, 0.96, 0.85) * twinkle * (0.7 + 0.3 * flake);
    let tint = base * 0.85 + sparkle + spec * 0.12;
    return vec4<f32>(tint, 0.28);
}

/// Offset-row ovals with a bright rim and darker overlap, like overlapping plates.
fn scales_look(base: vec3<f32>, uv: vec2<f32>, n: vec3<f32>) -> vec4<f32> {
    let p = uv * vec2<f32>(16.0, 20.0);
    let row = floor(p.y);
    let odd = step(0.5, fract(row * 0.5));
    var q = p;
    q.x += odd * 0.5;
    let cell = floor(q);
    let f = fract(q) - vec2<f32>(0.5);
    let d = length(f * vec2<f32>(1.05, 1.35));
    let rim = smoothstep(0.36, 0.48, d);
    let gap = smoothstep(0.48, 0.54, d);
    let jitter = hash21(cell);
    let shade = 0.78 + 0.22 * jitter;
    let highlight = pow(saturate(0.55 - d), 1.6) * (0.35 + 0.45 * saturate(n.y));
    let plate = mix(base * 0.55, base * 1.12, 1.0 - rim) * shade;
    let edge = mix(vec3<f32>(0.08, 0.07, 0.06), plate, 1.0 - gap);
    let tint = edge + vec3<f32>(1.0, 0.95, 0.82) * highlight;
    return vec4<f32>(tint, mix(0.22, 0.55, rim));
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: ClothingVertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();
    let uv = look_coord(mesh);
    let base = material.base_color.xyz;
    var look = vec4<f32>(base, 0.75);
    var metallic = 0.0;
    var emissive = vec3<f32>(0.0);

    switch material.kind {
        case KIND_SPACE_SUIT: {
            look = space_suit_look(base, uv, mesh.world_normal);
            metallic = 0.82;
        }
        case KIND_TATTERED: {
            look = tattered_look(base, uv, mesh.local_pos);
            metallic = 0.02;
        }
        case KIND_HAWAIIAN: {
            look = hawaiian_look(base, uv);
            metallic = 0.0;
        }
        case KIND_WIZARDS_VEINS: {
            look = wizards_veins_look(base, uv, mesh.local_pos);
            metallic = 0.08;
            let n1 = fbm(vec3<f32>(uv * 5.5, mesh.local_pos.y * 2.0 + globals.time * 0.12));
            let ridge = 1.0 - abs(n1 * 2.0 - 1.0);
            let vein = pow(saturate(ridge * 1.15), 7.0);
            let pulse = 0.55 + 0.45 * sin(globals.time * 2.4 + n1 * 18.0);
            emissive = vec3<f32>(0.25, 0.75, 1.0) * vein * (0.8 + 0.6 * pulse);
        }
        case KIND_GLITTER: {
            look = glitter_look(base, uv, mesh.world_normal);
            metallic = 0.55;
        }
        case KIND_SCALES: {
            look = scales_look(base, uv, mesh.world_normal);
            metallic = 0.42;
        }
        default: {
            look = cloth_look(base, uv);
            metallic = 0.0;
        }
    }

    pbr_input.material.base_color = vec4<f32>(look.xyz, 1.0);
    pbr_input.material.perceptual_roughness = look.w;
    pbr_input.material.metallic = metallic;
    pbr_input.material.reflectance = vec3<f32>(0.18, 0.18, 0.18);
    pbr_input.material.flags = pbr_input.material.flags | STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT;

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    let prepared_normal = fns::prepare_world_normal(mesh.world_normal, true, is_front);
    let n = normalize(prepared_normal);
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit_color.rgb + emissive, 1.0), view.color_grading);
}
