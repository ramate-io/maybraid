//! Per-leaf visibility. Dropped twigs reattach foliage to `major_parent`.

import vegetation_shaders::wesl::lod::{lod_bucket, nested_keep, LodThresholds};
import vegetation_shaders::wesl::layout::GrassViewUniform;
import vegetation_shaders::wesl::wind::max_wind_displacement_m;
import vegetation_tree_field::shaders::layout::{PlantDescriptor, BranchDescriptor, LeafDescriptor};

@group(0) @binding(0) var<uniform> view: GrassViewUniform;
@group(0) @binding(1) var<storage, read> plants: array<PlantDescriptor>;
@group(0) @binding(2) var<storage, read> branches: array<BranchDescriptor>;
@group(0) @binding(3) var<storage, read> leaves: array<LeafDescriptor>;
@group(0) @binding(4) var<storage, read_write> counters: array<atomic<u32>>;
@group(0) @binding(5) var<storage, read_write> list_near: array<u32>;
@group(0) @binding(6) var<storage, read_write> list_mid: array<u32>;
@group(0) @binding(7) var<storage, read_write> list_far: array<u32>;

const PARENT_NONE: u32 = 0xFFFFFFFFu;

fn sphere_in_frustum(center: vec3<f32>, radius: f32, clip: mat4x4<f32>) -> bool {
    let p = clip * vec4<f32>(center, 1.0);
    let slack = radius * length(clip[0].xyz) + 0.05;
    let w = p.w + slack;
    return w > 0.0 && abs(p.x) <= w && abs(p.y) <= w && p.z >= -slack && p.z <= w;
}

fn plant_bucket(plant: PlantDescriptor) -> u32 {
    if ((plant.flags & 1u) == 0u) {
        return 3u;
    }
    let radius = max_wind_displacement_m(plant.height_m, view.wind.w, plant.wood_stiffness)
        + max(plant.height_m, plant.crown_radius_m)
        + plant.crown_radius_m * 0.35;
    let center = plant.root + plant.up * (plant.height_m * 0.5);
    if (!sphere_in_frustum(center, radius, view.clip_from_world)) {
        return 3u;
    }
    let distance = length(view.world_position.xyz - plant.root);
    let thresholds = LodThresholds(
        view.lod_distances.y,
        view.lod_distances.z,
        view.lod_distances.w,
        view.lod_density.x,
        view.lod_density.y,
        view.lod_density.z,
        view.lod_density.w,
    );
    return lod_bucket(
        distance,
        plant.height_m,
        0.0,
        view.resolution.y,
        view.resolution.z,
        thresholds,
    );
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

fn density_for(lod: u32) -> f32 {
    if (lod == 0u) {
        return view.lod_density.x;
    }
    if (lod == 1u) {
        return view.lod_density.y;
    }
    return view.lod_density.z;
}

fn append_list(lod: u32, index: u32, capacity: u32) {
    let slot = atomicAdd(&counters[lod + 3u], 1u);
    if (slot >= capacity) {
        atomicAdd(&counters[lod + 9u], 1u);
        return;
    }
    if (lod == 0u) {
        list_near[slot] = index;
    } else if (lod == 1u) {
        list_mid[slot] = index;
    } else {
        list_far[slot] = index;
    }
}

@compute @workgroup_size(64, 1, 1)
fn select(@builtin(global_invocation_id) gid: vec3<u32>) {
    let leaf_index = gid.x;
    let leaf_count = u32(view.counts.z);
    if (leaf_index >= leaf_count) {
        return;
    }
    let leaf = leaves[leaf_index];
    let plant = plants[leaf.plant_index];
    let bucket = plant_bucket(plant);
    if (bucket > 2u) {
        return;
    }
    var parent_index = leaf.parent_branch;
    var parent = branches[parent_index];
    var hops = 0u;
    loop {
        if (parent.depth <= max_depth_for(bucket) || hops >= 3u) {
            break;
        }
        if (parent.major_parent == PARENT_NONE) {
            return;
        }
        parent_index = parent.major_parent;
        parent = branches[parent_index];
        hops = hops + 1u;
    }
    if (parent.depth > max_depth_for(bucket)) {
        return;
    }
    if (!nested_keep(leaf.density_rank, density_for(bucket))) {
        return;
    }
    append_list(bucket, leaf_index, u32(view.counts.y));
}
