//! Per-branch visibility, plant LOD, and topology-preserving prune.

import vegetation_shaders::wesl::lod::{
    catalog_select_index, lod_bucket, LodThresholds, sphere_in_frustum,
};
import vegetation_shaders::wesl::layout::{GrassViewUniform, VisibleGroveSpan};
import vegetation_shaders::wesl::wind::max_wind_displacement_m;
import vegetation_bush_field::shaders::layout::{PlantDescriptor, BranchDescriptor};

@group(0) @binding(0) var<uniform> view: GrassViewUniform;
@group(0) @binding(1) var<storage, read> plants: array<PlantDescriptor>;
@group(0) @binding(2) var<storage, read> branches: array<BranchDescriptor>;
@group(0) @binding(3) var<storage, read_write> counters: array<atomic<u32>>;
@group(0) @binding(4) var<storage, read_write> list_near: array<u32>;
@group(0) @binding(5) var<storage, read_write> list_mid: array<u32>;
@group(0) @binding(6) var<storage, read_write> list_far: array<u32>;
@group(0) @binding(7) var<storage, read> spans: array<VisibleGroveSpan>;
@group(0) @binding(8) var<storage, read> cull_info: array<u32>;

const PARENT_NONE: u32 = 0xFFFFFFFFu;

fn span_item_index(compact_index: u32, span_count: u32) -> vec2<u32> {
    var lo = 0u;
    var hi = span_count;
    for (var step = 0u; step < 32u; step++) {
        if (lo >= hi) {
            break;
        }
        let mid = (lo + hi) / 2u;
        let span = spans[mid];
        if (compact_index < span.prefix) {
            hi = mid;
        } else if (compact_index >= span.prefix + span.count) {
            lo = mid + 1u;
        } else {
            return vec2<u32>(span.offset + (compact_index - span.prefix), span.lod);
        }
    }
    return vec2<u32>(0xFFFFFFFFu, 3u);
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
        return 2u;
    }
    if (lod == 1u) {
        return 1u;
    }
    return 0u;
}

fn append_list(lod: u32, index: u32, capacity: u32) {
    let slot = atomicAdd(&counters[lod], 1u);
    if (slot >= capacity) {
        atomicAdd(&counters[lod + 6u], 1u);
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
fn select(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>,
) {
    let compact_index = catalog_select_index(gid, nwg);
    let total = cull_info[2];
    if (compact_index >= total) {
        return;
    }
    let found = span_item_index(compact_index, cull_info[0]);
    let branch_index = found.x;
    if (branch_index == 0xFFFFFFFFu) {
        return;
    }
    let branch_count = max(total, 1u);
    let branch = branches[branch_index];
    let plant = plants[branch.plant_index];
    let bucket = plant_bucket(plant);
    if (bucket > 2u) {
        return;
    }
    if (branch.depth > max_depth_for(bucket)) {
        return;
    }
    let capacity = u32(view.counts.y);
    let keep_fraction = min(1.0, f32(capacity) / max(f32(branch_count), 1.0));
    if (branch.keep_rank >= keep_fraction && branch.parent_index != PARENT_NONE) {
        return;
    }
    append_list(bucket, branch_index, capacity);
}
