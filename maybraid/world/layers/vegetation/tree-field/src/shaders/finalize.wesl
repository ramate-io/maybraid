//! Writes six `draw_indexed_indirect` args (wood then foliage). No CPU readback.

import vegetation_shaders::wesl::layout::{DrawIndexedIndirect, GrassViewUniform};

@group(0) @binding(0) var<uniform> view: GrassViewUniform;
@group(0) @binding(1) var<storage, read_write> counters: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> indirect: array<DrawIndexedIndirect>;

@compute @workgroup_size(1, 1, 1)
fn finalize() {
    let capacity = u32(view.counts.y);
    var wood_indices = array<u32, 3>(264u, 126u, 60u);
    var foliage_indices = array<u32, 3>(21u, 21u, 21u);
    for (var lod = 0u; lod < 3u; lod++) {
        var wood: DrawIndexedIndirect;
        wood.index_count = wood_indices[lod];
        wood.instance_count = min(atomicLoad(&counters[lod]), capacity);
        wood.first_index = 0u;
        wood.base_vertex = 0;
        wood.first_instance = 0u;
        indirect[lod] = wood;
        var foliage: DrawIndexedIndirect;
        foliage.index_count = foliage_indices[lod];
        foliage.instance_count = min(atomicLoad(&counters[lod + 3u]), capacity);
        foliage.first_index = 0u;
        foliage.base_vertex = 0;
        foliage.first_instance = 0u;
        indirect[lod + 3u] = foliage;
    }
}
