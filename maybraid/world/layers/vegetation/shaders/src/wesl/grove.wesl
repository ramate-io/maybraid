//! Visible-span layout after CPU grove culling. There is no GPU grove-cull pass.

struct GroveVisibleSpan {
    branch_off: u32,
    branch_count: u32,
    leaf_off: u32,
    leaf_count: u32,
    lod: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

const SPAN_ITEM_NONE: u32 = 0xFFFFFFFFu;

/// One step of the visible-span walk. Naga rejects storage-array pointers as
/// function arguments, so the five `select` entry points keep the short loop
/// and call this per loaded span.
///
/// `.x == 1`: hit, catalog index in `.y`, grove lod in `.z`.
/// `.x == 0`: miss, `.y` is the span count to add to the cursor.
/// Do not call WGSL `select()` here: those entry points are named `select`.
fn span_step(span: GroveVisibleSpan, work_index: u32, cursor: u32, use_leaf: u32) -> vec3<u32> {
    var count = span.branch_count;
    var off = span.branch_off;
    if (use_leaf != 0u) {
        count = span.leaf_count;
        off = span.leaf_off;
    }
    if (work_index < cursor + count) {
        return vec3<u32>(1u, off + (work_index - cursor), span.lod);
    }
    return vec3<u32>(0u, count, 3u);
}
