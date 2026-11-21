//! Layered canopy cards attached to the same deformed parent frame as the wood.
//! Close-up style is packed in plant flags: smooth clumps, hashed dither, or crisp cutout.

import vegetation_shaders::wesl::curve::{
    attachment_frame, branch_rest_sample, leaf_surface_normal, leaf_surface_vertex, stable_frame,
    CurveSample, LocalFrame,
};
import vegetation_shaders::wesl::wind::{deform_along_curve, wind_field, WindInputs};
import vegetation_shaders::wesl::shading::{
    canopy_clump_albedo, canopy_interior_shade, crown_layer_albedo, grass_variation_palette,
    leaf_close_uses_cluster, leaf_cluster_alpha_for_style, leaf_cluster_keep_for_style,
    leaf_cluster_outward_normal, leaf_cluster_tint, leaf_painterly_lit,
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
const DRAPE_SHAPE: f32 = 1.5;

struct TemplateVertex {
    @location(0) v_u: vec2<f32>,
}

struct LeafVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) albedo: vec3<f32>,
    @location(3) clump_out: vec3<f32>,
    @location(4) pack: vec4<f32>,
    @location(5) interior: vec3<f32>,
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

fn lod_detail(lod: u32) -> f32 {
    if (lod == 0u) {
        return 1.0;
    }
    if (lod == 1u) {
        return 0.42;
    }
    return 0.0;
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
    let is_drape = leaf.shape >= DRAPE_SHAPE;
    let shell = select(saturate(leaf.shape), 0.85, is_drape);
    let along_mix = mix(0.48, 0.08, shell);
    let up_mix = mix(0.42, 0.18, shell);
    var outward = normalize(
        leaf.facing * mix(0.62, 1.0, shell) + attach.along * along_mix + vec3<f32>(0.0, up_mix, 0.0),
    );
    if (is_drape) {
        outward = normalize(mix(-vec3<f32>(0.0, 1.0, 0.0), leaf.facing, 0.18) + attach.across * 0.08);
    }
    let across = normalize(cross(outward, attach.facing));
    let facing = cross(across, outward);
    let frame = LocalFrame(across, outward, facing);
    let v = vert.v_u.x;
    let u = vert.v_u.y;
    let size_scale = select(select(1.0, 1.28, draw.lod_bucket == 1u), 1.72, draw.lod_bucket == 2u);
    let card = leaf.size_m * size_scale;
    var world: vec3<f32>;
    if (is_drape) {
        let width = card * 0.14;
        let hang = v * card;
        let sag = hang * (0.42 + 0.58 * v);
        let sway = wind_now * (v * v) * (1.0 - parent.stiffness) * 0.55;
        world = sample.position
            + across * (u * width)
            + outward * sag
            + vec3<f32>(sway.x, 0.0, sway.y);
        let flutter = sin(view.world_position.w * 3.4 + f32(leaf.leaf_id) * 0.017) * leaf.flutter * v;
        world = world + across * flutter * 1.6;
    } else {
        world = leaf_surface_vertex(sample.position, frame, card, u, v, leaf.fold);
        world = world + outward * (card * mix(0.22, 0.78, shell));
        let flutter = sin(view.world_position.w * 2.6 + f32(leaf.leaf_id) * 0.011) * leaf.flutter * v;
        world = world + frame.across * flutter + frame.facing * (flutter * 0.22);
    }
    var n = leaf_surface_normal(frame, u, leaf.fold);
    let style = (plant.flags >> 1u) & 3u;
    let cluster_detail = select(0.0, lod_detail(draw.lod_bucket), leaf_close_uses_cluster(style));
    n = leaf_cluster_outward_normal(n, outward, vec2<f32>(u, v), cluster_detail);
    let height_frac = saturate((world.y - plant.root.y) / max(plant.height_m, 1e-3));
    let radial = saturate(length(world.xz - plant.root.xz) / max(plant.crown_radius_m, 1e-3));
    let pal = grass_variation_palette(leaf.palette, plant.root.xz, leaf.leaf_id >> 3u);
    let layered = crown_layer_albedo(pal, height_frac, radial, shell, leaf.density_rank);
    var albedo = canopy_clump_albedo(layered, pal.root_rgb, height_frac, n, leaf.density_rank);
    if (leaf_close_uses_cluster(style)) {
        let tint = leaf_cluster_tint(vec2<f32>(u, v), leaf.leaf_id);
        albedo = albedo * mix(vec3<f32>(0.82, 0.90, 0.88), vec3<f32>(1.14, 1.06, 0.90), tint);
    }
    albedo = albedo * canopy_interior_shade(height_frac, radial, shell, n);
    out.position = view.clip_from_world * vec4<f32>(world, 1.0);
    out.world_normal = n;
    out.uv = vec2<f32>(u, v);
    out.albedo = albedo;
    out.clump_out = outward;
    out.pack = vec4<f32>(
        lod_detail(draw.lod_bucket),
        f32(leaf.leaf_id),
        select(0.0, 1.0, is_drape),
        f32(style),
    );
    out.interior = vec3<f32>(height_frac, radial, shell);
    return out;
}

@fragment
fn fragment(in: LeafVertexOutput) -> @location(0) vec4<f32> {
    let detail = in.pack.x;
    let seed = u32(in.pack.y);
    let style = u32(in.pack.w + 0.5);
    var alpha = leaf_cluster_alpha_for_style(in.uv, seed, detail, style);
    if (in.pack.z > 0.5) {
        // Drapes: hashed leaves when the style uses clusters; always a soft side falloff.
        let drape_uv = vec2<f32>(in.uv.x * 1.35, in.uv.y);
        let drape_detail = max(detail, select(0.0, 0.55, leaf_close_uses_cluster(style)));
        alpha = leaf_cluster_alpha_for_style(drape_uv, seed ^ 0x9e3779b9u, drape_detail, style);
        alpha = alpha * (1.0 - smoothstep(0.78, 1.0, abs(in.uv.x)));
    }
    if (!leaf_cluster_keep_for_style(alpha, in.position.xy, seed, detail, style)) {
        discard;
    }
    let color = leaf_painterly_lit(in.albedo, in.world_normal, view.light_direction.xyz, in.uv.y);
    return vec4<f32>(color, alpha);
}
