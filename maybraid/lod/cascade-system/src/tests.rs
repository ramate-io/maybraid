//! Integration tests and wiring examples for **`lod-cascade-system`**: cascade production,
//! requirement-signal garbage collection (default **vs** opt-in scheduling), chunk reaction, and
//! managed entity re-parenting.
//!
//! See [`test_utils`] for shared fixtures.

mod managed_reparent_integration;
mod opt_in_garbage_collection;
mod production_and_tracker;
mod test_utils;
