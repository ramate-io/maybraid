use bevy::prelude::*;

/// Click payload stamped on a pickable leaf.
#[derive(Component, Clone, Copy, Debug)]
pub struct MenuButton<E: Copy + Send + Sync + 'static>(pub E);

/// Section header payload; the host maps the label to open-state.
#[derive(Component, Clone, Copy, Debug)]
pub struct ToggleSectionKey(pub &'static str);

/// Summary-row payload; the host opens an overlay keyed by this label.
#[derive(Component, Clone, Copy, Debug)]
pub struct OpenSelectKey(pub &'static str);

/// Dismiss control on the overlay backdrop or back button.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct CloseOverlaySelect;

/// Root of the stacked picker screen.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct OverlaySelectRoot;

/// Scroll viewport inside the overlay picker.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct OverlaySelectViewport;
