//! Scoped menu focus and rich routed-pad interpretation.
use crate::MenuController;
pub use crate::primary::{
	ExclusiveNav, MenuNavCapture, exclusive_nav_menu,
	refresh_primary_menu_focus as refresh_menu_focus,
};
use bevy::prelude::*;
use maybraid_controllers_core::{CaptureKind, InputRouting, ScopeOwner, UiTarget};
use menu_components::{HudMenu, HudOverlayMenu, MenuModal, MenuNav, MenuNavImpulse, TextMenu};

pub fn refresh_scoped_menu_focus(
	children: Query<&Children>,
	overlays: Query<Entity, With<HudOverlayMenu>>,
	hud: Query<Entity, (With<HudMenu>, Without<HudOverlayMenu>)>,
	all_hud: Query<Entity, With<HudMenu>>,
	text: Query<Entity, With<TextMenu>>,
	modal_roots: Query<(Entity, &MenuModal)>,
	mut controllers: Query<(Entity, &mut MenuController, Option<&crate::MenuSession>)>,
	mut routing: InputRouting,
) {
	for (root, mut controller, session) in &mut controllers {
		let Some(source) = controller.source else {
			continue;
		};
		let blocked = session.is_some_and(|s| s.text_locked || s.modals.consumed());
		let focus = if blocked {
			None
		} else if let Some(id) = session.and_then(|s| s.modals.nav_owner()) {
			modal_roots
				.iter()
				.filter(|(entity, modal)| {
					modal.id == id && UiTarget::new(*entity).is_under(root, &children)
				})
				.find_map(|(entity, _)| {
					std::iter::once(entity)
						.chain(children.iter_descendants(entity))
						.find(|e| all_hud.contains(*e))
				})
		} else {
			MenuController::resolve(root, &children, &overlays, &hud, &text)
		};
		controller.blocked = blocked;
		controller.context_blocked = session.is_some_and(|s| s.modals.owns_nav());
		controller.set_focus(focus);
		routing.replace_owned_capture(
			&mut controller.capture,
			source,
			focus
				.or_else(|| {
					session
						.filter(|s| s.text_locked || s.modals.owns_nav() || s.modals.consumed())
						.map(|_| root)
				})
				.map(|e| ScopeOwner::ui(UiTarget::new(e))),
			if blocked {
				CaptureKind::TextEntry
			} else if focus.is_none() {
				CaptureKind::Overlay
			} else {
				CaptureKind::MenuNav
			},
		);
	}
}

/// Interpret only the rich pad frame authorized for the addressed menu.
/// No hardware, router, text-lock, or global modal polling occurs here.
pub fn interpret_menu_pad(
	event: On<maybraid_controllers_core::UiPadEvent>,
	parents: Query<&ChildOf>,
	mut controllers: Query<&mut MenuController>,
	mut sessions: Query<&mut crate::MenuSession>,
	mut text: crate::text::ScopedTextInput,
	reorder: Query<(), With<menu_components::HudMenuNavLocked>>,
	config: Res<maybraid_virtual_pad_controller::VirtualPadConfig>,
	sources: Query<&maybraid_virtual_pad_controller::VirtualPadController>,
	mut commands: Commands,
) {
	let input = &event.input;
	let target = event.entity;
	let mut ancestor = Some(target);
	while let Some(entity) = ancestor {
		if let Ok(mut controller) = controllers.get_mut(entity) {
			if controller.blocked || controller.focused_entity() != Some(target) {
				return;
			}
			if sessions
				.get(entity)
				.is_ok_and(|session| session.text.dismissed || session.modals.consumed())
			{
				return;
			}
			let source = input.source();
			let frame = input.payload();
			let config = sources.get(frame.source).map(|source| &source.config).unwrap_or(&config);
			let binding = input.binding();
			let index = controller.streams.iter().position(|stream| stream.source == source);
			let index = index.unwrap_or_else(|| {
				controller.streams.push(crate::controller::MenuStream {
					source,
					binding,
					sequence: 0,
					await_release: false,
					repeat: Default::default(),
				});
				controller.streams.len() - 1
			});
			let start_select = controller.start == crate::MenuStartPolicy::Select;
			let context_blocked = controller.context_blocked;
			let stream = &mut controller.streams[index];
			if stream.binding == binding && frame.sequence <= stream.sequence {
				return;
			}
			if stream.binding != binding {
				stream.repeat = Default::default();
				stream.binding = binding;
				stream.await_release = true;
			}
			stream.sequence = frame.sequence;
			let mut interpreted_pad = frame.pad.clone();
			if stream.await_release {
				use maybraid_virtual_pad_controller::PadButton;
				let edge = [
					PadButton::DpadUp,
					PadButton::DpadDown,
					PadButton::DpadLeft,
					PadButton::DpadRight,
				]
				.into_iter()
				.any(|button| frame.pad.just_pressed(button));
				let direction =
					MenuNav::direction_from_pad(&frame.pad, config.menu_stick_threshold);
				if direction.is_none() || edge {
					stream.await_release = false;
				} else {
					interpreted_pad.move_stick = Vec2::ZERO;
					interpreted_pad.dpad = Vec2::ZERO;
				}
			}
			stream.repeat.derive(&interpreted_pad, config, frame.delta.as_secs_f32());
			let mut events = stream.repeat.events.clone();
			if !start_select
				&& !frame.pad.just_pressed(maybraid_virtual_pad_controller::PadButton::A)
			{
				events.retain(|nav| *nav != MenuNav::Select);
			}
			if let Ok(mut session) = sessions.get_mut(entity) {
				if text.apply(&mut session, target, input, &events, &mut commands) {
					input.consume_frame(&mut commands);
					return;
				}
			}
			// Back wins the frame; navigation wins over inventory context actions.
			if events.contains(&MenuNav::Back) {
				input.consume_frame(&mut commands);
				if !reorder.contains(target) {
					if let Ok(mut session) = sessions.get_mut(entity) {
						if let Some(id) = session.modals.take_back(true) {
							session.modals.close(id);
							controller.blocked = true;
							input.clone().map(move |_| id).trigger(&mut commands, move |routed| {
								menu_components::MenuModalDismissImpulse {
									entity: target,
									id: routed.into_payload(),
								}
							});
							return;
						}
					}
				}
				if !reorder.contains(target) {
					let source = frame.source;
					input
						.clone()
						.map(move |_| Some(source))
						.trigger(&mut commands, move |routed| {
							menu_components::MenuBackRequested::routed(routed)
						});
				}
				input
					.clone()
					.map(|_| MenuNav::Back)
					.trigger(&mut commands, move |routed| MenuNavImpulse::routed(routed));
				return;
			}
			if !events.is_empty() {
				input.consume_frame(&mut commands);
			}
			for nav in &events {
				let nav = *nav;
				input
					.clone()
					.map(move |_| nav)
					.trigger(&mut commands, move |routed| MenuNavImpulse::routed(routed));
			}
			if events.is_empty() && !context_blocked {
				use maybraid_virtual_pad_controller::PadButton;
				let action = if frame.pad.just_pressed(PadButton::Y) {
					Some(menu_components::MenuAction::Unequip)
				} else if frame.pad.just_pressed(PadButton::X) {
					Some(menu_components::MenuAction::Drop)
				} else {
					None
				};
				if let Some(action) = action {
					input.consume_frame(&mut commands);
					input.clone().map(move |_| action).trigger(&mut commands, move |routed| {
						menu_components::MenuActionImpulse::routed(routed)
					});
				}
			}
			return;
		}
		ancestor = parents.get(entity).ok().map(ChildOf::parent);
	}
}

/// Per-session frame bookkeeping contains no hardware or global host state.
pub fn begin_scoped_menu_frame(mut sessions: Query<&mut crate::MenuSession>) {
	for mut session in &mut sessions {
		session.modals.begin_frame();
	}
}

/// A source-specific controller always receives its own session state.
pub fn ensure_scoped_menu_sessions(
	controllers: Query<(Entity, &MenuController), Without<crate::MenuSession>>,
	mut commands: Commands,
) {
	for (entity, controller) in &controllers {
		if controller.source.is_some() {
			commands.entity(entity).try_insert(crate::MenuSession::default());
		}
	}
}
