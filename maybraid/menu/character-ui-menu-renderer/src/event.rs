use bevy::prelude::*;
use character_ui_menu::CameraFocus;

/// Outbound messages emitted after the host applies a widget press.
#[derive(Clone, Debug, Message, PartialEq)]
pub enum CharacterMenuEvent<M> {
	CameraFocus(CameraFocus),
	MenuUpdate(M),
}
