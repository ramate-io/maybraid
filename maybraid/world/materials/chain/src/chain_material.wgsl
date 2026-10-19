//---------------------------------------------------------
// Mythical chain: stylized metal, UV-locked energy veins,
// branching fracture atlas, and durability deterioration.
// V is along the chain. Link-local UVs drive the cracks.
//
// Vertex color: R = link id, G = optional localized damage,
// B = twist phase, A reserved.
// Scalars: intact, glow, fade, damage, link_count,
// crack_frequency, localized_blend, pulse_rate.
// Runtime: impact vec4 + packed per-link damage[32].
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

struct ChainMaterialUniform {
    colors: array<vec4<f32>, 8>,
    noise: vec4<f32>,
    scalars: array<vec4<f32>, 8>,
    rasters: array<array<vec4<f32>, 3>, 8>,
    impact: vec4<f32>,
    halo: vec4<f32>,
    link_damage: array<vec4<f32>, 8>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: ChainMaterialUniform;
@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var crack_atlas: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var crack_sampler: sampler;

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
    let n101 = hash13(i + vec3<f32>(1.0, 1.0, 1.0));
    let n011 = hash13(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash13(i + vec3<f32>(1.0, 1.0, 1.0));

    let nx00 = mix(n000, n100, f.x);
    let nx10 = mix(n010, n110, f.x);
    let nx01 = mix(n001, n101, f.x);
    let nx11 = mix(n011, n111, f.x);
    return mix(mix(nx00, nx10, f.y), mix(nx01, nx11, f.y), f.z);
}

fn chain_pulse(time: f32, pulse_rate: f32, link_id: f32) -> f32 {
    let rate = max(pulse_rate, 0.05);
    let phase = fract(time * rate);
    let t = min(abs(phase - 0.86) / 0.055, 1.0);
    let crest = (1.0 - t) * (1.0 - t);
    let body = 0.16 + 0.84 * crest;
    let link = 0.05 * sin(link_id * 2.1 + time * rate * 0.35);
    return saturate(body + link);
}

fn persistent_link_damage(id: f32) -> f32 {
    let idx = u32(clamp(id, 0.0, 31.0));
    let packed = material.link_damage[idx / 4u];
    let lane = idx % 4u;
    if lane == 0u {
        return packed.x;
    } else if lane == 1u {
        return packed.y;
    } else if lane == 2u {
        return packed.z;
    }
    return packed.w;
}

fn crack_variant(seed: f32, id: f32) -> f32 {
    return floor(hash13(vec3<f32>(seed * 0.017, id * 0.31, 5.9)) * 4.0);
}

fn sample_cracks(local_uv: vec2<f32>, variant: f32) -> vec4<f32> {
    let wrapped = fract(local_uv);
    let inset = wrapped * 0.984 + 0.008;
    let col = floor(variant) % 2.0;
    let row = floor(variant / 2.0);
    let atlas_uv = (inset + vec2<f32>(col, row)) * 0.5;
    return textureSample(crack_atlas, crack_sampler, atlas_uv);
}

fn apply_height_bump(n: vec3<f32>, world_pos: vec3<f32>, height: f32, strength: f32) -> vec3<f32> {
    let dhdx = dpdx(height);
    let dhdy = dpdy(height);
    let sigma_x = dpdx(world_pos);
    let sigma_y = dpdy(world_pos);
    let r1 = cross(sigma_y, n);
    let r2 = cross(n, sigma_x);
    let det = dot(sigma_x, r1);
    let surf_grad = sign(det) * (dhdx * r1 + dhdy * r2) / max(abs(det), 1e-8);
    return normalize(n - surf_grad * strength);
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> ChainVertexOutput {
    var out: ChainVertexOutput;
    var vertex = vertex_no_morph;
    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

#ifdef VERTEX_COLORS
    let pulse_rate = max(material.scalars[1].w, 0.05);
    let id = floor(vertex.color.r + 0.5);
    let pulse = chain_pulse(globals.time, pulse_rate, id);
#else
    let pulse = chain_pulse(globals.time, max(material.scalars[1].w, 0.05), 0.0);
#endif
    if material.halo.x > 0.5 {
        let expand = material.halo.y * (0.55 + 0.80 * pulse);
        vertex.position = vertex.position + vertex.normal * expand;
    }

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
    let amp = material.noise.y;

    let id = link_id(uv, color, link_count);
    let weakness = link_weakness(id, seed);
    let localized = saturate(max(color.g, 0.0));
    let persistent = persistent_link_damage(id);
    let mixed = max(
        damage_amount * (1.0 - localized_blend) + localized * localized_blend,
        persistent
    );
    let along = (id + 0.5) / link_count;
    let hit_dist = abs(along - material.impact.x);
    let hit_age = max(globals.time - material.impact.w, 0.0);
    let flash = material.impact.z
        * saturate(1.0 - hit_dist / max(material.impact.y, 1e-3))
        * exp(-hit_age * 3.2);
    let damage = saturate(mixed * (0.55 + 0.75 * weakness) + flash * 0.35);
    let look = look_from_damage(damage, fade);
    let pulse = chain_pulse(globals.time, pulse_rate, id);
    let flicker = 0.94 + 0.06 * sin(globals.time * 18.0 + weakness * 9.0);

    let around = uv.x;
    let along_link = saturate(uv.y * link_count - id);
    let crack_uv = vec2<f32>(around * crack_freq * 0.22, along_link * 3.05);
    let variant = crack_variant(seed, id);
    let crack_s = sample_cracks(crack_uv, variant);
    let dist = crack_s.x;
    let height = crack_s.y;
    let cell = crack_s.z;

    let hair = mix(0.008, 0.026, look.x);
    let fissure = mix(0.024, 0.095, look.x);
    let shoulder = fissure * 1.95;
    let crack_vis = saturate(look.x * 1.15 + persistent * 0.35 + flash * 0.25);
    let center = (1.0 - smoothstep(0.0, hair * 0.85, dist)) * crack_vis;
    let inner_edge = smoothstep(hair * 0.35, hair, dist)
        * (1.0 - smoothstep(hair, fissure * 0.72, dist))
        * crack_vis;
    let shoulder_t = smoothstep(fissure * 0.45, fissure, dist)
        * (1.0 - smoothstep(fissure, shoulder, dist))
        * crack_vis;

    let metal = material.colors[0].xyz;
    let glow = material.colors[1].xyz;
    let ash = material.colors[2].xyz;
    let ember = material.colors[3].xyz;

    var albedo = metal * (0.92 + 0.18 * value_noise_3d(vec3<f32>(uv * 18.0, seed)));
    albedo = mix(albedo, ash * 0.12, center);
    albedo = mix(albedo, glow * 0.55 + vec3<f32>(0.22, 0.18, 0.12), inner_edge);
    albedo = mix(albedo, ash * 0.55, shoulder_t * 0.92);
    let rust = saturate((value_noise_3d(vec3<f32>(around * 7.0, along_link * 5.0, seed)) - 0.55) * 2.4)
        * look.x
        * saturate(1.0 - dist * 1.6);
    albedo = mix(albedo, ash, rust * 0.55);

    let vein_u = 1.0 - abs(fract(around * 2.0 + along_link * 0.15) * 2.0 - 1.0);
    let vein = pow(saturate(vein_u * 0.92), 2.1) * (0.55 + 0.45 * (1.0 - look.x));
    let facing = saturate(abs(dot(
        normalize(mesh.world_normal),
        fns::calculate_view(mesh.world_position, view.clip_from_view[3].w == 1.0)
    )));
    let rim = pow(1.0 - facing, 2.2) * look.y * pulse;
    let wash = mix(0.10, 0.0, saturate(look.x * 1.35)) * look.y;
    let glow_amt = (vein * mix(0.55, 0.16, look.x) + wash + inner_edge * 0.85) * look.y * pulse * flicker;
    let ember_amt = (center * 1.05 + look.z * 0.32) * look.z * flicker;
    // Keep peaks HDR (no tone_mapping) but do not wash the metal to white at authored gain.
    let emit_gain = min(glow_gain, 2.4) * 0.42;
    let emissive = glow * (glow_amt + rim * 0.55) * emit_gain
        + ember * ember_amt * (1.15 + emit_gain * 0.45)
        + glow * flash * 0.85
        + glow * 0.04 * look.y;
    albedo = mix(albedo, glow, saturate(glow_amt * 0.16 + rim * 0.08));

    let open = look.w + look.x * 0.12;
    let piece = hash21(vec2<f32>(cell * 13.1 + id, seed * 0.05));
    if dist < mix(0.0, 0.10, open) && piece < open + look.w * 0.2 {
        discard;
    }
    if look.w > 0.52 && piece < look.w * 0.82 && dist < 0.20 {
        discard;
    }

    if material.halo.x > 0.5 {
        let fresnel = pow(1.0 - facing, 2.35);
        let opacity = fresnel * mix(0.05, 0.38, pulse) * max(look.y, 0.16);
        let halo_rgb = glow * (0.55 + pulse * 1.35) * min(glow_gain, 2.8) * opacity;
        return vec4<f32>(halo_rgb, opacity);
    }

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);
    let prepared_normal = fns::prepare_world_normal(mesh.world_normal, true, is_front);
    var n = normalize(prepared_normal);
    let bump_h = mix(height, 0.5, 1.0 - crack_vis);
    n = apply_height_bump(n, mesh.world_position.xyz, bump_h, 0.55 * amp * crack_vis);

    pbr_input.material.base_color = vec4<f32>(albedo, 1.0);
    pbr_input.material.perceptual_roughness = mix(0.38, 0.78, look.x);
    pbr_input.material.metallic = mix(0.82, 0.28, rust);
    pbr_input.material.reflectance = vec3<f32>(0.42, 0.42, 0.42);
    pbr_input.material.flags = pbr_input.material.flags | STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT;
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    // HDR: do not tone-map here so emission can survive to bloom / the additive shell.
    return vec4<f32>(lit_color.rgb + emissive, 1.0);
}
