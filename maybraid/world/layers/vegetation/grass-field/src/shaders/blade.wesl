//! Vertex-generated tapered ribbons. Opaque painterly shading.

import vegetation_shaders::wesl::curve::{
    blade_rest_sample, blade_width_at, ribbon_normal, ribbon_vertex,
};
import vegetation_shaders::wesl::wind::{deform_along_curve, wind_field, WindInputs};
import vegetation_shaders::wesl::shading::{grass_albedo, painterly_lit, VegetationPalette};
import vegetation_shaders::wesl::layout::{BladeDescriptor, GrassDrawParams, GrassViewUniform};

@group(0) @binding(0) var<uniform> view: GrassViewUniform;
@group(0) @binding(1) var<storage, read> blades: array<BladeDescriptor>;
@group(0) @binding(2) var<storage, read> compacted: array<u32>;
@group(0) @binding(3) var<uniform> draw: GrassDrawParams;

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
    let width_scale = select(select(1.0, 1.85, draw.lod_bucket == 1u), 3.6, draw.lod_bucket == 2u);
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
    let world = deform_along_curve(rest_pos, blade.root, t, wind_now, blade.shape.z);
    let prev = deform_along_curve(rest_pos, blade.root, t, wind_prev, blade.shape.z);
    let n = ribbon_normal(rest, side);
    let pal = VegetationPalette(blade.palette.xyz * 0.55, blade.palette.xyz * 1.35);
    out.position = view.clip_from_world * vec4<f32>(world, 1.0);
    out.world_position = world;
    out.world_normal = n;
    out.t = t;
    out.albedo = grass_albedo(pal, t, blade.shape.w);
    out.prev_position = prev;
    return out;
}

@fragment
fn fragment(in: BladeVertexOutput) -> @location(0) vec4<f32> {
    let color = painterly_lit(in.albedo, in.world_normal, view.light_direction.xyz, in.t);
    return vec4<f32>(color, 1.0);
}
