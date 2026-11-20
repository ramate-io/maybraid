//! Vertex-generated tapered ribbons. Opaque painterly shading.
//!
//! Rest curve → wind → view-dependent parting. Parting is applied on the already
//! wind-deformed point so arch and gusts are not evaluated twice.

import vegetation_shaders::wesl::curve::{
    blade_rest_sample, blade_width_at, ribbon_normal, ribbon_vertex,
};
import vegetation_shaders::wesl::wind::{deform_along_curve, wind_field, WindInputs};
import vegetation_shaders::wesl::shading::{
    grass_albedo, grass_variation_palette, painterly_lit,
};
import vegetation_shaders::wesl::layout::{
    BladeDescriptor, GrassDrawParams, GrassPartingUniform, GrassViewUniform,
};
import vegetation_shaders::wesl::parting::{
    capsule_away_xz, capsule_behind_weight, capsule_radial_weight, clip_to_uv,
    depth_gap_weight, ground_occluder_weight, part_along_curve, reconstruct_world,
    screen_part_direction, world_part_from_screen, PARTING_MODE_DEPTH, PARTING_MODE_OFF,
    PARTING_MODE_RASTER, PARTING_MODE_WORLD,
};

@group(0) @binding(0) var<uniform> view: GrassViewUniform;
@group(0) @binding(1) var<storage, read> blades: array<BladeDescriptor>;
@group(0) @binding(2) var<storage, read> compacted: array<u32>;
@group(0) @binding(3) var<uniform> draw: GrassDrawParams;

@group(1) @binding(0) var<uniform> parting: GrassPartingUniform;
@group(1) @binding(1) var depth_prepass: texture_depth_2d;
@group(1) @binding(2) var raster_mask: texture_2d<f32>;
@group(1) @binding(3) var raster_sampler: sampler;

struct TemplateVertex {
    @location(0) t_side: vec2<f32>,
}

struct BladeVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) t: f32,
    @location(3) albedo: vec3<f32>,
    @location(4) prev_position: vec3<f32>,
}

fn load_prepass_depth(uv: vec2<f32>) -> f32 {
    let dims = vec2<f32>(textureDimensions(depth_prepass));
    let max_t = max(vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1), vec2<i32>(0));
    let texel = vec2<i32>(vec2<f32>(clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0))) * dims);
    return textureLoad(depth_prepass, clamp(texel, vec2<i32>(0), max_t), 0);
}

fn camera_up() -> vec3<f32> {
    return normalize(cross(parting.camera_forward.xyz, parting.camera_right.xyz));
}

fn camera_world() -> vec3<f32> {
    return parting.camera_position.xyz;
}

fn depth_behind_at(uv: vec2<f32>, blade_world: vec3<f32>, root: vec3<f32>) -> f32 {
    let depth = load_prepass_depth(uv);
    if (depth <= 1e-5) {
        return 0.0;
    }
    let occ = reconstruct_world(uv, depth, parting.camera_world_from_clip);
    let ground = ground_occluder_weight(occ.y, root.y, parting.extras.y);
    if (ground <= 1e-4) {
        return 0.0;
    }
    let camera = camera_world();
    let blade_dist = length(blade_world - camera);
    let occ_dist = length(occ - camera);
    return depth_gap_weight(blade_dist, occ_dist, parting.params.z) * ground;
}

fn depth_part_xz(tip: vec3<f32>, root: vec3<f32>, plant_id: u32) -> vec2<f32> {
    let clip = parting.camera_clip_from_world * vec4<f32>(tip, 1.0);
    if (clip.w <= 1e-4) {
        return vec2<f32>(0.0);
    }
    let uv = clip_to_uv(clip);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return vec2<f32>(0.0);
    }
    let dims = vec2<f32>(textureDimensions(depth_prepass));
    let kernel = max(parting.extras.z, 1.0);
    let du = vec2<f32>(kernel / max(dims.x, 1.0), 0.0);
    let dv = vec2<f32>(0.0, kernel / max(dims.y, 1.0));
    let w_c = depth_behind_at(uv, tip, root);
    if (w_c <= 1e-4) {
        return vec2<f32>(0.0);
    }
    let w_l = depth_behind_at(uv - du, tip, root);
    let w_r = depth_behind_at(uv + du, tip, root);
    let w_d = depth_behind_at(uv + dv, tip, root);
    let w_u = depth_behind_at(uv - dv, tip, root);
    let dir_ss = screen_part_direction(vec2<f32>(w_r - w_l, w_u - w_d), plant_id);
    let world = world_part_from_screen(
        dir_ss,
        parting.camera_right.xyz,
        camera_up(),
        parting.camera_forward.xyz,
    );
    return world.xz * (w_c * parting.params.x);
}

fn raster_part_xz(tip: vec3<f32>, root: vec3<f32>) -> vec2<f32> {
    let clip = parting.camera_clip_from_world * vec4<f32>(tip, 1.0);
    if (clip.w <= 1e-4) {
        return vec2<f32>(0.0);
    }
    let uv = clip_to_uv(clip);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return vec2<f32>(0.0);
    }
    let sample = textureSampleLevel(raster_mask, raster_sampler, uv, 0.0);
    if (sample.b <= 1e-4) {
        return vec2<f32>(0.0);
    }
    let camera = camera_world();
    let blade_dist = length(tip - camera);
    let gap = depth_gap_weight(blade_dist, sample.a, parting.params.z);
    let ground = ground_occluder_weight(root.y + parting.extras.y + 0.4, root.y, parting.extras.y);
    let dir_len = length(sample.rg);
    let away = select(sample.rg, sample.rg / dir_len, dir_len > 1e-4);
    return away * (sample.b * gap * ground * parting.params.x);
}

fn world_part_xz(root: vec3<f32>) -> vec2<f32> {
    let camera = camera_world();
    let count = u32(parting.extras.w);
    var acc = vec2<f32>(0.0);
    for (var i = 0u; i < 16u; i++) {
        if (i >= count) {
            break;
        }
        if (parting.occluder_meta[i].y < 0.5) {
            continue;
        }
        let center = parting.occluders[i].xyz;
        let radius = parting.occluders[i].w + parting.params.y;
        let behind = capsule_behind_weight(
            root,
            camera,
            center,
            parting.camera_forward.xyz,
            parting.params.z,
        );
        let ground = ground_occluder_weight(center.y, root.y, parting.extras.y);
        let radial = capsule_radial_weight(
            root.xz,
            center.xz,
            parting.camera_forward.xz,
            radius,
        );
        let weight = behind * ground * radial;
        if (weight <= 1e-5) {
            continue;
        }
        let away = capsule_away_xz(
            root.xz,
            center.xz,
            parting.camera_right.xz,
            parting.camera_forward.xz,
        );
        acc += away * weight;
    }
    return acc * parting.params.x;
}

fn parting_offset_xz(tip: vec3<f32>, root: vec3<f32>, plant_id: u32, distance_m: f32) -> vec2<f32> {
    let mode = u32(parting.params.w);
    if (mode == PARTING_MODE_OFF || parting.params.x <= 1e-5) {
        return vec2<f32>(0.0);
    }
    if (draw.lod_bucket >= 2u || distance_m > parting.extras.x) {
        return vec2<f32>(0.0);
    }
    if (mode == PARTING_MODE_DEPTH) {
        return depth_part_xz(tip, root, plant_id);
    }
    if (mode == PARTING_MODE_RASTER) {
        return raster_part_xz(tip, root);
    }
    if (mode == PARTING_MODE_WORLD) {
        return world_part_xz(root);
    }
    return vec2<f32>(0.0);
}

@vertex
fn vertex(vert: TemplateVertex, @builtin(instance_index) instance_index: u32) -> BladeVertexOutput {
    var out: BladeVertexOutput;
    let blade_index = compacted[instance_index];
    let blade = blades[blade_index];
    let t = vert.t_side.x;
    let side = vert.t_side.y;
    let rest = blade_rest_sample(
        blade.root,
        blade.normal,
        blade.facing,
        blade.height_m,
        blade.shape.x,
        t,
    );
    let width_scale = select(select(1.0, 2.35, draw.lod_bucket == 1u), 5.8, draw.lod_bucket == 2u);
    let half_w = 0.5 * blade_width_at(blade.width_m, blade.shape.y, t) * width_scale;
    let rest_pos = ribbon_vertex(rest, side, half_w);
    let wind_now = wind_field(
        blade.root.xz,
        WindInputs(view.wind.xy, view.lod_distances.x, view.wind.w, view.world_position.w),
    );
    let wind_prev = wind_field(
        blade.root.xz,
        WindInputs(view.wind.xy, view.lod_distances.x, view.wind.w, view.prev_world_position.w),
    );
    var world = deform_along_curve(rest_pos, blade.root, t, wind_now, blade.shape.z);
    var prev = deform_along_curve(rest_pos, blade.root, t, wind_prev, blade.shape.z);
    let distance_m = length(view.world_position.xyz - blade.root);
    let part_xz = parting_offset_xz(world, blade.root, blade.plant_id, distance_m);
    world = part_along_curve(world, blade.root, t, part_xz, blade.shape.z);
    prev = part_along_curve(prev, blade.root, t, part_xz, blade.shape.z);
    let n = ribbon_normal(rest, side);
    let pal = grass_variation_palette(blade.palette.xyz, blade.root.xz, blade.plant_id);
    out.position = view.clip_from_world * vec4<f32>(world, 1.0);
    out.world_position = world;
    out.world_normal = n;
    out.t = t;
    out.albedo = grass_albedo(pal, t, 0.55);
    out.prev_position = prev;
    return out;
}

@fragment
fn fragment(in: BladeVertexOutput) -> @location(0) vec4<f32> {
    let color = painterly_lit(in.albedo, in.world_normal, view.light_direction.xyz, in.t);
    return vec4<f32>(color, 1.0);
}
