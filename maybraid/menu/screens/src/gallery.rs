//! Character gallery: pick a saved character or start a new one.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use crozon_character_model_user::{list_summaries, CharacterSummary};
use crozon_character_persist::{CharacterId, SaveRoot};
use maybraid_menu_controller::MenuController;
use menu_components::single_select::republish_menu_activate;
use menu_components::{
	screen_back_scene, TextCursorColumn, TextCursorRow, TextMenuPlugin, MENU_CLEAR,
};

use crate::input::add_menu_input;
use crate::show::take_menu_show_request;
use crate::MenuScreen;

/// Queue a gallery spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowGallery;

/// Marker on the spawned gallery root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct GalleryScreen;

/// Gallery destinations. Each pickable row stamps this as a component.
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum GalleryChoice {
	#[default]
	New,
	Open(CharacterId),
}

pub fn request_show_gallery(commands: &mut Commands) {
	commands.spawn(RequestShowGallery);
}

pub struct GalleryScreenPlugin;

impl Plugin for GalleryScreenPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.add_plugins(TextMenuPlugin::<GalleryChoice>::default())
			.add_systems(Update, apply_show_gallery);
	}
}

fn apply_show_gallery(
	mut commands: Commands,
	requests: Query<Entity, With<RequestShowGallery>>,
	existing: Query<Entity, With<MenuScreen>>,
	save_root: Option<Res<SaveRoot>>,
) {
	if !take_menu_show_request(&mut commands, &requests, &existing) {
		return;
	}
	let summaries = save_root.as_ref().map(|root| list_summaries(root)).unwrap_or_default();
	commands.spawn_scene(gallery_scene(&summaries));
}

fn gallery_scene(summaries: &[CharacterSummary]) -> impl Scene + 'static {
	let mut rows = vec![TextCursorRow::new("New Character", GalleryChoice::New)];
	rows.extend(summaries.iter().map(|summary| {
		TextCursorRow::new(summary.name.clone(), GalleryChoice::Open(summary.id))
			.with_subtext(summary.species_title)
	}));
	let children: Vec<Box<dyn Scene>> = vec![
		Box::new(TextCursorColumn::rows("Characters", rows).scene()),
		Box::new(screen_back_scene()),
	];
	bsn! {
		GalleryScreen
		MenuScreen
		MenuController
		BackgroundColor(MENU_CLEAR)
		Node {
			width: percent(100),
			height: percent(100),
		}
		Pickable::IGNORE
		on(republish_menu_activate::<GalleryChoice>)
		Children [ {children} ]
	}
}

#[cfg(test)]
mod tests {
	use super::GalleryChoice;
	use crozon_character_persist::CharacterId;

	#[test]
	fn new_is_the_default_row() {
		assert_eq!(GalleryChoice::default(), GalleryChoice::New);
		assert_ne!(GalleryChoice::Open(CharacterId(1)), GalleryChoice::New);
	}
}
