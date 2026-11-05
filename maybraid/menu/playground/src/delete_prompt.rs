//! Gallery deletion confirmation. The session owns persistence and selection.

use bevy::prelude::*;
use bevy::text::Justify;
use character_persist::{CharacterId, SaveRoot};
use menu_components::theme::{
	menu_list_panel_node, menu_region_wash, HEADER_FONT_SIZE, PANEL_ROW_GAP, TEXT_YELLOW,
};
use menu_components::{
	spawn_hud_action, spawn_hud_text, HudFonts, HudMenu, HudMenuItem, HudOverlayMenu, MenuModal,
	MenuModalDismiss, MenuModalId, MenuModalLayer, MenuModalStack, MenuModalSystems, MenuNav,
	MenuNavImpulse, TextMenuSystems,
};
use menu_screens::{request_show_gallery, GalleryChoice, GalleryScreen};

use crate::session::{delete_saved_character, ActiveCharacter};

#[derive(Resource, Default)]
struct DeletePrompt {
	target: Option<(CharacterId, String)>,
	error: Option<String>,
	opened_this_frame: bool,
}

#[derive(Component)]
struct DeletePromptRoot;
#[derive(Component)]
struct DeletePromptMenu;
#[derive(Component, Clone, Copy)]
struct DeleteAction(bool);

#[derive(EntityEvent)]
struct DeletePromptActivate {
	entity: Entity,
	confirm: bool,
}

pub(crate) struct CharacterDeletePlugin;

impl Plugin for CharacterDeletePlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<DeletePrompt>()
			.add_observer(on_click)
			.add_observer(on_activate)
			.add_observer(on_nav)
			.add_systems(First, |mut prompt: ResMut<DeletePrompt>| {
				if prompt.opened_this_frame {
					prompt.opened_this_frame = false;
				}
			})
			.add_systems(Update, dismiss.in_set(MenuModalSystems::Apply))
			.add_systems(Update, (open, paint).chain().after(TextMenuSystems::Navigate));
	}
}

fn open(
	mut choices: MessageReader<GalleryChoice>,
	screens: Query<(), With<GalleryScreen>>,
	root: Res<SaveRoot>,
	active: Option<Res<ActiveCharacter>>,
	mut prompt: ResMut<DeletePrompt>,
	mut stack: ResMut<MenuModalStack>,
) -> Result {
	if !choices.read().any(|choice| *choice == GalleryChoice::Delete)
		|| screens.is_empty()
		|| prompt.target.is_some()
		|| stack.is_open()
	{
		return Ok(());
	}
	let Some(active) = active else {
		return Ok(());
	};
	let model = character_model_user::load(&root, active.id)?;
	prompt.target = Some((active.id, model.name));
	prompt.error = None;
	prompt.opened_this_frame = true;
	stack.open(MenuModalLayer::new(MenuModalId::CHARACTER_DELETE, 30));
	stack.mark_consumed();
	Ok(())
}

fn dismiss(
	mut events: MessageReader<MenuModalDismiss>,
	mut prompt: ResMut<DeletePrompt>,
	mut stack: ResMut<MenuModalStack>,
) {
	if events.read().any(|event| event.is(MenuModalId::CHARACTER_DELETE)) {
		prompt.close(&mut stack);
	}
}

impl DeletePrompt {
	fn close(&mut self, stack: &mut MenuModalStack) {
		self.target = None;
		self.error = None;
		stack.close(MenuModalId::CHARACTER_DELETE);
		stack.mark_consumed();
	}

	fn apply(
		&mut self,
		confirm: bool,
		root: &SaveRoot,
		active: Option<&ActiveCharacter>,
		stack: &mut MenuModalStack,
		commands: &mut Commands,
	) {
		if self.opened_this_frame {
			return;
		}
		let Some((id, _)) = self.target.as_ref() else {
			return;
		};
		if !confirm {
			self.close(stack);
			return;
		}
		match delete_saved_character(root, *id, active.map(|active| active.id)) {
			Ok(next) => {
				if let Some(id) = next {
					commands.insert_resource(ActiveCharacter { id });
				} else {
					commands.remove_resource::<ActiveCharacter>();
				}
				self.close(stack);
				request_show_gallery(commands);
			}
			Err(error) => {
				warn!("failed to delete character {}: {error}", id.to_hex());
				self.error = Some(format!("Couldn't delete character: {error}"));
			}
		}
	}
}

fn on_nav(
	impulse: On<MenuNavImpulse>,
	menus: Query<&HudMenu, With<DeletePromptMenu>>,
	mut commands: Commands,
) {
	let Ok(menu) = menus.get(impulse.entity) else {
		return;
	};
	let confirm = match impulse.event().nav {
		MenuNav::Select => menu.selected == 1,
		MenuNav::Back => false,
		_ => return,
	};
	impulse.deliver(&mut commands, DeletePromptActivate { entity: impulse.entity, confirm });
}

fn on_click(
	mut click: On<PointerClick>,
	actions: Query<&DeleteAction>,
	parents: Query<&ChildOf>,
	mut commands: Commands,
) {
	let mut entity = click.entity;
	loop {
		if let Ok(action) = actions.get(entity) {
			click.propagate(false);
			commands.trigger(DeletePromptActivate { entity, confirm: action.0 });
			return;
		}
		let Ok(parent) = parents.get(entity) else {
			return;
		};
		entity = parent.parent();
	}
}

fn on_activate(
	action: On<DeletePromptActivate>,
	root: Res<SaveRoot>,
	active: Option<Res<ActiveCharacter>>,
	mut prompt: ResMut<DeletePrompt>,
	mut stack: ResMut<MenuModalStack>,
	mut commands: Commands,
) {
	prompt.apply(action.confirm, &root, active.as_deref(), &mut stack, &mut commands);
}

fn paint(
	mut prompt: ResMut<DeletePrompt>,
	mut stack: ResMut<MenuModalStack>,
	screens: Query<Entity, With<GalleryScreen>>,
	roots: Query<Entity, With<DeletePromptRoot>>,
	assets: Res<AssetServer>,
	mut painted: Local<Option<(CharacterId, String, Option<String>)>>,
	mut commands: Commands,
) {
	if screens.is_empty() && prompt.target.is_some() {
		prompt.close(&mut stack);
	}
	let desired = prompt
		.target
		.as_ref()
		.map(|(id, name)| (*id, name.clone(), prompt.error.clone()));
	if *painted == desired && (desired.is_none() || !roots.is_empty()) {
		return;
	}
	*painted = desired;
	for entity in &roots {
		commands.entity(entity).try_despawn();
	}
	let Some((_, name)) = prompt.target.as_ref() else {
		return;
	};
	let Ok(screen) = screens.single() else {
		return;
	};
	let fonts = HudFonts::load(&assets);
	commands.entity(screen).with_children(|parent| {
		parent
			.spawn((
				DeletePromptRoot,
				MenuModal { id: MenuModalId::CHARACTER_DELETE, order: 30 },
				Node {
					position_type: PositionType::Absolute,
					width: percent(100),
					height: percent(100),
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
					..default()
				},
				GlobalZIndex(100),
				BackgroundColor(menu_region_wash()),
				Pickable::default(),
			))
			.with_children(|root| {
				let mut card = menu_list_panel_node(AlignItems::FlexStart);
				card.width = px(720);
				card.max_width = percent(92);
				root.spawn((card, BackgroundColor(menu_region_wash()))).with_children(|card| {
					spawn_hud_text(
						card,
						fonts.header(HEADER_FONT_SIZE * 0.42),
						&format!("Delete \"{name}\"?"),
						TEXT_YELLOW,
						Justify::Left,
					);
					spawn_hud_text(
						card,
						fonts.header(HEADER_FONT_SIZE * 0.22),
						prompt.error.as_deref().unwrap_or(
							"This permanently deletes their appearance, inventory, and saved position.",
						),
						TEXT_YELLOW,
						Justify::Left,
					);
					let mut list = card.spawn((
						DeletePromptMenu,
						HudOverlayMenu,
						HudMenu::new(2),
						Node {
							flex_direction: FlexDirection::Column,
							row_gap: px(PANEL_ROW_GAP),
							..default()
						},
						Pickable::IGNORE,
					));
					let menu = list.id();
					list.with_children(|list| {
						for (index, label) in ["Cancel", "Delete Character"].into_iter().enumerate()
						{
							spawn_hud_action(
								list,
								&fonts,
								label,
								JustifyContent::FlexStart,
								(DeleteAction(index == 1), HudMenuItem { index, menu }),
							);
						}
					});
				});
			});
	});
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::session::save_editing_character;
	use crate::CharacterMenuState;
	use bevy::ecs::system::RunSystemOnce;

	fn world() -> Result<World, Box<dyn std::error::Error>> {
		let root = SaveRoot::at(
			std::env::temp_dir().join(format!("maybraid-delete-{}", CharacterId::new().to_hex())),
		);
		let id = CharacterId(7);
		save_editing_character(&root, id, &CharacterMenuState::default().0)?;
		character_persist::save_active(&root, id)?;
		let mut world = World::new();
		world.insert_resource(root);
		world.insert_resource(ActiveCharacter { id });
		world.insert_resource(DeletePrompt {
			target: Some((id, "Roamer".into())),
			error: None,
			opened_this_frame: false,
		});
		let mut stack = MenuModalStack::default();
		stack.open(MenuModalLayer::new(MenuModalId::CHARACTER_DELETE, 30));
		world.insert_resource(stack);
		world.add_observer(on_nav);
		world.add_observer(on_activate);
		Ok(world)
	}

	#[test]
	fn default_confirmation_and_back_cancel_without_deleting(
	) -> Result<(), Box<dyn std::error::Error>> {
		for nav in [MenuNav::Select, MenuNav::Back] {
			let mut world = world()?;
			let menu = world.spawn((DeletePromptMenu, HudMenu::new(2))).id();
			world.trigger(MenuNavImpulse::new(menu, nav));
			world.flush();
			assert!(world.resource::<DeletePrompt>().target.is_none());
			assert!(!world.resource::<MenuModalStack>().is_open());
			assert!(world.resource::<MenuModalStack>().consumed());
			let root = world.resource::<SaveRoot>();
			assert!(root.character_path(CharacterId(7)).is_file());
			assert_eq!(character_persist::load_active(root), Some(CharacterId(7)));
			std::fs::remove_dir_all(&root.path)?;
		}
		Ok(())
	}

	#[test]
	fn opening_edge_cannot_confirm_and_next_confirm_deletes_last_character(
	) -> Result<(), Box<dyn std::error::Error>> {
		let mut world = world()?;
		let menu = world.spawn((DeletePromptMenu, HudMenu { selected: 1, item_count: 2 })).id();
		world.resource_mut::<DeletePrompt>().opened_this_frame = true;
		world.trigger(MenuNavImpulse::new(menu, MenuNav::Select));
		assert!(world.resource::<DeletePrompt>().target.is_some());
		world.resource_mut::<DeletePrompt>().opened_this_frame = false;
		world.trigger(MenuNavImpulse::new(menu, MenuNav::Select));
		world.flush();
		assert!(!world.contains_resource::<ActiveCharacter>());
		assert!(world.resource::<DeletePrompt>().target.is_none());
		assert_eq!(world.query::<&menu_screens::RequestShowGallery>().iter(&world).count(), 1);
		let root = world.resource::<SaveRoot>();
		assert!(root.list_ids()?.is_empty());
		assert!(!root.active_path().exists());
		std::fs::remove_dir_all(&root.path)?;
		Ok(())
	}

	#[test]
	fn storage_failure_leaves_confirmation_roster_and_selection_available(
	) -> Result<(), Box<dyn std::error::Error>> {
		let mut world = world()?;
		let root = world.resource::<SaveRoot>().clone();
		std::fs::create_dir(root.position_path(CharacterId(7)))?;
		world
			.run_system_once(
				|mut prompt: ResMut<DeletePrompt>,
				 root: Res<SaveRoot>,
				 active: Res<ActiveCharacter>,
				 mut stack: ResMut<MenuModalStack>,
				 mut commands: Commands| {
					prompt.apply(true, &root, Some(&active), &mut stack, &mut commands);
				},
			)
			.map_err(|error| format!("{error:?}"))?;
		assert!(world.resource::<DeletePrompt>().target.is_some());
		assert!(world.resource::<DeletePrompt>().error.is_some());
		assert!(world.resource::<MenuModalStack>().is_open());
		assert_eq!(world.resource::<ActiveCharacter>().id, CharacterId(7));
		assert!(root.character_path(CharacterId(7)).is_file());
		assert_eq!(world.query::<&menu_screens::RequestShowGallery>().iter(&world).count(), 0);
		std::fs::remove_dir_all(&root.path)?;
		Ok(())
	}
}
