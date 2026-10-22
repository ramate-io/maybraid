//---------------------------------------------------------
// Mythical chain: painterly metal, traveling energy, staged
// structural loss. V is along the chain. UV_1.x is rest-space
// arc length in metres (flow_s_m); UV_1.y is total rest length.
//
// Vertex color: R = link id, G = optional localized damage,
// B = twist phase, A reserved.
// Scalars: intact, glow, fade, damage, link_count,
// conduit_frequency, localized_blend, reserved,
// flow speed / wavelength / baseline / peak,
// halo width / halo gain.
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

struct ChainVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) flow_s_m: f32,
    @location(7) rest_length_m: f32,
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

fn link_weakness(id: f32, seed: f32) -> f32 {
    return hash13(vec3<f32>(id * 17.13, seed * 0.013, 4.2));
}

fn value_noise_3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (3.0 - 2.0 * f0);

    let n000 = hash13(i);
    let n100 = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash13(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash13(i + vec3<f32>(1.0, 0.0, 1.0));
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

fn chain_flow(s_m: f32, time: f32, speed: f32, wavelength: f32, baseline: f32, peak: f32) -> f32 {
    let lam = max(wavelength, 1e-3);
    let phase = 6.2831853 * (s_m - speed * time) / lam;
    let broad_wave = 0.5 + 0.5 * cos(phase);
    let envelope = smoothstep(0.05, 0.95, broad_wave);
    return mix(baseline, max(peak, baseline), envelope);
}

fn hit_flash(age: f32, strength: f32) -> f32 {
    let rise = smoothstep(0.0, 0.07, age);
    let fall = 1.0 - smoothstep(0.10, 0.55, age);
    return saturate(strength * rise * fall);
}

fn art_curve(t: f32) -> f32 {
    let s = t * t * (3.0 - 2.0 * t);
    return t * 0.65 + s * 0.35;
}

fn global_severity(damage: f32, weakness: f32) -> f32 {
    if damage <= 1e-5 {
        return 0.0;
    }
    if damage >= 0.999 {
        return 1.0;
    }
    let shift = (weakness - 0.5) * 0.30 * (4.0 * damage * (1.0 - damage));
    return art_curve(saturate(damage + shift));
}

fn wrap_delta_u(a: f32, b: f32) -> f32 {
    let d = a - b;
    return d - round(d);
}

fn bite_threshold(bite: u32) -> f32 {
    if bite == 0u {
        return 0.18;
    } else if bite == 1u {
        return 0.36;
    } else if bite == 2u {
        return 0.58;
    } else if bite == 3u {
        return 0.80;
    }
    return 0.0;
}

fn bite_half_range(bite: u32) -> vec2<f32> {
    if bite == 0u {
        return vec2<f32>(0.045, 0.070);
    } else if bite == 1u {
        return vec2<f32>(0.075, 0.115);
    } else if bite == 2u {
        return vec2<f32>(0.085, 0.135);
    } else if bite == 3u {
        return vec2<f32>(0.110, 0.180);
    }
    return vec2<f32>(0.0, 0.0);
}

fn bite_radius(severity: f32, bite: u32) -> f32 {
    let threshold = bite_threshold(bite);
    if threshold <= 0.0 || severity + 1e-5 < threshold {
        return 0.0;
    }
    let range = bite_half_range(bite);
    let t = saturate((severity - threshold) / 0.22);
    return range.x + (range.y - range.x) * t;
}

fn bite_center_v(link_id: f32, seed: f32, bite: u32) -> f32 {
    let origin = hash13(vec3<f32>(0.5, seed * 0.017, link_id * 0.31));
    if bite == 0u {
        return origin;
    } else if bite == 1u {
        return fract(origin + 0.11);
    } else if bite == 2u {
        return fract(origin + 0.50);
    }
    return fract(origin + 0.68);
}

// x = inside (1 = discard), y = distance to nearest active cut face.
fn sample_bites(local_uv: vec2<f32>, seed: f32, id: f32, severity: f32) -> vec2<f32> {
    var inside = 0.0;
    var lip = 10.0;
    let fw = fwidth(local_uv.y);
    for (var bite = 0u; bite < 4u; bite++) {
        let half = bite_radius(severity, bite);
        if half <= 0.0 {
            continue;
        }
        let d = abs(wrap_delta_u(local_uv.y, bite_center_v(id, seed, bite)));
        if d < half - fw * 0.35 {
            inside = 1.0;
        }
        lip = min(lip, abs(d - half));
    }
    return vec2<f32>(inside, lip);
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

fn painted_metal(p: vec3<f32>, base: vec3<f32>, accent: vec3<f32>, shade: vec3<f32>) -> vec3<f32> {
    let stain = value_noise_3d(p * 0.72);
    let wash = value_noise_3d(p * 0.28 + vec3<f32>(3.1, 1.7, 8.2));
    var tint = mix(base, accent, smoothstep(0.34, 0.66, stain));
    tint = mix(tint, shade, smoothstep(0.42, 0.78, wash) * 0.42);
    return tint;
}

fn conduit_mask(around: f32, along: f32, seed: f32, freq: f32) -> f32 {
    let veins = max(freq, 0.8);
    let warp = 0.07 * value_noise_3d(vec3<f32>(around * 2.0, along * 0.55, seed));
    let u = around * veins + warp;
    let ridge = 1.0 - abs(fract(u) * 2.0 - 1.0);
    return pow(saturate(ridge * 1.06), 2.2);
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> ChainVertexOutput {
    var out: ChainVertexOutput;
    var vertex = vertex_no_morph;
    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

    let speed = material.scalars[2].x;
    let wavelength = material.scalars[2].y;
    let baseline = material.scalars[2].z;
    let peak = material.scalars[2].w;
    let halo_width = material.scalars[3].x;
#ifdef VERTEX_UVS_B
    let flow_s = vertex.uv_b.x;
#else
    let flow_s = 0.0;
#endif
    let energy = chain_flow(flow_s, globals.time, speed, wavelength, baseline, peak);

    if material.halo.x > 0.5 {
        let expand = halo_width * (0.94 + 0.08 * energy);
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
#ifdef VERTEX_UVS_B
    out.flow_s_m = vertex.uv_b.x;
    out.rest_length_m = vertex.uv_b.y;
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

fn chain_flow_s(mesh: ChainVertexOutput, id: f32, link_count: f32) -> f32 {
#ifdef VERTEX_UVS_B
    return mesh.flow_s_m;
#else
    // Stepped per-link fallback. Not continuous along a bent string.
    return (id + 0.5) / max(link_count, 1.0) * 3.1;
#endif
}

fn link_id(uv: vec2<f32>, color: vec4<f32>, link_count: f32) -> f32 {
    if color.r >= 0.0 {
        return floor(color.r + 0.5);
    }
    return floor(saturate(uv.y) * max(link_count, 1.0));
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: ChainVertexOutput,
) -> @location(0) vec4<f32> {
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
    let conduit_freq = max(material.scalars[1].y, 0.5);
    let localized_blend = saturate(material.scalars[1].z);
    let seed = material.noise.z;
    let amp = material.noise.y;
    let speed = material.scalars[2].x;
    let wavelength = material.scalars[2].y;
    let baseline = material.scalars[2].z;
    let peak = material.scalars[2].w;
    let halo_gain = material.scalars[3].y;

    let id = link_id(uv, color, link_count);
    let weakness = link_weakness(id, seed);
    let localized = saturate(max(color.g, 0.0));
    let persistent = persistent_link_damage(id);
    let mixed_global = damage_amount * (1.0 - localized_blend) + localized * localized_blend;
    let severity = max(global_severity(mixed_global, weakness), persistent);

    if fade >= 0.995 {
        discard;
    }

    let around = uv.x;
    let along_link = saturate(uv.y * link_count - id);
    let local_uv = vec2<f32>(around, along_link);
    let bites = sample_bites(local_uv, seed, id, severity);
    if bites.x > 0.5 {
        discard;
    }

    let flow_s = chain_flow_s(mesh, id, link_count);
    let energy = chain_flow(flow_s, globals.time, speed, wavelength, baseline, peak);
    let along = (id + 0.5) / link_count;
    let hit_dist = abs(along - material.impact.x);
    let hit_age = max(globals.time - material.impact.w, 0.0);
    let hit_mask = saturate(1.0 - hit_dist / max(material.impact.y, 1e-3));
    let flash = hit_flash(hit_age, material.impact.z) * hit_mask;
    // One lighting value. Do not multiply the envelope again.
    let light = saturate(energy + flash * 0.28);
    let breath = 0.96 + 0.04 * sin(globals.time * 1.05);
    let lit = light * breath;

    let facing = saturate(abs(dot(
        normalize(mesh.world_normal),
        fns::calculate_view(mesh.world_position, view.clip_from_view[3].w == 1.0)
    )));

    if material.halo.x > 0.5 {
        // Soft volume, not a hard rim. Presence is authored gain * one energy.
        let fresnel = pow(1.0 - facing, 2.15);
        var opacity = (0.22 + 0.78 * fresnel) * lit * halo_gain * (1.0 - fade);
        if !is_front {
            opacity *= 0.38;
        }
        let glow = material.colors[1].xyz;
        var rgb = glow * opacity;
        // Keep additive turquoise from stacking to white on interlocking links.
        rgb = rgb / (vec3<f32>(1.0) + rgb * 0.55);
        return vec4<f32>(rgb, saturate(opacity));
    }

    var pbr_input: PbrInput = pbr_input_new();
    let metal = material.colors[0].xyz;
    let glow = material.colors[1].xyz;
    let ash = material.colors[2].xyz;
    let ember = material.colors[3].xyz;
    let bronze = metal * vec3<f32>(1.22, 0.90, 0.62);
    let shade = metal * vec3<f32>(0.62, 0.64, 0.70);
    var albedo = painted_metal(vec3<f32>(around, along_link, seed * 0.05), metal, bronze, shade);

    let conduit = conduit_mask(around, along_link, seed, conduit_freq);
    let conduit_core = pow(conduit, 2.2);
    let lip_w = 0.040 + fwidth(bites.y);
    let exposed = saturate(1.0 - smoothstep(0.0, lip_w, bites.y));

    albedo = mix(albedo, ash * 0.82, exposed * 0.72);
    albedo = mix(albedo, glow, conduit * (0.42 + 0.48 * lit));
    albedo = mix(albedo, glow, conduit_core * (0.28 + 0.40 * lit));
    albedo = mix(albedo, glow * 0.88, exposed * (0.46 + 0.42 * lit));
    albedo = mix(albedo, ember * 0.45, exposed * smoothstep(0.72, 1.0, severity) * 0.14);
    if !is_front {
        albedo *= 0.74;
    }

    let pale = mix(glow, vec3<f32>(0.55, 0.96, 0.90), 0.16);
    let emit_gain = glow_gain / (1.0 + glow_gain * 0.10);
    let body_emit = glow * conduit * lit * emit_gain * 0.92;
    let core_emit = pale * conduit_core * lit * emit_gain * 0.22;
    let edge_emit = glow * exposed * lit * emit_gain * 0.80;
    let hit_emit = glow * flash * exposed * 0.35;
    var emissive = (body_emit + core_emit + edge_emit + hit_emit) * (1.0 - fade);
    emissive = emissive / (vec3<f32>(1.0) + emissive * 0.40);

    let height = 0.5 + conduit * 0.08 - exposed * 0.16;
    let prepared_normal = fns::prepare_world_normal(mesh.world_normal, true, is_front);
    var n = apply_height_bump(
        normalize(prepared_normal),
        mesh.world_position.xyz,
        height,
        0.10 * amp
    );

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);
    pbr_input.material.base_color = vec4<f32>(albedo, 1.0);
    pbr_input.material.perceptual_roughness = mix(0.68, 0.82, exposed);
    pbr_input.material.metallic = mix(0.22, 0.10, exposed);
    pbr_input.material.reflectance = vec3<f32>(0.16, 0.16, 0.16);
    pbr_input.material.flags = pbr_input.material.flags | STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT;
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);
    return vec4<f32>(lit_color.rgb + emissive, 1.0);
}
