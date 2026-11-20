//! Folded leaf cards attached to the same deformed parent frame as the wood.

import vegetation_shaders::wesl::curve::{
    attachment_frame, branch_rest_sample, leaf_surface_normal, leaf_surface_vertex, stable_frame,
    CurveSample, LocalFrame,
};
import vegetation_shaders::wesl::wind::{deform_along_curve, wind_field, WindInputs};
import vegetation_shaders::wesl::shading::{crown_albedo, leaf_painterly_lit};
import vegetation_shaders::wesl::layout::{GrassDrawParams, GrassViewUniform};
import vegetation_bush_field::shaders::layout::{BranchDescriptor, LeafDescriptor, PlantDescriptor};

@group(0) @binding(0) var<uniform> view: GrassViewUniform;
@group(0) @binding(1) var<storage, read> plants: array<PlantDescriptor>;
@group(0) @binding(2) var<storage, read> branches: array<BranchDescriptor>;
@group(0) @binding(3) var<storage, read> leaves: array<LeafDescriptor>;
@group(0) @binding(4) var<storage, read> compacted: array<u32>;
@group(0) @binding(5) var<uniform> draw: GrassDrawParams;

const PARENT_NONE: u32 = 0xFFFFFFFFu;

struct TemplateVertex {
    @location(0) v_u: vec2<f32>,
}

struct LeafVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) v: f32,
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
        return deform_trunk(branch, plant.root, t, wind_xz);
    }
    let parent = branches[branch.parent_index];
    var parent_sample: CurveSample;
    if (parent.parent_index == PARENT_NONE) {
        parent_sample = deform_trunk(parent, plant.root, branch.parent_t, wind_xz);
    } else {
        let grand = branches[parent.parent_index];
        let grand_sample = deform_trunk(grand, plant.root, parent.parent_t, wind_xz);
        parent_sample = deform_child(parent, branch.parent_t, grand_sample, wind_xz);
    }
    return deform_child(branch, t, parent_sample, wind_xz);
}

@vertex
fn vertex(vert: TemplateVertex, @builtin(instance_index) instance_index: u32) -> LeafVertexOutput {
    var out: LeafVertexOutput;
    let leaf = leaves[compacted[instance_index]];
    let plant = plants[leaf.plant_index];
    var parent_index = leaf.parent_branch;
    var parent = branches[parent_index];
    if (draw.lod_bucket >= 1u && parent.depth > 1u) {
        parent_index = parent.major_parent;
        if (parent_index != PARENT_NONE) {
            parent = branches[parent_index];
        }
    }
    let wind_now = wind_field(
        plant.root.xz,
        WindInputs(view.wind.xy, view.lod_distances.x, view.wind.w, view.world_position.w),
    );
    let sample = world_sample(parent, leaf.attach_t, wind_now);
    let attach = attachment_frame(sample, leaf.twist);
    let outward = normalize(leaf.facing + attach.along * 0.28 + vec3<f32>(0.0, 0.22, 0.0));
    let across = normalize(cross(outward, attach.facing));
    let facing = cross(across, outward);
    let frame = LocalFrame(across, outward, facing);
    let v = vert.v_u.x;
    let u = vert.v_u.y;
    let size_scale = select(select(1.0, 1.22, draw.lod_bucket == 1u), 1.55, draw.lod_bucket == 2u);
    var world = leaf_surface_vertex(sample.position, frame, leaf.size_m * size_scale, u, v, leaf.fold);
    let flutter = sin(view.world_position.w * 4.2 + f32(leaf.leaf_id) * 0.013) * leaf.flutter * v;
    world = world + frame.across * flutter;
    let n = leaf_surface_normal(frame, u, leaf.fold);
    let height_frac = saturate((world.y - plant.root.y) / max(plant.height_m, 1e-3));
    let albedo = crown_albedo(
        leaf.palette,
        vec3<f32>(0.05, 0.22, 0.10),
        height_frac,
        leaf.density_rank,
    );
    out.position = view.clip_from_world * vec4<f32>(world, 1.0);
    out.world_normal = n;
    out.v = v;
    out.albedo = albedo;
    return out;
}

@fragment
fn fragment(in: LeafVertexOutput) -> @location(0) vec4<f32> {
    let color = leaf_painterly_lit(in.albedo, in.world_normal, view.light_direction.xyz, in.v);
    return vec4<f32>(color, 1.0);
}
