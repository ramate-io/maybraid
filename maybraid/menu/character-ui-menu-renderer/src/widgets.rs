use bevy::prelude::*;

/// Click payload stamped on a pickable leaf.
#[derive(Component, Clone, Copy, Debug)]
pub struct MenuButton<E: Copy + Send + Sync + 'static>(pub E);

/// Header payload; activate opens the overlay keyed by this IR label.
#[derive(Component, Clone, Copy, Debug)]
pub struct OpenSelectKey(pub &'static str);

/// Dismiss control on the overlay backdrop or back button.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CloseOverlaySelect;

/// Root of the stacked picker screen.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct OverlaySelectRoot;

/// Scroll viewport inside the overlay picker.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct OverlaySelectViewport;
