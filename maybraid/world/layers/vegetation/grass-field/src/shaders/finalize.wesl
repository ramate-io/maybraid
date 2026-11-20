//! Writes `draw_indexed_indirect` args from compacted counters. No CPU readback.

import vegetation_shaders::wesl::layout::{DrawIndexedIndirect, GrassViewUniform};

@group(0) @binding(0) var<uniform> view: GrassViewUniform;
@group(0) @binding(1) var<storage, read_write> counters: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> indirect: array<DrawIndexedIndirect>;

@compute @workgroup_size(1, 1, 1)
fn finalize() {
    let capacity = u32(view.counts.y);
    var index_counts = array<u32, 3>(30u, 12u, 6u);
    for (var lod = 0u; lod < 3u; lod++) {
        let count = min(atomicLoad(&counters[lod]), capacity);
        var args: DrawIndexedIndirect;
        args.index_count = index_counts[lod];
        args.instance_count = count;
        args.first_index = 0u;
        args.base_vertex = 0;
        args.first_instance = 0u;
        indirect[lod] = args;
    }
}
