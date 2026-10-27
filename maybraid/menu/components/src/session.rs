//! State shared by widgets inside one independent UI session.
use bevy::prelude::*;
/// Modal and text ownership for one UI root, independent of the primary host resources.
#[derive(Component, Debug, Default, Clone)]
#[require(crate::ScopedMenuSession)]
pub struct MenuSession {
	pub modals: crate::MenuModalStack,
	pub text_locked: bool,
	pub text: crate::ShortTextModal,
	/// The keypad created by the scoped text UI adapter.
	pub text_root: Option<Entity>,
}
