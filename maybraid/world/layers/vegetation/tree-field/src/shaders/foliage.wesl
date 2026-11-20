//! Layered canopy cards attached to the same deformed parent frame as the wood.

import vegetation_shaders::wesl::curve::{
    attachment_frame, branch_rest_sample, leaf_surface_normal, leaf_surface_vertex, stable_frame,
    CurveSample, LocalFrame,
};
import vegetation_shaders::wesl::wind::{deform_along_curve, wind_field, WindInputs};
import vegetation_shaders::wesl::shading::{
    canopy_clump_albedo, crown_layer_albedo, grass_variation_palette, leaf_painterly_lit,
};
import vegetation_shaders::wesl::layout::{GrassDrawParams, GrassViewUniform};
import vegetation_tree_field::shaders::layout::{BranchDescriptor, LeafDescriptor, PlantDescriptor};

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
    let p1 = branches[branch.parent_index];
    if (p1.parent_index == PARENT_NONE) {
        let s1 = deform_trunk(p1, plant.root, branch.parent_t, wind_xz);
        return deform_child(branch, t, s1, wind_xz);
    }
    let p2 = branches[p1.parent_index];
    if (p2.parent_index == PARENT_NONE) {
        let s2 = deform_trunk(p2, plant.root, p1.parent_t, wind_xz);
        let s1 = deform_child(p1, branch.parent_t, s2, wind_xz);
        return deform_child(branch, t, s1, wind_xz);
    }
    let p3 = branches[p2.parent_index];
    if (p3.parent_index == PARENT_NONE) {
        let s3 = deform_trunk(p3, plant.root, p2.parent_t, wind_xz);
        let s2 = deform_child(p2, p1.parent_t, s3, wind_xz);
        let s1 = deform_child(p1, branch.parent_t, s2, wind_xz);
        return deform_child(branch, t, s1, wind_xz);
    }
    let p4 = branches[p3.parent_index];
    let s4 = deform_trunk(p4, plant.root, p3.parent_t, wind_xz);
    let s3 = deform_child(p3, p2.parent_t, s4, wind_xz);
    let s2 = deform_child(p2, p1.parent_t, s3, wind_xz);
    let s1 = deform_child(p1, branch.parent_t, s2, wind_xz);
    return deform_child(branch, t, s1, wind_xz);
}

fn max_depth_for(lod: u32) -> u32 {
    if (lod == 0u) {
        return 3u;
    }
    if (lod == 1u) {
        return 2u;
    }
    return 1u;
}

@vertex
fn vertex(vert: TemplateVertex, @builtin(instance_index) instance_index: u32) -> LeafVertexOutput {
    var out: LeafVertexOutput;
    let leaf = leaves[compacted[instance_index]];
    let plant = plants[leaf.plant_index];
    var parent_index = leaf.parent_branch;
    var parent = branches[parent_index];
    var hops = 0u;
    loop {
        if (parent.depth <= max_depth_for(draw.lod_bucket) || hops >= 3u) {
            break;
        }
        if (parent.major_parent == PARENT_NONE) {
            break;
        }
        parent_index = parent.major_parent;
        parent = branches[parent_index];
        hops = hops + 1u;
    }
    let wind_now = wind_field(
        plant.root.xz,
        WindInputs(view.wind.xy, view.lod_distances.x, view.wind.w, view.world_position.w),
    );
    let sample = world_sample(parent, leaf.attach_t, wind_now);
    let attach = attachment_frame(sample, leaf.twist);
    let shell = saturate(leaf.shape);
    let along_mix = mix(0.48, 0.08, shell);
    let up_mix = mix(0.42, 0.18, shell);
    let outward = normalize(
        leaf.facing * mix(0.62, 1.0, shell) + attach.along * along_mix + vec3<f32>(0.0, up_mix, 0.0),
    );
    let across = normalize(cross(outward, attach.facing));
    let facing = cross(across, outward);
    let frame = LocalFrame(across, outward, facing);
    let v = vert.v_u.x;
    let u = vert.v_u.y;
    let size_scale = select(select(1.0, 1.28, draw.lod_bucket == 1u), 1.72, draw.lod_bucket == 2u);
    var world = leaf_surface_vertex(sample.position, frame, leaf.size_m * size_scale, u, v, leaf.fold);
    // Push outer shells into the crown volume so cards form clumps, not bark ivy.
    world = world + outward * (leaf.size_m * size_scale * mix(0.18, 0.62, shell));
    let flutter = sin(view.world_position.w * 2.6 + f32(leaf.leaf_id) * 0.011) * leaf.flutter * v;
    world = world + frame.across * flutter + frame.facing * (flutter * 0.22);
    let n = leaf_surface_normal(frame, u, leaf.fold);
    let height_frac = saturate((world.y - plant.root.y) / max(plant.height_m, 1e-3));
    let radial = saturate(length(world.xz - plant.root.xz) / max(plant.crown_radius_m, 1e-3));
    let pal = grass_variation_palette(leaf.palette, plant.root.xz, leaf.leaf_id >> 3u);
    let layered = crown_layer_albedo(pal, height_frac, radial, shell, leaf.density_rank);
    let albedo = canopy_clump_albedo(layered, pal.root_rgb, height_frac, n, leaf.density_rank);
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
