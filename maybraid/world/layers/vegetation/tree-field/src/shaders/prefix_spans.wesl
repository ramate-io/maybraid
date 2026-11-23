//! Prefix visible spans and write tiled indirect dispatch args.

import vegetation_shaders::wesl::lod::tiled_dispatch_groups;
import vegetation_shaders::wesl::layout::VisibleGroveSpan;

@group(0) @binding(0) var<storage, read_write> cull_info: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> wood_spans: array<VisibleGroveSpan>;
@group(0) @binding(2) var<storage, read_write> leaf_spans: array<VisibleGroveSpan>;
@group(0) @binding(3) var<storage, read_write> wood_dispatch: array<u32>;
@group(0) @binding(4) var<storage, read_write> leaf_dispatch: array<u32>;

@compute @workgroup_size(1, 1, 1)
fn prefix() {
    let wood_n = atomicLoad(&cull_info[0]);
    var wood_prefix = 0u;
    for (var i = 0u; i < wood_n; i++) {
        wood_spans[i].prefix = wood_prefix;
        wood_prefix = wood_prefix + wood_spans[i].count;
    }
    let leaf_n = atomicLoad(&cull_info[1]);
    var leaf_prefix = 0u;
    for (var i = 0u; i < leaf_n; i++) {
        leaf_spans[i].prefix = leaf_prefix;
        leaf_prefix = leaf_prefix + leaf_spans[i].count;
    }
    atomicStore(&cull_info[2], wood_prefix);
    atomicStore(&cull_info[3], leaf_prefix);
    let wood_groups = tiled_dispatch_groups(wood_prefix);
    wood_dispatch[0] = wood_groups.x;
    wood_dispatch[1] = wood_groups.y;
    wood_dispatch[2] = wood_groups.z;
    let leaf_groups = tiled_dispatch_groups(leaf_prefix);
    leaf_dispatch[0] = leaf_groups.x;
    leaf_dispatch[1] = leaf_groups.y;
    leaf_dispatch[2] = leaf_groups.z;
}
