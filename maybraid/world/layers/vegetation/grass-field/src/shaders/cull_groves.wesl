//! Pass 1: one thread per grove slot. Compact visible blade spans.

import vegetation_shaders::wesl::lod::{catalog_select_index, grove_lod_bucket};
import vegetation_shaders::wesl::layout::{GrassViewUniform, GroveRecord, VisibleGroveSpan};

@group(0) @binding(0) var<uniform> view: GrassViewUniform;
@group(0) @binding(1) var<storage, read> groves: array<GroveRecord>;
@group(0) @binding(2) var<storage, read_write> cull_info: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> blade_spans: array<VisibleGroveSpan>;

@compute @workgroup_size(64, 1, 1)
fn cull(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>,
) {
    let grove_index = catalog_select_index(gid, nwg);
    let grove_count = u32(view.counts.x);
    if (grove_index >= grove_count) {
        return;
    }
    let grove = groves[grove_index];
    let lod = grove_lod_bucket(grove.aabb_min, grove.aabb_max, grove.flags, view);
    if (lod > 2u) {
        return;
    }
    if (grove.plant_count > 0u) {
        let slot = atomicAdd(&cull_info[0], 1u);
        var span: VisibleGroveSpan;
        span.offset = grove.plant_off;
        span.count = grove.plant_count;
        span.prefix = 0u;
        span.lod = lod;
        blade_spans[slot] = span;
    }
}
