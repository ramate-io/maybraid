// Packed orchard kits: shared mesh + storage-buffer instances.
// Wind / watercolor follow leaf_material.wgsl; sticks skip sway.

struct PackedView {
    clip_from_world: mat4x4<f32>,
    camera_position: vec4<f32>,
    sun_direction: vec4<f32>,
    time_day: vec4<f32>,
};

struct PackedInstance {
    world_from_local: mat4x4<f32>,
    plant_origin: vec4<f32>,
};

struct PackedMaterial {
    base_color: vec4<f32>,
    kind_pad: vec4<f32>,
};

@group(0) @binding(0) var<uniform> view_u: PackedView;
@group(1) @binding(0) var<storage, read> instances: array<PackedInstance>;
@group(2) @binding(0) var<uniform> material: PackedMaterial;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) local_pos: vec3<f32>,
    @location(3) view_dist: f32,
    @location(4) @interpolate(flat) plant_origin: vec3<f32>,
};

const WIND_DIR: vec3<f32> = vec3<f32>(0.72, 0.0, 0.38);
const LEAF_SWAY_CUT_DIST: f32 = 140.0;
const LEAF_MID_DIST: f32 = 80.0;
const LEAF_RIM_CUT: f32 = 0.96;

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn value_noise_3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (vec3<f32>(3.0) - 2.0 * f0);
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

fn instance_scale(m: mat4x4<f32>) -> f32 {
    let s = vec3<f32>(length(m[0].xyz), length(m[1].xyz), length(m[2].xyz));
    return max(max(s.x, s.y), s.z);
}

fn canopy_sway(
    local_pos: vec3<f32>,
    world_normal: vec3<f32>,
    centroid: vec3<f32>,
    scale: f32,
    view_dist: f32,
    time: f32,
) -> vec3<f32> {
    if (view_dist > LEAF_SWAY_CUT_DIST) {
        return vec3<f32>(0.0);
    }
    let phase = hash13(centroid) * 6.283185;
    let gust = sin(time * 0.55 + phase) * 0.5 + 0.5;
    let flutter = sin(time * 3.1 + phase * 2.0) * 0.35;
    let tip = saturate(length(local_pos));
    let amp = (0.04 + 0.07 * gust + 0.02 * flutter) * tip * scale;
    let along = normalize(WIND_DIR + vec3<f32>(0.0, 0.15, 0.0));
    return along * amp + world_normal * amp * 0.25;
}

@vertex
fn vertex(v: VertexIn, @builtin(instance_index) instance_index: u32) -> VertexOut {
    var out: VertexOut;
    let inst = instances[instance_index];
    let world_from_local = inst.world_from_local;
    let world_pos = (world_from_local * vec4<f32>(v.position, 1.0)).xyz;
    let n = normalize((world_from_local * vec4<f32>(v.normal, 0.0)).xyz);
    let centroid = world_from_local[3].xyz;
    let scale = instance_scale(world_from_local);
    let view_dist = length(centroid - view_u.camera_position.xyz);
    let kind = material.kind_pad.x;
    var displaced = world_pos;
    if (kind < 0.5 || kind > 1.5) {
        displaced += canopy_sway(v.position, n, centroid, scale, view_dist, view_u.time_day.x);
    }
    out.clip = view_u.clip_from_world * vec4<f32>(displaced, 1.0);
    out.world_position = displaced;
    out.world_normal = n;
    out.local_pos = v.position;
    out.view_dist = view_dist;
    out.plant_origin = inst.plant_origin.xyz;
    return out;
}

@fragment
fn fragment(in: VertexOut, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let kind = material.kind_pad.x;
    var n = normalize(in.world_normal);
    if (!is_front) {
        n = -n;
    }
    let day = saturate(view_u.time_day.y);
    let L = view_u.sun_direction.xyz;
    let ndl = saturate(dot(n, L));
    let sun = ndl * mix(0.22, 0.95, day) + mix(0.02, 0.14, day);
    let sky = mix(0.06, 0.38, day) + mix(0.04, 0.17, day) * saturate(n.y);
    let sky_rgb = mix(vec3<f32>(0.16, 0.08, 0.30), vec3<f32>(0.78, 0.88, 1.0), day);
    let base_rgb = material.base_color.xyz;

    if (kind > 0.5 && kind < 1.5) {
        let dN = abs(dpdx(n)) + abs(dpdy(n));
        let edge = smoothstep(0.0001, 0.05, length(dN));
        let shaded = base_rgb * (sun + sky * sky_rgb) * (1.0 - edge * 0.55);
        return vec4<f32>(shaded, 1.0);
    }

    let r = saturate(length(in.local_pos));
    let cheese = in.view_dist < LEAF_MID_DIST;
    var radial_alpha = 1.0;
    var hole_alpha = 1.0;
    var tint_noise = 0.5;
    if (cheese) {
        if (r > LEAF_RIM_CUT) {
            discard;
        }
        let blot = 0.52 + 0.38 * value_noise_3d(in.local_pos * 2.4);
        radial_alpha = smoothstep(0.0, 0.08, blot - r);
        if (radial_alpha < 0.08) {
            discard;
        }
        let hole = value_noise_3d(in.local_pos * 3.25) * 0.62 + value_noise_3d(in.local_pos * 8.5) * 0.38;
        let hub = 1.0 - smoothstep(0.22, 0.62, r);
        let threshold = mix(0.22, 0.52, 1.0 - hub);
        hole_alpha = smoothstep(threshold - 0.02, threshold + 0.02, hole);
        tint_noise = value_noise_3d(in.local_pos * 1.75);
        if (hole_alpha * radial_alpha < 0.08) {
            discard;
        }
    } else {
        tint_noise = value_noise_3d(in.local_pos * 3.25);
        hole_alpha = mix(0.85, 1.0, tint_noise);
    }
    let warm_cool = mix(vec3<f32>(0.94, 1.02, 0.96), vec3<f32>(1.08, 1.04, 0.94), tint_noise);
    let albedo = base_rgb * warm_cool * mix(0.94, 1.08, tint_noise);
    let outward = in.world_position - in.plant_origin;
    let olen = length(outward);
    var occ = mix(0.72, 1.0, r);
    if (olen > 1e-3) {
        occ *= mix(0.68, 1.0, saturate(dot(n, outward / olen)) * saturate(dot(n, outward / olen)));
    }
    occ = mix(occ, 1.0, saturate((in.view_dist - 32.0) / 48.0));
    let lifted = (albedo * sun + albedo * sky * sky_rgb) * occ;
    return vec4<f32>(lifted, hole_alpha * radial_alpha);
}
