//! Live character session: Users plus save/load for the gallery editor.

use bevy::prelude::*;
use crozon_character_items::Inventory;
use crozon_character_model_user::{
	spawn_model, CharacterModel, CharacterModelUser, CharacterModelUserPlugin,
};
use crozon_character_persist::{CharacterId, PersistError, SaveRoot};
use crozon_character_ui_menus::{CharacterMenu, MenuEvent};
use crozon_inventory_user::{spawn_bag, InventoryUser, InventoryUserPlugin};
use menu_components::info::description::{set_description_for_menu, TextMenuDescription};
use menu_components::{MenuActivate, MenuObjectiveKind, MenuObjectiveMarker, ScreenEditPressed};
use menu_screens::{
	request_show_create_character_id, request_show_gallery, CreateCharacterReady, GalleryChoice,
	GalleryScreen, GalleryScreenPlugin,
};

use crate::character::{
	request_show_character, CharacterEditBaseline, CharacterMenuState, CharacterScreen,
};

/// Host entity for the character currently being created or edited.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CharacterSession;

/// Id of the character open in the editor.
#[derive(Resource, Clone, Copy, Debug)]
pub struct EditingCharacter {
	pub id: CharacterId,
}

/// Character shown on home and in the gallery pane.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveCharacter {
	pub id: CharacterId,
}

/// Where Back goes after the character editor closes.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CharacterEditorReturn {
	#[default]
	Gallery,
	InGame,
}

/// Open the saved-character editor. `inventory` overrides the disk bag so the
/// in-game pause menu can edit the live loadout.
#[derive(Message, Clone, Debug)]
pub struct RequestEditCharacter {
	pub id: CharacterId,
	pub return_to: CharacterEditorReturn,
	pub inventory: Option<Inventory>,
}

pub struct CharacterSessionPlugin;

impl Plugin for CharacterSessionPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<InventoryUserPlugin>() {
			app.add_plugins(InventoryUserPlugin);
		}
		if !app.is_plugin_added::<CharacterModelUserPlugin>() {
			app.add_plugins(CharacterModelUserPlugin);
		}
		if !app.is_plugin_added::<GalleryScreenPlugin>() {
			app.add_plugins(GalleryScreenPlugin);
		}
		app.insert_resource(SaveRoot::workspace())
			.add_message::<RequestEditCharacter>()
			.add_observer(on_save_character)
			.add_systems(Startup, load_active_character)
			.add_systems(
				Update,
				(
					open_gallery_choice,
					open_gallery_edit,
					open_requested_character_editor,
					open_create_character_hud,
					sync_gallery_active_caption,
					sync_gallery_selected_tag,
				),
			);
	}
}

type SessionQuery<'w, 's> = Query<
	'w,
	's,
	(Entity, &'static CharacterModelUser, &'static InventoryUser),
	With<CharacterSession>,
>;

fn despawn_sessions(commands: &mut Commands, sessions: &SessionQuery) {
	for (host, model_user, inventory_user) in sessions.iter() {
		commands.entity(model_user.model).despawn();
		commands.entity(inventory_user.bag).despawn();
		commands.entity(host).despawn();
	}
}

fn spawn_session(
	commands: &mut Commands,
	sessions: &SessionQuery,
	id: CharacterId,
	name: String,
	appearance: crozon_characters::CharacterAppearance,
	inventory: Inventory,
) {
	despawn_sessions(commands, sessions);
	let host = commands.spawn(CharacterSession).id();
	spawn_model(commands, host, CharacterModel::new(id, name, appearance));
	spawn_bag(commands, host, inventory);
}

/// Write appearance and inventory. Only the Save chrome calls this; Back
/// returns to the gallery without writing, so name and clothing edits can be
/// abandoned.
pub fn save_editing_character(
	root: &SaveRoot,
	id: CharacterId,
	menu: &CharacterMenu,
) -> Result<(), PersistError> {
	let inventory = menu.inventory.clone().unwrap_or_default();
	let model = CharacterModel::new(id, menu.saved_name(), menu.appearance());
	crozon_character_model_user::save(root, &model)?;
	crozon_inventory_user::save(root, id, &inventory)?;
	Ok(())
}

pub fn set_active_character(commands: &mut Commands, root: &SaveRoot, id: CharacterId) {
	commands.insert_resource(ActiveCharacter { id });
	if let Err(error) = crozon_character_persist::save_active(root, id) {
		warn!("failed to save active character {}: {error}", id.to_hex());
	}
}

fn load_active_character(mut commands: Commands, save_root: Res<SaveRoot>) {
	if let Some(id) = crozon_character_persist::load_active(&save_root) {
		if save_root.character_path(id).is_file() {
			commands.insert_resource(ActiveCharacter { id });
			return;
		}
	}
	let Ok(ids) = save_root.list_ids() else {
		return;
	};
	let Some(id) = ids.first().copied() else {
		return;
	};
	set_active_character(&mut commands, &save_root, id);
}

fn sync_gallery_active_caption(
	active: Option<Res<ActiveCharacter>>,
	save_root: Res<SaveRoot>,
	screens: Query<Entity, With<GalleryScreen>>,
	children: Query<&Children>,
	mut lines: Query<&mut Text, With<TextMenuDescription>>,
	mut last: Local<Option<(Entity, Option<CharacterId>)>>,
) {
	let Ok(root) = screens.single() else {
		*last = None;
		return;
	};
	let id = active.map(|active| active.id);
	if last.as_ref().is_some_and(|(entity, stored)| *entity == root && *stored == id) {
		return;
	}
	*last = Some((root, id));
	let caption = id
		.and_then(|id| {
			crozon_character_model_user::load(&save_root, id).ok().map(|model| model.name)
		})
		.unwrap_or_default();
	set_description_for_menu(root, caption, &children, &mut lines);
}

/// Keep the Selected chip on the active character after a new row is chosen.
fn sync_gallery_selected_tag(
	active: Option<Res<ActiveCharacter>>,
	screens: Query<Entity, With<GalleryScreen>>,
	items: Query<(Entity, &GalleryChoice, &Children)>,
	markers: Query<(), Or<(With<MenuObjectiveKind>, With<MenuObjectiveMarker>)>>,
	mut commands: Commands,
	mut visibilities: Query<&mut Visibility>,
) {
	if screens.is_empty() {
		return;
	}
	let active_id = active.map(|active| active.id);
	for (entity, choice, children) in &items {
		let GalleryChoice::Select(id) = *choice else {
			continue;
		};
		let is_active = gallery_row_is_active(Some(id), active_id);
		if is_active {
			commands.entity(entity).insert(MenuObjectiveKind::Selected);
		} else {
			commands.entity(entity).remove::<MenuObjectiveKind>();
		}
		for child in children {
			if markers.get(*child).is_err() {
				continue;
			}
			if let Ok(mut visibility) = visibilities.get_mut(*child) {
				*visibility = if is_active { Visibility::Inherited } else { Visibility::Hidden };
			}
			if is_active {
				commands.entity(*child).insert(MenuObjectiveKind::Selected);
			} else {
				commands.entity(*child).remove::<MenuObjectiveKind>();
			}
		}
	}
}

fn open_gallery_choice(
	mut choices: MessageReader<GalleryChoice>,
	save_root: Res<SaveRoot>,
	active: Option<Res<ActiveCharacter>>,
	mut commands: Commands,
	mut edits: MessageWriter<ScreenEditPressed>,
) {
	let Some(choice) = choices.read().last().copied() else {
		return;
	};
	match choice {
		GalleryChoice::New => {
			let id = CharacterId::new();
			commands.insert_resource(EditingCharacter { id });
			commands.remove_resource::<CharacterEditBaseline>();
			request_show_create_character_id(&mut commands, id);
		}
		GalleryChoice::Select(id) => {
			if gallery_select_opens_edit(active.as_deref().map(|active| active.id), id) {
				edits.write(ScreenEditPressed);
				return;
			}
			if let Err(error) = crozon_character_model_user::load(&save_root, id) {
				warn!("failed to load character {}: {error}", id.to_hex());
				return;
			}
			set_active_character(&mut commands, &save_root, id);
		}
	}
}

fn open_gallery_edit(
	mut edits: MessageReader<ScreenEditPressed>,
	gallery: Query<(), With<GalleryScreen>>,
	save_root: Res<SaveRoot>,
	active: Option<Res<ActiveCharacter>>,
	mut menu_state: ResMut<CharacterMenuState>,
	mut commands: Commands,
	sessions: SessionQuery,
) {
	if gallery.is_empty() || edits.read().next().is_none() {
		return;
	}
	let Some(active) = active else {
		return;
	};
	open_saved_editor(
		&mut commands,
		&sessions,
		&mut menu_state,
		&save_root,
		active.id,
		None,
		CharacterEditorReturn::Gallery,
	);
}

fn open_requested_character_editor(
	mut requests: MessageReader<RequestEditCharacter>,
	save_root: Res<SaveRoot>,
	mut menu_state: ResMut<CharacterMenuState>,
	mut commands: Commands,
	sessions: SessionQuery,
) {
	let Some(request) = requests.read().last().cloned() else {
		return;
	};
	open_saved_editor(
		&mut commands,
		&sessions,
		&mut menu_state,
		&save_root,
		request.id,
		request.inventory,
		request.return_to,
	);
}

fn open_saved_editor(
	commands: &mut Commands,
	sessions: &SessionQuery,
	menu_state: &mut CharacterMenuState,
	save_root: &SaveRoot,
	id: CharacterId,
	inventory: Option<Inventory>,
	return_to: CharacterEditorReturn,
) {
	let model = match crozon_character_model_user::load(save_root, id) {
		Ok(model) => model,
		Err(error) => {
			warn!("failed to load character {}: {error}", id.to_hex());
			return;
		}
	};
	let inventory = match inventory {
		Some(inventory) => inventory,
		None => match crozon_inventory_user::load(save_root, id) {
			Ok(inventory) => inventory,
			Err(error) => {
				warn!("failed to load inventory {}: {error}", id.to_hex());
				return;
			}
		},
	};
	commands.insert_resource(EditingCharacter { id });
	commands.insert_resource(return_to);
	spawn_session(
		commands,
		sessions,
		id,
		model.name.clone(),
		model.appearance.clone(),
		inventory.clone(),
	);
	menu_state.0 = CharacterMenu::for_saved(model.name, &model.appearance, inventory);
	commands.insert_resource(CharacterEditBaseline::capture(&menu_state.0));
	request_show_character(commands);
}

fn open_create_character_hud(
	mut ready: MessageReader<CreateCharacterReady>,
	mut menu_state: ResMut<CharacterMenuState>,
	mut commands: Commands,
	sessions: SessionQuery,
) {
	let Some(ready) = ready.read().last() else {
		return;
	};
	let inventory = Inventory::with_starter_outfit(ready.items.clone());
	commands.insert_resource(EditingCharacter { id: ready.id });
	commands.remove_resource::<CharacterEditBaseline>();
	spawn_session(
		&mut commands,
		&sessions,
		ready.id,
		String::from("Unnamed"),
		crozon_characters::CharacterAppearance::default(),
		inventory,
	);
	*menu_state = CharacterMenuState::for_create(ready.items.clone());
	request_show_character(&mut commands);
}

fn on_save_character(
	activate: On<MenuActivate<MenuEvent>>,
	screens: Query<Entity, With<CharacterScreen>>,
	menu_state: Res<CharacterMenuState>,
	save_root: Res<SaveRoot>,
	editing: Option<Res<EditingCharacter>>,
	baseline: Option<ResMut<CharacterEditBaseline>>,
	mut commands: Commands,
) {
	if screens.is_empty() || activate.event().choice != MenuEvent::Save {
		return;
	}
	let Some(editing) = editing else {
		warn!("save character: no editing id");
		return;
	};
	if let Err(error) = save_editing_character(&save_root, editing.id, &menu_state.0) {
		warn!("failed to save character {}: {error}", editing.id.to_hex());
		return;
	}
	set_active_character(&mut commands, &save_root, editing.id);
	if menu_state.0.is_create() {
		commands.remove_resource::<CharacterEditBaseline>();
		request_show_gallery(&mut commands);
		return;
	}
	if let Some(mut baseline) = baseline {
		*baseline = CharacterEditBaseline::capture(&menu_state.0);
	} else {
		commands.insert_resource(CharacterEditBaseline::capture(&menu_state.0));
	}
}

fn gallery_select_opens_edit(active: Option<CharacterId>, picked: CharacterId) -> bool {
	active == Some(picked)
}

fn gallery_row_is_active(row: Option<CharacterId>, active: Option<CharacterId>) -> bool {
	row.is_some() && row == active
}

#[cfg(test)]
mod tests {
	use super::{gallery_row_is_active, gallery_select_opens_edit};
	use crozon_character_persist::CharacterId;

	#[test]
	fn second_select_on_the_active_row_is_edit() {
		let id = CharacterId(7);
		assert!(!gallery_select_opens_edit(None, id));
		assert!(!gallery_select_opens_edit(Some(CharacterId(1)), id));
		assert!(gallery_select_opens_edit(Some(id), id));
	}

	#[test]
	fn selected_tag_stays_on_the_active_character() {
		let id = CharacterId(7);
		assert!(gallery_row_is_active(Some(id), Some(id)));
		assert!(!gallery_row_is_active(Some(id), Some(CharacterId(1))));
		assert!(!gallery_row_is_active(Some(id), None));
	}
}
