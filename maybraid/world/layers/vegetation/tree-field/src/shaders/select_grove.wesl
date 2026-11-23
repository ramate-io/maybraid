//! Pass 1: whole-grove frustum and residency cull; build visible item spans.

import vegetation_shaders::wesl::grove::{
    WoodyGroveRecord, GroveVisibleSpan, grove_active, grove_center, grove_radius, aabb_in_frustum,
};
import vegetation_shaders::wesl::lod::{lod_bucket, LodThresholds};
import vegetation_shaders::wesl::layout::GrassViewUniform;
import vegetation_shaders::wesl::lod::catalog_select_index;

@group(0) @binding(0) var<uniform> view: GrassViewUniform;
@group(0) @binding(1) var<storage, read> groves: array<WoodyGroveRecord>;
@group(0) @binding(2) var<storage, read_write> counters: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> visible_spans: array<GroveVisibleSpan>;

const COUNTER_VISIBLE_GROVES: u32 = 9u;
const COUNTER_VISIBLE_BRANCHES: u32 = 10u;
const COUNTER_VISIBLE_LEAVES: u32 = 11u;

@compute @workgroup_size(64, 1, 1)
fn cull_groves(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>,
) {
    let slot_count = u32(view.counts.w);
    let grove_index = catalog_select_index(gid, nwg);
    if (grove_index >= slot_count) {
        return;
    }
    let record = groves[grove_index];
    if (!grove_active(record)) {
        return;
    }
    if (!aabb_in_frustum(record.aabb_min, record.aabb_max, view.clip_from_world)) {
        return;
    }
    let center = grove_center(record);
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
    let height = grove_radius(record) * 2.0;
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
    let span_slot = atomicAdd(&counters[COUNTER_VISIBLE_GROVES], 1u);
    visible_spans[span_slot] = GroveVisibleSpan(
        record.branch_off,
        record.branch_count,
        record.leaf_off,
        record.leaf_count,
    );
    atomicAdd(&counters[COUNTER_VISIBLE_BRANCHES], record.branch_count);
    atomicAdd(&counters[COUNTER_VISIBLE_LEAVES], record.leaf_count);
}