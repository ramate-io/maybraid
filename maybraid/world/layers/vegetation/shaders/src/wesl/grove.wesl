//! Visible-span layout after CPU grove culling. There is no GPU grove-cull pass.

struct GroveVisibleSpan {
    branch_off: u32,
    branch_count: u32,
    leaf_off: u32,
    leaf_count: u32,
}
