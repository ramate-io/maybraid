use bevy::prelude::*;
use character_ui_menu::CameraFocus;

/// Outbound messages after the host applies a leaf or moves camera focus.
#[derive(Clone, Debug, Message, PartialEq)]
pub enum CharacterMenuEvent<E> {
	CameraFocus(CameraFocus),
	Menu(E),
}

/// In-screen: open the overlay keyed by this IR label.
#[derive(EntityEvent, Clone, Copy, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct OverlayOpen {
	pub entity: Entity,
	pub key: &'static str,
}

/// In-screen: dismiss the open overlay.
#[derive(EntityEvent, Clone, Copy, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct OverlayClose {
	pub entity: Entity,
}
