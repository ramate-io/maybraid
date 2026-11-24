//! Vertex-generated tapered tubes. Opaque painterly bark.

import vegetation_shaders::wesl::curve::{
    attachment_frame, branch_radius_at, branch_rest_sample, stable_frame, woody_surface_normal,
    woody_surface_vertex, CurveSample, LocalFrame,
};
import vegetation_shaders::wesl::wind::{deform_along_curve, wind_field, WindInputs};
import vegetation_shaders::wesl::shading::{painterly_lit, wood_albedo, VegetationPalette};
import vegetation_shaders::wesl::layout::{GrassDrawParams, GrassViewUniform};
import vegetation_tree_field::shaders::layout::{BranchDescriptor, PlantDescriptor};

@group(0) @binding(0) var<uniform> view: GrassViewUniform;
@group(0) @binding(1) var<storage, read> plants: array<PlantDescriptor>;
@group(0) @binding(2) var<storage, read> branches: array<BranchDescriptor>;
@group(0) @binding(3) var<storage, read> compacted: array<u32>;
@group(0) @binding(4) var<uniform> draw: GrassDrawParams;

const PARENT_NONE: u32 = 0xFFFFFFFFu;

struct TemplateVertex {
    @location(0) t_radial: vec2<f32>,
}

struct WoodVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) t: f32,
    @location(2) albedo: vec3<f32>,
}

fn deform_trunk(branch: BranchDescriptor, root: vec3<f32>, t: f32, wind_xz: vec2<f32>) -> CurveSample {
    let rest = branch_rest_sample(branch.p0, branch.p1, branch.p2, t);
    let position = deform_along_curve(rest.position, root, t, wind_xz, branch.stiffness);
    let rest_hi = branch_rest_sample(branch.p0, branch.p1, branch.p2, saturate(t + 0.02));
    let hi = deform_along_curve(rest_hi.position, root, saturate(t + 0.02), wind_xz, branch.stiffness);
    let frame = stable_frame(hi - position, vec3<f32>(0.0, 1.0, 0.0));
    return CurveSample(position, frame.along, frame);
}

fn local_to_parent(local: vec3<f32>, attach: LocalFrame, origin: vec3<f32>) -> vec3<f32> {
    return origin + attach.across * local.x + attach.along * local.y + attach.facing * local.z;
}

fn deform_child(
    branch: BranchDescriptor,
    t: f32,
    parent_sample: CurveSample,
    wind_xz: vec2<f32>,
) -> CurveSample {
    let attach = attachment_frame(parent_sample, branch.twist);
    let local = branch_rest_sample(branch.p0, branch.p1, branch.p2, t);
    let rest_world = local_to_parent(local.position, attach, parent_sample.position);
    let position = deform_along_curve(rest_world, parent_sample.position, t, wind_xz, branch.stiffness);
    let local_hi = branch_rest_sample(branch.p0, branch.p1, branch.p2, saturate(t + 0.02));
    let rest_hi = local_to_parent(local_hi.position, attach, parent_sample.position);
    let hi = deform_along_curve(rest_hi, parent_sample.position, saturate(t + 0.02), wind_xz, branch.stiffness);
    let frame = stable_frame(hi - position, vec3<f32>(0.0, 1.0, 0.0));
    return CurveSample(position, frame.along, frame);
}

fn world_sample(branch: BranchDescriptor, t: f32, wind_xz: vec2<f32>) -> CurveSample {
    let plant = plants[branch.plant_index];
    if (branch.parent_index == PARENT_NONE) {
        return deform_trunk(branch, branch.p0, t, wind_xz);
    }
    let p1 = branches[branch.parent_index];
    if (p1.parent_index == PARENT_NONE) {
        let s1 = deform_trunk(p1, p1.p0, branch.parent_t, wind_xz);
        return deform_child(branch, t, s1, wind_xz);
    }
    let p2 = branches[p1.parent_index];
    if (p2.parent_index == PARENT_NONE) {
        let s2 = deform_trunk(p2, p2.p0, p1.parent_t, wind_xz);
        let s1 = deform_child(p1, branch.parent_t, s2, wind_xz);
        return deform_child(branch, t, s1, wind_xz);
    }
    let p3 = branches[p2.parent_index];
    if (p3.parent_index == PARENT_NONE) {
        let s3 = deform_trunk(p3, p3.p0, p2.parent_t, wind_xz);
        let s2 = deform_child(p2, p1.parent_t, s3, wind_xz);
        let s1 = deform_child(p1, branch.parent_t, s2, wind_xz);
        return deform_child(branch, t, s1, wind_xz);
    }
    let p4 = branches[p3.parent_index];
    let s4 = deform_trunk(p4, p4.p0, p3.parent_t, wind_xz);
    let s3 = deform_child(p3, p2.parent_t, s4, wind_xz);
    let s2 = deform_child(p2, p1.parent_t, s3, wind_xz);
    let s1 = deform_child(p1, branch.parent_t, s2, wind_xz);
    return deform_child(branch, t, s1, wind_xz);
}

@vertex
fn vertex(vert: TemplateVertex, @builtin(instance_index) instance_index: u32) -> WoodVertexOutput {
    var out: WoodVertexOutput;
    let branch = branches[compacted[instance_index]];
    let plant = plants[branch.plant_index];
    let t = vert.t_radial.x;
    let radial = vert.t_radial.y;
    let wind_now = wind_field(
        plant.root.xz,
        WindInputs(view.wind.xy, view.lod_distances.x, view.wind.w, view.world_position.w),
    );
    let sample = world_sample(branch, t, wind_now);
    var radius = branch_radius_at(branch.root_radius_m, branch.tip_radius_m, t);
    // Bounded junction collar: child tubes overlap the parent a little at t≈0.
    if (branch.parent_index != PARENT_NONE) {
        radius = radius * (1.0 + (1.0 - t) * 0.18);
    }
    if (radial < 0.0) {
        radius = 0.0;
    }
    let world = woody_surface_vertex(sample, radius, max(radial, 0.0));
    let n = woody_surface_normal(sample, max(radial, 0.0));
    let pal = VegetationPalette(vec3<f32>(0.18, 0.10, 0.055), vec3<f32>(0.50, 0.34, 0.16));
    out.position = view.clip_from_world * vec4<f32>(world, 1.0);
    out.world_normal = n;
    out.t = t;
    out.albedo = wood_albedo(pal, t, branch.keep_rank);
    return out;
}

@fragment
fn fragment(in: WoodVertexOutput) -> @location(0) vec4<f32> {
    let color = painterly_lit(in.albedo, in.world_normal, view.light_direction.xyz, in.t);
    return vec4<f32>(color, 1.0);
}
