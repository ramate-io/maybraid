//! CPU-selected packed rendering for orchard groves ([#956](https://github.com/ramate-io/maybraid/issues/956)).
//!
//! Orchard tiles keep the existing grow and stick-capsule path. Their kit
//! instances are grouped by shared GLB and material, uploaded once, and drawn
//! with one direct call per group. Other vegetation stays on the host path.
//!
//! # Launch
//!
//! * `MAYBRAID_PACKED_GROVES=orchard` (also `1`, `on`, `true`) enables the path.
//!   Unset leaves the existing host renderer.
//! * `MAYBRAID_PACKED_GROVE_EMPHASIS=ag-town` pins Discovery's forest layering to
//!   Ag Town so the upper canopy is orchard-weighted. Leave it unset to measure
//!   the ordinary mixed world.
//! * `MAYBRAID_PACKED_GROVE_BUDGET_MB` caps retained instance payload (default 256).
//! * `MAYBRAID_PACKED_GROVE_METRICS=1` logs counters about twice a second.
//!
//! # Capture
//!
//! Same seed, resolution, and camera route for both modes. Fixed start:
//! `MAYBRAID_START_AT` if the shell already uses it, plus
//! `MAYBRAID_PACKED_GROVES` toggled off then `orchard`. Read Tracy zones
//! `write_binned_instance_buffers<Opaque3d>`, `packed_grove_select`, and
//! `packed_grove_upload`, and the metrics log for instance, draw, upload, and
//! retained-byte counts. A lower binned-write time only counts if those packed
//! zones did not absorb the same work.
//!
//! This slice does not assume a win. Compare warm traversal (no new groves),
//! cold streaming, one invalidated grove, and a return to retained then evicted
//! content before deciding to expand the family.

mod cache;
mod instances;
mod kit;
mod mode;
mod render;
mod select;
mod visual;

#[cfg(test)]
mod tests;

pub(crate) use cache::PackedGroveCell;
pub(crate) use mode::PackMode;
pub(crate) use render::PackedGrovePlugin;
