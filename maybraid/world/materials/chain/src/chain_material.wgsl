//---------------------------------------------------------
// Mythical chain: stylized metal, UV-locked energy veins,
// and durability deterioration. V is along the chain.
//
// Vertex color: R = link id, G = localized damage,
// B = twist phase, A reserved.
// Scalars: intact, glow, fade, damage, link_count,
// crack_frequency, localized_blend, pulse_rate.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    forward_io::Vertex,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct ChainMaterialUniform {
    colors: array<vec4<f32>, 8>,
    noise: vec4<f32>,
    scalars: array<vec4<f32>, 8>,
    rasters: array<array<vec4<f32>, 3>, 8>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: ChainMaterialUniform;

struct ChainVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
}

// Hash helpers are the shared bodies in hash.wgsl. Keep them verbatim.
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
    let octaves = i32(clamp(material.noise.w, 1.0, 5.0));
    for (var i = 0; i < 5; i++) {
        if i >= octaves {
            break;
        }
        s += a * value_noise_3d(q);
        q *= 2.03;
        a *= 0.5;
    }
    return s;
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> ChainVertexOutput {
    var out: ChainVertexOutput;
    var vertex = vertex_no_morph;
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
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif
    return out;
}

fn chain_uv(mesh: ChainVertexOutput) -> vec2<f32> {
#ifdef VERTEX_UVS_A
    return mesh.uv;
#else
    return vec2<f32>(0.5, 0.5);
#endif
}

fn chain_color(mesh: ChainVertexOutput) -> vec4<f32> {
#ifdef VERTEX_COLORS
    return mesh.color;
#else
    return vec4<f32>(-1.0, 0.0, 0.0, 1.0);
#endif
}

fn link_id(uv: vec2<f32>, color: vec4<f32>, link_count: f32) -> f32 {
    if color.r >= 0.0 {
        return floor(color.r + 0.5);
    }
    return floor(saturate(uv.y) * max(link_count, 1.0));
}

fn link_weakness(id: f32, seed: f32) -> f32 {
    return hash13(vec3<f32>(id * 17.13, seed * 0.013, 4.2));
}

fn look_from_damage(damage: f32, fade: f32) -> vec4<f32> {
    let crack = smoothstep(0.08, 0.72, damage);
    let glow_scale = (1.0 - damage * 0.82) * (1.0 - fade * 0.85);
    let ember = smoothstep(0.42, 0.92, damage) * (1.0 - fade * 0.35);
    let dissolve = saturate(fade * 0.72 + damage * fade * 0.28 + smoothstep(0.78, 1.0, damage) * 0.18);
    return vec4<f32>(crack, glow_scale, ember, dissolve);
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: ChainVertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();
    let uv = chain_uv(mesh);
    let color = chain_color(mesh);

    let intact = material.scalars[0].x;
    let glow_gain = max(material.scalars[0].y, 0.0);
    let fade = saturate(material.scalars[0].z);
    var damage_amount = material.scalars[0].w;
    if damage_amount <= 1e-5 {
        damage_amount = 1.0 - saturate(intact);
    }
    let link_count = max(material.scalars[1].x, 1.0);
    let crack_freq = max(material.scalars[1].y, 0.5);
    let localized_blend = saturate(material.scalars[1].z);
    let pulse_rate = max(material.scalars[1].w, 0.05);
    let seed = material.noise.z;
    let freq = max(material.noise.x, 0.05);
    let amp = material.noise.y;

    let id = link_id(uv, color, link_count);
    let weakness = link_weakness(id, seed);
    let localized = saturate(max(color.g, 0.0));
    let mixed = damage_amount * (1.0 - localized_blend) + localized * localized_blend;
    let damage = saturate(mixed * (0.55 + 0.75 * weakness));
    let look = look_from_damage(damage, fade);

    let along = uv.y;
    let around = uv.x;
    let n1 = fbm(vec3<f32>(around * crack_freq * freq, along * crack_freq * 1.6 * freq, seed * 0.02));
    let n2 = fbm(vec3<f32>(around * crack_freq * 2.4 * freq + 3.1, along * 3.2 * freq, 1.7));
    let ridge = 1.0 - abs(n1 * 2.0 - 1.0);
    let crack_mask = pow(saturate(ridge * 1.18), 5.5) * look.x;
    let rust = saturate((n2 - 0.32) * 2.1) * look.x;

    let vein_u = 1.0 - abs(fract(around * 2.0) * 2.0 - 1.0);
    let vein_v = 0.62 + 0.38 * sin(along * 22.0 + n1 * 3.0);
    let vein = pow(saturate(vein_u * vein_v), 1.65);
    let pulse = 0.84 + 0.16 * sin(globals.time * pulse_rate * 6.0 + id * 1.7 + along * 8.0);
    let flicker = 0.88 + 0.12 * sin(globals.time * 22.0 + weakness * 9.0);
    let facing = saturate(abs(dot(normalize(mesh.world_normal), fns::calculate_view(mesh.world_position, view.clip_from_view[3].w == 1.0))));

    let metal = material.colors[0].xyz;
    let glow = material.colors[1].xyz;
    let ash = material.colors[2].xyz;
    let ember = material.colors[3].xyz;

    var albedo = metal * (0.92 + 0.22 * value_noise_3d(vec3<f32>(uv * 18.0, seed)));
    albedo = mix(albedo, ash, saturate(look.x * 0.48 + rust * 0.95 + crack_mask * 0.75));
    let cavity = mix(1.0, 0.38, crack_mask);
    albedo *= cavity;

    let wash = mix(0.20, 0.0, saturate(look.x * 1.35)) * look.y;
    let glow_amt = (vein * mix(0.9, 0.35, look.x) + wash) * look.y * pulse * flicker;
    let ember_amt = (crack_mask * 0.85 + look.z * 0.35) * look.z * flicker;
    let rim = pow(1.0 - facing, 2.2) * look.y * 0.55;
    let emissive = glow * (glow_amt + rim) * glow_gain + ember * ember_amt * (1.6 + glow_gain * 0.35);
    albedo = mix(albedo, glow, saturate(glow_amt * 0.28 + rim * 0.15));

    let dissolve_noise = hash21(vec2<f32>(id * 3.1, around * 8.0 + along * 5.0));
    let keep_spark = fade > 0.7 && hash21(vec2<f32>(id * 9.2, around * 3.0)) > 0.82;
    if !keep_spark && dissolve_noise < look.w + crack_mask * fade * 0.35 {
        discard;
    }

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);
    let prepared_normal = fns::prepare_world_normal(mesh.world_normal, true, is_front);
    var n = normalize(prepared_normal);
    let g = vec2<f32>(dpdx(n1), dpdy(n1)) * look.x * amp;
    n = normalize(n - vec3<f32>(g.x, 0.0, g.y) * 0.35);

    pbr_input.material.base_color = vec4<f32>(albedo, 1.0);
    pbr_input.material.perceptual_roughness = mix(0.38, 0.78, look.x);
    pbr_input.material.metallic = mix(0.82, 0.28, rust);
    pbr_input.material.reflectance = vec3<f32>(0.42, 0.42, 0.42);
    pbr_input.material.flags = pbr_input.material.flags | STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT;
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit_color.rgb + emissive, 1.0), view.color_grading);
}
