//! Diagnostic-only counters used by the checked-in Bevy 0.20.1 patch.
//! The ordinary workspace does not depend on patched Bevy or register observers.

#[cfg(feature = "diagnostics")]
mod capture;
#[cfg(feature = "diagnostics")]
pub use capture::{BufferKind, BufferSnapshot, Capture, Census};
