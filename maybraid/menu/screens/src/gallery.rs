//! Character gallery: pick a saved character or start a new one.

use bevy::prelude::*;
use bevy::scene::prelude::{Scene, bsn};
use character_model_user::{CharacterSummary, list_summaries};
use character_persist::{CharacterId, SaveRoot, load_active};
use maybraid_menu_controller::MenuController;
use menu_components::single_select::republish_menu_activate;
use menu_components::{
	GalleryPromptKind, InputPromptDevice, MenuBackRequested, MenuNav, MenuNavImpulse,
	MenuObjectiveKind, RotaryMenu, ScreenBackPressed, ScreenEditPressed, TextCursorRow, TextMenu,
	TextMenuItem, TextMenuPlugin, gallery_action_prompts_scene,
};

use crate::MenuScreen;
use crate::input::add_menu_input;
use crate::show::{MenuScreens, MenuShowRequest, add_menu_show_systems};

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

/// Index in the gallery column: 0 is Create a Character, then saved rows in order.
fn gallery_selected_index(summaries: &[CharacterSummary], active: Option<CharacterId>) -> usize {
	active
		.and_then(|id| summaries.iter().position(|summary| summary.id == id))
		.map(|index| index + 1)
		.unwrap_or(0)
}

impl GalleryChoice {
	pub fn create_row(empty_roster: bool) -> TextCursorRow<Self> {
		let row = TextCursorRow::new("Create a Character", Self::New);
		if empty_roster { row.with_objective(MenuObjectiveKind::StartHere) } else { row }
	}
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
	let mut rows = vec![GalleryChoice::create_row(summaries.is_empty())];
	rows.extend(summaries.iter().map(|summary| {
		let row = TextCursorRow::new(summary.name.clone(), GalleryChoice::Select(summary.id))
			.with_subtext(summary.species_title);
		if Some(summary.id) == active {
			row.with_objective(MenuObjectiveKind::Selected)
		} else {
			row.with_hidden_objective(MenuObjectiveKind::Selected)
		}
	}));
	rows
}

fn gallery_scene(
	summaries: &[CharacterSummary],
	active: Option<CharacterId>,
	device: InputPromptDevice,
) -> impl Scene + 'static {
	let rows = gallery_rows(summaries, active);
	let selected = gallery_selected_index(summaries, active);
	let children: Vec<Box<dyn Scene>> = vec![
		Box::new(RotaryMenu::rows("Characters", rows).with_selected(selected).scene()),
		Box::new(gallery_action_prompts_scene(device)),
	];
	bsn! {
		GalleryScreen
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
	use super::{GalleryChoice, gallery_rows, gallery_selected_index};
	use character_model_user::CharacterSummary;
	use character_persist::CharacterId;
	use menu_components::{MenuObjectiveKind, RotaryLayout, RotaryMenu};

	#[test]
	fn new_is_the_default_row() {
		assert_eq!(GalleryChoice::default(), GalleryChoice::New);
		assert_ne!(GalleryChoice::Select(CharacterId(1)), GalleryChoice::New);
	}

	#[test]
	fn selected_row_follows_the_active_character() {
		let jeff = CharacterId(1);
		let unnamed = CharacterId(2);
		let summaries = [
			CharacterSummary { id: jeff, name: "Jeff".into(), species_title: "Braidman" },
			CharacterSummary { id: unnamed, name: "Unnamed".into(), species_title: "Braidman" },
		];
		assert_eq!(gallery_selected_index(&summaries, Some(jeff)), 1);
		assert_eq!(gallery_selected_index(&summaries, Some(unnamed)), 2);
		assert_eq!(gallery_selected_index(&summaries, None), 0);
		assert_eq!(gallery_selected_index(&summaries, Some(CharacterId(99))), 0);
	}

	#[test]
	fn empty_roster_marks_create_a_character() {
		let row = GalleryChoice::create_row(true);
		assert_eq!(row.label, "Create a Character");
		assert_eq!(row.objective, Some(MenuObjectiveKind::StartHere));
		assert!(!row.locked);
		assert!(row.subtext.is_none());
		assert!(gallery_rows(&[], None).len() == 1);
	}

	#[test]
	fn saved_roster_drops_the_start_here_badge() {
		let summaries = [CharacterSummary {
			id: CharacterId(1),
			name: "Jeff".into(),
			species_title: "Braidman",
		}];
		let rows = gallery_rows(&summaries, None);
		assert!(rows[0].objective.is_none());
		assert_eq!(rows[0].label, "Create a Character");
		assert_eq!(rows[1].subtext.as_deref(), Some("Braidman"));
		assert_eq!(rows[1].objective, Some(MenuObjectiveKind::Selected));
		assert!(!rows[1].objective_visible);
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
		assert_eq!(rows.len(), 2);
		assert!(!rows.iter().any(|row| row.action == GalleryChoice::Delete));
		assert_eq!(GalleryChoice::Select(CharacterId(1)).saved_id(), Some(CharacterId(1)));
		assert_eq!(GalleryChoice::New.saved_id(), None);
		assert_eq!(GalleryChoice::Delete.saved_id(), None);
	}

	#[test]
	fn roster_uses_the_gallery_rotary_layout() {
		let summaries = [CharacterSummary {
			id: CharacterId(1),
			name: "Jeff".into(),
			species_title: "Braidman",
		}];
		let menu = RotaryMenu::rows("Characters", gallery_rows(&summaries, Some(CharacterId(1))))
			.with_selected(1);
		assert_eq!(menu.layout, RotaryLayout::GALLERY);
		assert_eq!(menu.items.len(), 2);
		assert_eq!(menu.selected, 1);
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
		assert!(rows[0].objective.is_none());
		assert_eq!(rows[1].objective, Some(MenuObjectiveKind::Selected));
		assert!(rows[1].objective_visible);
		assert_eq!(rows[2].objective, Some(MenuObjectiveKind::Selected));
		assert!(!rows[2].objective_visible);
	}
}
