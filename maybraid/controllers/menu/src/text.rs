//! Gamepad text editing and UI owned by one scoped menu session.
use crate::MenuSession;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use maybraid_controllers_core::Routed;
use maybraid_virtual_pad_controller::{PadButton, VirtualPadEvent};
use menu_components::controls::short_text::{
	ShortTextModalValue, ShortTextPadKey, ShortTextPadLetter, ShortTextSession, ShortTextSubmit,
	apply_short_text_pad_key, spawn_short_text_modal,
};
use menu_components::{
	HudFonts, HudMenu, HudMenuItem, MenuModalId, MenuModalLayer, MenuNav, ScopedMenuSession,
	ShortTextChange, ShortTextField, ShortTextKey, ShortTextToggle,
};

#[derive(SystemParam)]
pub struct ScopedTextInput<'w, 's> {
	fields: Query<'w, 's, &'static mut ShortTextField>,
	menus: Query<'w, 's, &'static HudMenu>,
	items: Query<
		'w,
		's,
		(
			Entity,
			&'static HudMenuItem,
			Option<&'static ShortTextKey>,
			Option<&'static ShortTextPadKey>,
			Option<&'static ShortTextSubmit>,
		),
	>,
}
impl ScopedTextInput<'_, '_> {
	/// Returns true when this frame belongs to text editing rather than ordinary menu activation.
	pub fn apply(
		&mut self,
		session: &mut MenuSession,
		target: Entity,
		input: &Routed<VirtualPadEvent>,
		events: &[MenuNav],
		commands: &mut Commands,
	) -> bool {
		let pad = &input.payload().pad;
		if session.text.is_open() {
			if events.contains(&MenuNav::Back) {
				self.finish(session, input, false, commands);
				return true;
			}
			if pad.just_pressed(PadButton::Start) {
				self.finish(session, input, true, commands);
				return true;
			}
			if pad.just_pressed(PadButton::StickClickMove) {
				apply_short_text_pad_key(ShortTextPadKey::Shift, &mut session.text);
				return true;
			}
			if !events.contains(&MenuNav::Select) {
				return false;
			}
			let Ok(menu) = self.menus.get(target) else {
				return true;
			};
			if let Some((_, _, _, key, submit)) = self
				.items
				.iter()
				.find(|(_, item, ..)| item.menu == target && item.index == menu.selected)
			{
				if submit.is_some() {
					self.finish(session, input, true, commands);
				} else if let Some(key) = key {
					apply_short_text_pad_key(*key, &mut session.text);
				}
			}
			return true;
		}
		if !events.contains(&MenuNav::Select) || session.text.dismissed {
			return false;
		}
		let Ok(menu) = self.menus.get(target) else {
			return false;
		};
		let Some((entity, _, Some(key), ..)) = self.items.iter().find(|(_, item, key, ..)| {
			item.menu == target && item.index == menu.selected && key.is_some()
		}) else {
			return false;
		};
		let Ok(mut field) = self.fields.get_mut(entity) else {
			return false;
		};
		field.editing = true;
		session.text.session = Some(ShortTextSession {
			key: key.0,
			source: entity,
			value: field.value.clone(),
			original: field.value.clone(),
			max_len: field.max_len,
			shift: false,
		});
		session.modals.open(MenuModalLayer::SHORT_TEXT);
		let key = key.0;
		input.clone().queue(commands, move |world, _| {
			world.trigger(ShortTextToggle { entity, key, editing: true })
		});
		true
	}
	fn finish(
		&mut self,
		session: &mut MenuSession,
		input: &Routed<VirtualPadEvent>,
		submit: bool,
		commands: &mut Commands,
	) {
		let Some(text) = session.text.session.take() else {
			return;
		};
		if let Ok(mut field) = self.fields.get_mut(text.source) {
			field.editing = false;
			if submit {
				field.value.clone_from(&text.value);
			}
		}
		session.text.dismissed = true;
		session.modals.close(MenuModalId::SHORT_TEXT);
		session.modals.mark_consumed();
		if let Some(root) = session.text_root.take() {
			commands.entity(root).try_despawn();
		}
		input.clone().queue(commands, move |world, _| {
			if submit {
				world.trigger(ShortTextChange {
					entity: text.source,
					key: text.key,
					value: text.value,
				});
			}
			world.trigger(ShortTextToggle { entity: text.source, key: text.key, editing: false });
		});
	}
}

/// Keep every node distinguishable from the explicit primary keyboard/IME host.
pub fn mark_scoped_subtrees(
	roots: Query<Entity, With<MenuSession>>,
	children: Query<&Children>,
	unmarked: Query<(), Without<ScopedMenuSession>>,
	mut commands: Commands,
) {
	for root in &roots {
		for entity in std::iter::once(root).chain(children.iter_descendants(root)) {
			if unmarked.contains(entity) {
				commands.entity(entity).try_insert(ScopedMenuSession);
			}
		}
	}
}

pub fn sync_scoped_text_ui(
	mut commands: Commands,
	assets: Option<Res<AssetServer>>,
	mut sessions: Query<(Entity, &mut MenuSession)>,
	children: Query<&Children>,
	mut values: Query<&mut Text, With<ShortTextModalValue>>,
	mut letters: Query<(&ShortTextPadLetter, &mut Text), Without<ShortTextModalValue>>,
) {
	for (owner, mut session) in &mut sessions {
		let Some(text) = session.text.session.as_ref() else {
			if let Some(root) = session.text_root.take() {
				commands.entity(root).try_despawn();
			}
			continue;
		};
		let Some(root) = session.text_root else {
			let Some(assets) = assets.as_ref() else {
				continue;
			};
			let root = spawn_short_text_modal(&mut commands, &HudFonts::load(assets), text, true);
			commands.entity(root).insert((ScopedMenuSession, ChildOf(owner)));
			session.text_root = Some(root);
			continue;
		};
		for entity in children.iter_descendants(root) {
			if let Ok(mut value) = values.get_mut(entity) {
				value.0 = if text.value.is_empty() { " ".into() } else { text.value.clone() };
			}
			if let Ok((letter, mut value)) = letters.get_mut(entity) {
				value.0 = if text.shift {
					letter.0.to_ascii_uppercase()
				} else {
					letter.0.to_ascii_lowercase()
				}
				.to_string();
			}
		}
	}
}
