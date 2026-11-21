//! Compiles against every reusable vegetation module. Not a production entry point.
//!
//! ```wesl
//! import vegetation_shaders::wesl::hash::{hash_u32, density_rank, plant_id};
//! import vegetation_shaders::wesl::curve::{blade_rest_sample, ribbon_vertex, stable_frame, attachment_frame};
//! import vegetation_shaders::wesl::wind::{wind_field, deform_along_curve};
//! import vegetation_shaders::wesl::lod::{projected_height_px, lod_bucket, nested_keep};
//! import vegetation_shaders::wesl::shading::{grass_albedo, painterly_lit};
//! import vegetation_shaders::wesl::layout::{BladeDescriptor, GrassViewUniform, GrassPartingUniform};
//! import vegetation_shaders::wesl::parting::{part_along_curve, part_along_length, depth_gap_weight};
//! ```

import vegetation_shaders::wesl::hash::{hash_u32, density_rank, plant_id};
import vegetation_shaders::wesl::curve::{
    blade_rest_sample,
    ribbon_vertex,
    ribbon_normal,
    stable_frame,
    attachment_frame,
    blade_width_at,
};
import vegetation_shaders::wesl::wind::{wind_field, deform_along_curve, WindInputs};
import vegetation_shaders::wesl::lod::{projected_height_px, lod_bucket, nested_keep, LodThresholds};
import vegetation_shaders::wesl::shading::{grass_albedo, painterly_lit, VegetationPalette};
import vegetation_shaders::wesl::layout::{BladeDescriptor, GrassViewUniform, GrassPartingUniform};
import vegetation_shaders::wesl::parting::{
    part_along_curve,
    part_along_length,
    depth_gap_weight,
    parting_along,
    PARTING_MODE_DEPTH,
    PARTING_MODE_RASTER,
};

fn smoke_eval(
    blade: BladeDescriptor,
    view: GrassViewUniform,
    parting: GrassPartingUniform,
    t: f32,
    side: f32,
) -> vec4<f32> {
    let id = plant_id(blade.plant_id, 0u);
    let rank = density_rank(id);
    let _h = hash_u32(id);
    let rest = blade_rest_sample(
        blade.root,
        blade.normal,
        blade.facing,
        blade.height_m,
        blade.shape.x,
        t,
    );
    let attached = attachment_frame(rest, 0.0);
    let _frame = stable_frame(rest.tangent, blade.normal);
    let _along = attached.along;
    let half_w = 0.5 * blade_width_at(blade.width_m, blade.shape.y, t);
    let pos = ribbon_vertex(rest, side, half_w);
    let n = ribbon_normal(rest, side);
    let wind = wind_field(
        blade.root.xz,
        WindInputs(view.wind.xy, view.lod_distances.x, view.wind.w, view.world_position.w),
    );
    let winded = deform_along_curve(pos, blade.root, t, wind, blade.shape.z);
    let gap = depth_gap_weight(2.0, 2.4, parting.params.z);
    let part_xz = vec2<f32>(parting.params.x, 0.0) * gap * parting_along(t, blade.shape.z);
    var deformed = part_along_curve(winded, blade.root, t, part_xz, blade.shape.z);
    if (u32(parting.params.w) == PARTING_MODE_RASTER) {
        deformed = part_along_length(winded, blade.root, t, part_xz, blade.shape.z);
    }
    if (u32(parting.params.w) == PARTING_MODE_DEPTH) {
        deformed = deformed;
    }
    let thresholds = LodThresholds(
        view.lod_distances.y,
        view.lod_distances.z,
        view.lod_distances.w,
        view.lod_density.x,
        view.lod_density.y,
        view.lod_density.z,
        view.lod_density.w,
    );
    let keep = nested_keep(rank, view.lod_density.x);
    let bucket = lod_bucket(
        length(deformed - view.world_position.xyz),
        blade.height_m,
        rank,
        view.resolution.y,
        view.resolution.z,
        thresholds,
    );
    let pal = VegetationPalette(blade.palette.xyz, blade.palette.xyz * 1.4);
    let albedo = grass_albedo(pal, t, rank);
    var color = painterly_lit(albedo, n, view.light_direction.xyz, t);
    if (!keep || bucket > 2u) {
        color = color * 0.0;
    }
    return vec4(color, 1.0);
}

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    var blade: BladeDescriptor;
    blade.root = vec3<f32>(0.0, 0.0, 0.0);
    blade.height_m = 0.5;
    blade.normal = vec3<f32>(0.0, 1.0, 0.0);
    blade.width_m = 0.03;
    blade.facing = vec3<f32>(1.0, 0.0, 0.0);
    blade.plant_id = 1u;
    blade.shape = vec4<f32>(0.3, 1.2, 0.0, 0.1);
    blade.palette = vec4<f32>(0.1, 0.4, 0.05, 0.0);
    var view: GrassViewUniform;
    view.clip_from_world = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0),
    );
    view.world_from_clip = view.clip_from_world;
    view.world_position = vec4<f32>(0.0, 1.0, 4.0, 0.0);
    view.prev_world_position = view.world_position;
    view.resolution = vec4<f32>(1280.0, 720.0, 1.0, 0.0);
    view.wind = vec4<f32>(0.92, 0.38, 0.0, 0.2);
    view.lod_distances = vec4<f32>(0.85, 18.0, 48.0, 90.0);
    view.lod_density = vec4<f32>(1.0, 0.72, 0.38, 1.15);
    view.counts = vec4<f32>(1.0, 1.0, 0.0, 0.0);
    view.light_direction = vec4<f32>(0.4, 0.8, 0.3, 0.0);
    var parting: GrassPartingUniform;
    parting.camera_clip_from_world = view.clip_from_world;
    parting.camera_world_from_clip = view.world_from_clip;
    parting.params = vec4<f32>(0.35, 0.85, 1.6, 1.0);
    parting.extras = vec4<f32>(28.0, 0.22, 3.0, 0.0);
    parting.camera_right = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    parting.camera_forward = vec4<f32>(0.0, 0.0, 1.0, 0.0);
    parting.camera_position = view.world_position;
    return smoke_eval(blade, view, parting, 0.5, 1.0);
}
