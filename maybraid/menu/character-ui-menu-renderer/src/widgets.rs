use bevy::prelude::*;

/// Click payload stamped on a pickable leaf.
#[derive(Component, Clone, Copy, Debug)]
pub struct MenuButton<E: Copy + Send + Sync + 'static>(pub E);

/// Section header payload; the host maps the label to open-state.
#[derive(Component, Clone, Copy, Debug)]
pub struct ToggleSectionKey(pub &'static str);
