//! Pass 1: whole-patch frustum and residency cull; build visible blade spans.

import vegetation_shaders::wesl::grove::{
    GroveVisibleSpan, aabb_in_frustum,
};
import vegetation_shaders::wesl::lod::{lod_bucket, LodThresholds};
import vegetation_shaders::wesl::layout::{GrassPatchHeader, GrassViewUniform};
import vegetation_shaders::wesl::lod::catalog_select_index;

@group(0) @binding(0) var<uniform> view: GrassViewUniform;
@group(0) @binding(1) var<storage, read> patches: array<GrassPatchHeader>;
@group(0) @binding(2) var<storage, read_write> counters: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> visible_spans: array<GroveVisibleSpan>;

const COUNTER_VISIBLE_SPANS: u32 = 9u;
const COUNTER_VISIBLE_BRANCHES: u32 = 10u;

fn patch_active(header: GrassPatchHeader) -> bool {
    return (header.flags & 1u) != 0u;
}

fn patch_center(header: GrassPatchHeader) -> vec3<f32> {
    return (header.aabb_min + header.aabb_max) * 0.5;
}

fn patch_radius(header: GrassPatchHeader) -> f32 {
    return length(header.aabb_max - header.aabb_min) * 0.5;
}

@compute @workgroup_size(64, 1, 1)
fn cull_groves(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>,
) {
    let slot_count = u32(view.counts.w);
    let patch_index = catalog_select_index(gid, nwg);
    if (patch_index >= slot_count) {
        return;
    }
    let header = patches[patch_index];
    if (!patch_active(header) || header.blade_count == 0u) {
        return;
    }
    if (!aabb_in_frustum(header.aabb_min, header.aabb_max, view.clip_from_world)) {
        return;
    }
    let center = patch_center(header);
    let camera = view.world_position.xyz;
    let distance = length(camera - center);
    let thresholds = LodThresholds(
        view.lod_distances.y,
        view.lod_distances.z,
        view.lod_distances.w,
        view.lod_density.x,
        view.lod_density.y,
        view.lod_density.z,
        view.lod_density.w,
    );
    let height = patch_radius(header) * 2.0;
    let bucket = lod_bucket(
        distance,
        height,
        0.0,
        view.resolution.y,
        view.resolution.z,
        thresholds,
    );
    if (bucket > 2u) {
        return;
    }
    let span_slot = atomicAdd(&counters[COUNTER_VISIBLE_SPANS], 1u);
    visible_spans[span_slot] = GroveVisibleSpan(
        header.blade_offset,
        header.blade_count,
        0u,
        0u,
    );
    atomicAdd(&counters[COUNTER_VISIBLE_BRANCHES], header.blade_count);
}