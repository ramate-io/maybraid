//! Per-blade visibility, nested density, and LOD. Compacts survivors into per-LOD lists.

import vegetation_shaders::wesl::lod::{catalog_select_index, lod_bucket, LodThresholds};
import vegetation_shaders::wesl::layout::{BladeDescriptor, GrassViewUniform};
import vegetation_shaders::wesl::wind::max_wind_displacement_m;
import vegetation_shaders::wesl::grove::{GroveVisibleSpan, span_step, SPAN_ITEM_NONE};

@group(0) @binding(0) var<uniform> view: GrassViewUniform;
@group(0) @binding(1) var<storage, read> blades: array<BladeDescriptor>;
@group(0) @binding(2) var<storage, read_write> counters: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> list_near: array<u32>;
@group(0) @binding(4) var<storage, read_write> list_mid: array<u32>;
@group(0) @binding(5) var<storage, read_write> list_card: array<u32>;
@group(0) @binding(6) var<storage, read> visible_spans: array<GroveVisibleSpan>;

fn sphere_in_frustum(center: vec3<f32>, radius: f32, clip: mat4x4<f32>) -> bool {
    let p = clip * vec4<f32>(center, 1.0);
    let slack = radius * length(clip[0].xyz) + 0.05;
    // `p.w` (not abs) so the mirrored pyramid behind the camera is rejected.
    let w = p.w + slack;
    return w > 0.0 && abs(p.x) <= w && abs(p.y) <= w && p.z >= -slack && p.z <= w;
}

fn append_list(lod: u32, blade_index: u32, capacity: u32) {
    let slot = atomicAdd(&counters[lod], 1u);
    if (slot >= capacity) {
        atomicAdd(&counters[lod + 4u], 1u);
        return;
    }
    if (lod == 0u) {
        list_near[slot] = blade_index;
    } else if (lod == 1u) {
        list_mid[slot] = blade_index;
    } else {
        list_card[slot] = blade_index;
    }
}

@compute @workgroup_size(64, 1, 1)
fn select(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>,
) {
    var blade_index = catalog_select_index(gid, nwg);
    var blade_count = u32(view.counts.x);
    let span_count = u32(view.light_direction.w);
    var bucket = 3u;
    if (span_count > 0u) {
        if (blade_index >= blade_count) {
            return;
        }
        var mapped_index = SPAN_ITEM_NONE;
        var cursor = 0u;
        for (var i = 0u; i < span_count; i++) {
            let step = span_step(visible_spans[i], blade_index, cursor, 0u);
            if (step.x == 1u) {
                mapped_index = step.y;
                bucket = step.z;
                break;
            }
            cursor += step.y;
        }
        if (mapped_index == SPAN_ITEM_NONE) {
            return;
        }
        blade_index = mapped_index;
    } else if (blade_index >= blade_count) {
        return;
    }
    let blade = blades[blade_index];
    if (blade.height_m <= 0.0) {
        return;
    }
    let capacity = u32(view.counts.y);
    let keep_fraction = min(1.0, f32(capacity) / max(f32(blade_count), 1.0));
    if (blade.shape.w >= keep_fraction) {
        return;
    }
    if (span_count == 0u) {
        let radius = max_wind_displacement_m(blade.height_m, view.wind.w, blade.shape.z)
            + blade.height_m
            + blade.width_m * 2.4;
        let center = blade.root + blade.normal * (blade.height_m * 0.5);
        if (!sphere_in_frustum(center, radius, view.clip_from_world)) {
            return;
        }
        let distance = length(view.world_position.xyz - blade.root);
        let thresholds = LodThresholds(
            view.lod_distances.y,
            view.lod_distances.z,
            view.lod_distances.w,
            view.lod_density.x,
            view.lod_density.y,
            view.lod_density.z,
            view.lod_density.w,
        );
        bucket = lod_bucket(
            distance,
            blade.height_m,
            blade.shape.w,
            view.resolution.y,
            view.resolution.z,
            thresholds,
        );
    }
    if (bucket > 2u) {
        return;
    }
    append_list(bucket, blade_index, capacity);
}
