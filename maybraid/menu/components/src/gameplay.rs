//! Markers for in-world HUD that yields to a full-screen focus overlay.

use bevy::prelude::*;

/// Stamp on gameplay HUD roots that should hide while a focus overlay is up.
///
/// The skill-map tunnel-vision pass hides these when RB expands the overlay and
/// leaves them alone on release so each HUD's own sync can restore visibility.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GameplayHud;
