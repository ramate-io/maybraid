//! Braidman preset notes.
//!
//! The first implementation reuses the shared [`crate::GenderPreset`] and
//! [`crate::BuildPreset`] enums directly. Braidman-specific preset behavior lives
//! in `pose`, where those preset IDs become small bone-scale refinement layers.
