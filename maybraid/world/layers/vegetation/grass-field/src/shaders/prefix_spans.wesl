//! Prefix visible blade spans and write tiled indirect dispatch args.

import vegetation_shaders::wesl::lod::tiled_dispatch_groups;
import vegetation_shaders::wesl::layout::VisibleGroveSpan;

@group(0) @binding(0) var<storage, read_write> cull_info: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> blade_spans: array<VisibleGroveSpan>;
@group(0) @binding(2) var<storage, read_write> blade_dispatch: array<u32>;

@compute @workgroup_size(1, 1, 1)
fn prefix() {
    let n = atomicLoad(&cull_info[0]);
    var prefix = 0u;
    for (var i = 0u; i < n; i++) {
        blade_spans[i].prefix = prefix;
        prefix = prefix + blade_spans[i].count;
    }
    atomicStore(&cull_info[1], prefix);
    let groups = tiled_dispatch_groups(prefix);
    blade_dispatch[0] = groups.x;
    blade_dispatch[1] = groups.y;
    blade_dispatch[2] = groups.z;
}
