//! Character gallery: pick a saved character or start a new one.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use character_model_user::{list_summaries, CharacterSummary};
use character_persist::{load_active, CharacterId, SaveRoot};
use maybraid_menu_controller::MenuController;
use menu_components::info::description::TextMenuDescription;
use menu_components::single_select::republish_menu_activate;
use menu_components::theme::GALLERY_DESCRIPTION_BOTTOM;
use menu_components::{
	gallery_action_prompts_scene, GalleryHudMenu, GalleryPromptKind, InputPromptDevice,
	MenuBackRequested, MenuNav, MenuNavImpulse, MenuObjectiveKind, ScreenBackPressed,
	ScreenEditPressed, TextCursorColumn, TextCursorRow, TextMenu, TextMenuItem, TextMenuPlugin,
};

use crate::input::add_menu_input;
use crate::show::{add_menu_show_systems, MenuScreens, MenuShowRequest};
use crate::MenuScreen;

/// Queue a gallery spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Clone, Copy)]
#[require(MenuShowRequest = MenuShowRequest::next())]
pub struct RequestShowGallery;

/// Marker on the spawned gallery root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct GalleryScreen;

/// Gallery destinations. Each pickable row stamps this as a component.
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum GalleryChoice {
	/// Not a roster row. Footer Start / **N** emit this to open the editor.
	#[default]
	New,
	Select(CharacterId),
	/// Not a roster row. Footer Delete / pad View emit this to open confirmation.
	Delete,
}

impl GalleryChoice {
	pub fn saved_id(self) -> Option<CharacterId> {
		match self {
			Self::Select(id) => Some(id),
			Self::New | Self::Delete => None,
		}
	}
}

pub fn request_show_gallery(commands: &mut Commands) {
	commands.spawn(RequestShowGallery);
}

pub struct GalleryScreenPlugin;

impl Plugin for GalleryScreenPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.add_plugins(TextMenuPlugin::<GalleryChoice>::default());
		add_menu_show_systems(app, apply_show_gallery);
		app.add_observer(emit_gallery_prompt_on_click)
			.add_systems(Update, sync_gallery_character_prompts);
	}
}

fn apply_show_gallery(
	mut commands: Commands,
	requests: Query<(), With<RequestShowGallery>>,
	screens: MenuScreens,
	save_root: Option<Res<SaveRoot>>,
	device: Option<Res<InputPromptDevice>>,
) {
	if screens.take(&mut commands, &requests).is_none() {
		return;
	}
	let summaries = save_root.as_ref().map(|root| list_summaries(root)).unwrap_or_default();
	let active = save_root.as_ref().and_then(|root| load_active(root));
	let device = device.as_deref().copied().unwrap_or_default();
	commands.spawn_scene(gallery_scene(&summaries, active, device));
}

/// Index of the active saved character, or 0 when the roster is empty / unmatched.
fn gallery_selected_index(summaries: &[CharacterSummary], active: Option<CharacterId>) -> usize {
	active
		.and_then(|id| summaries.iter().position(|summary| summary.id == id))
		.unwrap_or(0)
}

impl GalleryScreen {
	/// Roster choice under the text-menu cursor, if the gallery is live.
	pub fn highlighted_choice(
		screens: &Query<Entity, With<Self>>,
		menus: &Query<(Entity, &TextMenu)>,
		items: &Query<(Entity, &TextMenuItem, &GalleryChoice, &ChildOf)>,
		child_of: &Query<&ChildOf>,
	) -> Option<GalleryChoice> {
		let screen = screens.single().ok()?;
		menus.iter().find_map(|(entity, menu)| {
			Self::owns(screen, entity, child_of)
				.then(|| menu.selected_action(entity, items, child_of))
				.flatten()
		})
	}

	fn owns(screen: Entity, entity: Entity, child_of: &Query<&ChildOf>) -> bool {
		std::iter::once(entity)
			.chain(child_of.iter_ancestors(entity))
			.any(|ancestor| ancestor == screen)
	}
}

fn gallery_rows(
	summaries: &[CharacterSummary],
	active: Option<CharacterId>,
) -> Vec<TextCursorRow<GalleryChoice>> {
	summaries
		.iter()
		.map(|summary| {
			let row = TextCursorRow::new(summary.name.clone(), GalleryChoice::Select(summary.id))
				.with_subtext(summary.species_title);
			if Some(summary.id) == active {
				row.with_objective(MenuObjectiveKind::Selected)
			} else {
				row.with_hidden_objective(MenuObjectiveKind::Selected)
			}
		})
		.collect()
}

fn gallery_scene(
	summaries: &[CharacterSummary],
	active: Option<CharacterId>,
	device: InputPromptDevice,
) -> impl Scene + 'static {
	let rows = gallery_rows(summaries, active);
	let selected = gallery_selected_index(summaries, active);
	let children: Vec<Box<dyn Scene>> = vec![
		Box::new(
			TextCursorColumn::rows("Characters", rows)
				.with_selected(selected)
				.scrollable()
				.scene(),
		),
		Box::new(TextMenuDescription::scene_from_bottom(String::new(), GALLERY_DESCRIPTION_BOTTOM)),
		Box::new(gallery_action_prompts_scene(device)),
	];
	bsn! {
		GalleryScreen
		GalleryHudMenu
		MenuScreen
		MenuController::default()
		BackgroundColor(Color::NONE)
		Node {
			width: percent(100),
			height: percent(100),
		}
		Pickable::IGNORE
		on(republish_menu_activate::<GalleryChoice>)
		Children [ {children} ]
	}
}

fn emit_gallery_prompt_on_click(
	mut click: On<PointerClick>,
	kinds: Query<&GalleryPromptKind>,
	parents: Query<&ChildOf>,
	screens: Query<Entity, With<GalleryScreen>>,
	menus: Query<Entity, With<TextMenu>>,
	mut backs: MessageWriter<ScreenBackPressed>,
	mut edits: MessageWriter<ScreenEditPressed>,
	mut choices: MessageWriter<GalleryChoice>,
	mut commands: Commands,
) {
	if screens.is_empty() {
		return;
	}
	let mut entity = click.entity;
	let kind = loop {
		if let Ok(kind) = kinds.get(entity) {
			break *kind;
		}
		let Ok(parent) = parents.get(entity) else {
			return;
		};
		entity = parent.parent();
	};
	click.propagate(false);
	match kind {
		GalleryPromptKind::Select => {
			let Ok(screen) = screens.single() else {
				return;
			};
			let Some(menu) = menus.iter().find(|menu| GalleryScreen::owns(screen, *menu, &parents))
			else {
				return;
			};
			commands.trigger(MenuNavImpulse::new(menu, MenuNav::Select));
		}
		GalleryPromptKind::Back => {
			backs.write(ScreenBackPressed);
			commands.trigger(MenuBackRequested::new(click.entity, None));
		}
		GalleryPromptKind::Edit => {
			edits.write(ScreenEditPressed);
		}
		GalleryPromptKind::Delete => {
			choices.write(GalleryChoice::Delete);
		}
		GalleryPromptKind::Create => {
			choices.write(GalleryChoice::New);
		}
	}
}

fn sync_gallery_character_prompts(
	screens: Query<Entity, With<GalleryScreen>>,
	menus: Query<(Entity, &TextMenu)>,
	items: Query<(Entity, &TextMenuItem, &GalleryChoice, &ChildOf)>,
	child_of: Query<&ChildOf>,
	mut prompts: Query<(&GalleryPromptKind, &mut Visibility)>,
) {
	if screens.is_empty() {
		return;
	}
	let show = GalleryScreen::highlighted_choice(&screens, &menus, &items, &child_of)
		.and_then(GalleryChoice::saved_id)
		.is_some();
	for (kind, mut visibility) in &mut prompts {
		if !kind.needs_character() {
			continue;
		}
		*visibility = if show { Visibility::Inherited } else { Visibility::Hidden };
	}
}

#[cfg(test)]
mod tests {
	use super::{gallery_rows, gallery_selected_index, GalleryChoice};
	use character_model_user::CharacterSummary;
	use character_persist::CharacterId;
	use menu_components::MenuObjectiveKind;

	#[test]
	fn new_is_not_a_roster_row() {
		assert_eq!(GalleryChoice::default(), GalleryChoice::New);
		assert_ne!(GalleryChoice::Select(CharacterId(1)), GalleryChoice::New);
		assert!(gallery_rows(&[], None).is_empty());
		assert!(!gallery_rows(
			&[CharacterSummary {
				id: CharacterId(1),
				name: "Jeff".into(),
				species_title: "Braidman",
			}],
			None
		)
		.iter()
		.any(|row| row.action == GalleryChoice::New || row.label == "Create a Character"));
	}

	#[test]
	fn selected_row_follows_the_active_character() {
		let jeff = CharacterId(1);
		let unnamed = CharacterId(2);
		let summaries = [
			CharacterSummary { id: jeff, name: "Jeff".into(), species_title: "Braidman" },
			CharacterSummary { id: unnamed, name: "Unnamed".into(), species_title: "Braidman" },
		];
		assert_eq!(gallery_selected_index(&summaries, Some(jeff)), 0);
		assert_eq!(gallery_selected_index(&summaries, Some(unnamed)), 1);
		assert_eq!(gallery_selected_index(&summaries, None), 0);
		assert_eq!(gallery_selected_index(&summaries, Some(CharacterId(99))), 0);
		assert_eq!(gallery_selected_index(&[], None), 0);
	}

	#[test]
	fn delete_is_not_a_roster_row() {
		assert!(!gallery_rows(&[], None).iter().any(|row| row.action == GalleryChoice::Delete));
		let summaries = [CharacterSummary {
			id: CharacterId(1),
			name: "Jeff".into(),
			species_title: "Braidman",
		}];
		let rows = gallery_rows(&summaries, Some(CharacterId(1)));
		assert_eq!(rows.len(), 1);
		assert!(!rows.iter().any(|row| row.action == GalleryChoice::Delete));
		assert_eq!(GalleryChoice::Select(CharacterId(1)).saved_id(), Some(CharacterId(1)));
		assert_eq!(GalleryChoice::New.saved_id(), None);
		assert_eq!(GalleryChoice::Delete.saved_id(), None);
	}

	#[test]
	fn selected_chip_is_visible_only_on_the_active_row() {
		let jeff = CharacterId(1);
		let unnamed = CharacterId(2);
		let summaries = [
			CharacterSummary { id: jeff, name: "Jeff".into(), species_title: "Braidman" },
			CharacterSummary { id: unnamed, name: "Unnamed".into(), species_title: "Braidman" },
		];
		let rows = gallery_rows(&summaries, Some(jeff));
		assert_eq!(rows[0].objective, Some(MenuObjectiveKind::Selected));
		assert!(rows[0].objective_visible);
		assert_eq!(rows[1].objective, Some(MenuObjectiveKind::Selected));
		assert!(!rows[1].objective_visible);
	}
}
