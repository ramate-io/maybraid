//! After grove cull: write compute indirect dispatch grids for pass 2.

import vegetation_shaders::wesl::lod::select_dispatch_grid;

@group(0) @binding(0) var<storage, read_write> counters: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> indirect: array<u32>;

const COUNTER_VISIBLE_BRANCHES: u32 = 10u;
const COUNTER_VISIBLE_LEAVES: u32 = 11u;

@compute @workgroup_size(1, 1, 1)
fn write_wood(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x != 0u) {
        return;
    }
    let grid = select_dispatch_grid(atomicLoad(&counters[COUNTER_VISIBLE_BRANCHES]));
    indirect[0] = grid.x;
    indirect[1] = grid.y;
    indirect[2] = grid.z;
}

@compute @workgroup_size(1, 1, 1)
fn write_foliage(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x != 0u) {
        return;
    }
    let grid = select_dispatch_grid(atomicLoad(&counters[COUNTER_VISIBLE_LEAVES]));
    indirect[3] = grid.x;
    indirect[4] = grid.y;
    indirect[5] = grid.z;
}