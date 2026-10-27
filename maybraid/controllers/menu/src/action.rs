//! Context actions use the same exclusive UI capture as menu navigation.

use bevy::prelude::*;
use maybraid_controllers_core::{CaptureKind, InputRouter, InputSource, Routed};
use maybraid_input::{MenuNavPad, VirtualPad, INVENTORY_DROP_PAD, INVENTORY_UNEQUIP_PAD};
use menu_components::{MenuModalStack, ShortTextModal, TextMenuInputLock};

/// Semantic menu actions. Renderers interpret these without reading hardware.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
	Unequip,
	Drop,
}

/// Delivered only to the owning menu; it must not bubble into a parent catalog.
#[derive(EntityEvent, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuActionImpulse {
	pub entity: Entity,
	pub source: InputSource,
	pub action: MenuAction,
}

impl MenuActionImpulse {
	pub fn from_routed(routed: Routed<MenuAction>) -> Option<Self> {
		Some(Self {
			entity: routed.target().ui_entity()?,
			source: routed.source(),
			action: routed.into_payload(),
		})
	}
}

pub fn dispatch_menu_actions(
	pad: Res<VirtualPad>,
	nav: Res<MenuNavPad>,
	router: Res<InputRouter>,
	lock: Res<TextMenuInputLock>,
	modal: Res<ShortTextModal>,
	stack: Res<MenuModalStack>,
	mut commands: Commands,
) {
	// Back/text consumption wins even if focus has not refreshed after dismissal.
	if lock.0 || modal.is_open() || modal.dismissed || stack.consumed() || stack.owns_nav() {
		return;
	}
	let source = InputSource::local_pad();
	if !router.connected(source) {
		return;
	}
	// Navigation owns a frame with a simultaneous context action. Y wins over
	// X if both context buttons are pressed together.
	if !nav.events.is_empty() {
		return;
	}
	let action = if pad.just_pressed(INVENTORY_UNEQUIP_PAD) {
		Some(MenuAction::Unequip)
	} else if pad.just_pressed(INVENTORY_DROP_PAD) {
		Some(MenuAction::Drop)
	} else {
		None
	};
	let Some(action) = action else {
		return;
	};
	let Some(layer) = router.top_capture() else {
		return;
	};
	if layer.kind() != CaptureKind::MenuNav || layer.source() != source {
		return;
	}
	let Some(target) = layer.ui_owner() else {
		return;
	};
	let routed = Routed::to_ui(source, target, action);
	if let Some(impulse) = MenuActionImpulse::from_routed(routed) {
		commands.trigger(impulse);
	}
}

#[cfg(test)]
mod tests {
	use crate::action::{dispatch_menu_actions, MenuAction, MenuActionImpulse};
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use maybraid_controllers_core::{CaptureKind, InputRouter, InputSource, ScopeOwner, UiTarget};
	use maybraid_input::{MenuNav, MenuNavPad, PadButton, VirtualPad};
	use menu_components::{MenuModalLayer, MenuModalStack, ShortTextModal, TextMenuInputLock};

	#[derive(Resource, Default)]
	struct Hits(Vec<MenuActionImpulse>);

	fn record(impulse: On<MenuActionImpulse>, mut hits: ResMut<Hits>) {
		hits.0.push(*impulse.event());
	}

	fn world() -> (World, Entity) {
		world_with_button(PadButton::Y)
	}

	fn world_with_button(button: PadButton) -> (World, Entity) {
		world_with_buttons(&[button])
	}

	fn world_with_buttons(buttons: &[PadButton]) -> (World, Entity) {
		let mut world = World::new();
		world.init_resource::<InputRouter>();
		world.init_resource::<MenuNavPad>();
		world.init_resource::<TextMenuInputLock>();
		world.init_resource::<ShortTextModal>();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<Hits>();
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		for button in buttons {
			pad.hold_digital(*button);
		}
		pad.finish_digital();
		world.insert_resource(pad);
		world.add_observer(record);
		let menu = world.spawn_empty().id();
		world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::ui(UiTarget::new(menu)),
			CaptureKind::MenuNav,
		);
		(world, menu)
	}

	fn dispatch(world: &mut World) -> anyhow::Result<()> {
		world
			.run_system_once(dispatch_menu_actions)
			.map_err(|e| anyhow::anyhow!("{e:?}"))?;
		world.flush();
		Ok(())
	}

	#[test]
	fn y_delivers_once_to_the_owning_menu_with_its_source() -> anyhow::Result<()> {
		let (mut world, menu) = world();
		let parent = world.spawn_empty().id();
		world.entity_mut(parent).add_child(menu);
		dispatch(&mut world)?;
		assert_eq!(
			world.resource::<Hits>().0,
			vec![MenuActionImpulse {
				entity: menu,
				source: InputSource::local_pad(),
				action: MenuAction::Unequip,
			}]
		);
		let mut pad = world.resource_mut::<VirtualPad>();
		pad.begin_frame();
		pad.hold_digital(PadButton::Y);
		pad.finish_digital();
		dispatch(&mut world)?;
		assert_eq!(world.resource::<Hits>().0.len(), 1, "holding Y must not repeat");
		Ok(())
	}

	#[test]
	fn higher_menu_receives_the_action_instead_of_inventory() -> anyhow::Result<()> {
		let (mut world, inventory) = world();
		let modal = world.spawn_empty().id();
		world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::local_pad(),
			ScopeOwner::ui(UiTarget::new(modal)),
			CaptureKind::MenuNav,
		);
		dispatch(&mut world)?;
		assert_eq!(world.resource::<Hits>().0.len(), 1);
		assert_eq!(world.resource::<Hits>().0[0].entity, modal);
		assert_ne!(world.resource::<Hits>().0[0].entity, inventory);
		Ok(())
	}

	#[test]
	fn x_delivers_drop_to_the_captured_menu_and_nav_takes_priority() -> anyhow::Result<()> {
		let (mut world, menu) = world_with_button(PadButton::X);
		dispatch(&mut world)?;
		assert_eq!(
			world.resource::<Hits>().0,
			vec![MenuActionImpulse {
				entity: menu,
				source: InputSource::local_pad(),
				action: MenuAction::Drop,
			}]
		);

		for nav in [MenuNav::Select, MenuNav::Right] {
			let (mut world, _) = world_with_button(PadButton::X);
			world.resource_mut::<MenuNavPad>().events.push(nav);
			dispatch(&mut world)?;
			assert!(
				world.resource::<Hits>().0.is_empty(),
				"navigation must win over simultaneous Drop"
			);
		}
		Ok(())
	}

	#[test]
	fn navigation_wins_over_unequip_and_y_has_priority_over_x() -> anyhow::Result<()> {
		let (mut world, _) = world_with_button(PadButton::Y);
		world.resource_mut::<MenuNavPad>().events.push(MenuNav::Select);
		dispatch(&mut world)?;
		assert!(world.resource::<Hits>().0.is_empty(), "Select must win over Unequip");

		let (mut world, menu) = world_with_buttons(&[PadButton::X, PadButton::Y]);
		dispatch(&mut world)?;
		assert_eq!(
			world.resource::<Hits>().0,
			vec![MenuActionImpulse {
				entity: menu,
				source: InputSource::local_pad(),
				action: MenuAction::Unequip,
			}]
		);
		Ok(())
	}

	#[test]
	fn holding_x_does_not_repeat_drop() -> anyhow::Result<()> {
		let (mut world, _) = world_with_button(PadButton::X);
		dispatch(&mut world)?;
		let mut pad = world.resource_mut::<VirtualPad>();
		pad.begin_frame();
		pad.hold_digital(PadButton::X);
		pad.finish_digital();
		drop(pad);
		dispatch(&mut world)?;
		assert_eq!(world.resource::<Hits>().0.len(), 1, "held X must not repeat Drop");
		Ok(())
	}

	#[test]
	fn overlay_text_and_awaiting_capture_block_inventory_actions() -> anyhow::Result<()> {
		for (kind, owner) in [
			(CaptureKind::Overlay, ScopeOwner::map_overlay()),
			(CaptureKind::Overlay, ScopeOwner::awaiting_nav()),
			(CaptureKind::TextEntry, ScopeOwner::console()),
		] {
			for button in [PadButton::X, PadButton::Y] {
				let (mut world, _) = world_with_button(button);
				world.resource_mut::<InputRouter>().acquire_capture(
					InputSource::local_pad(),
					owner,
					kind,
				);
				dispatch(&mut world)?;
				assert!(
					world.resource::<Hits>().0.is_empty(),
					"capture {kind:?} must block {button:?}"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn nav_owning_modal_blocks_unequip_and_drop() -> anyhow::Result<()> {
		for button in [PadButton::X, PadButton::Y] {
			let (mut world, _) = world_with_button(button);
			world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
			dispatch(&mut world)?;
			assert!(world.resource::<Hits>().0.is_empty(), "item-drop modal must block {button:?}");
		}
		Ok(())
	}

	#[test]
	fn dismissal_lock_and_disconnect_block_stale_focus() -> anyhow::Result<()> {
		let (mut world, _) = world();
		world.resource_mut::<ShortTextModal>().dismissed = true;
		dispatch(&mut world)?;
		world.resource_mut::<ShortTextModal>().dismissed = false;
		world.resource_mut::<TextMenuInputLock>().0 = true;
		dispatch(&mut world)?;
		world.resource_mut::<TextMenuInputLock>().0 = false;
		world.resource_mut::<InputRouter>().disconnect(InputSource::local_pad());
		dispatch(&mut world)?;
		assert!(world.resource::<Hits>().0.is_empty());
		Ok(())
	}

	#[test]
	fn an_open_text_modal_blocks_actions_without_a_host_text_lock() -> anyhow::Result<()> {
		use menu_components::controls::short_text::ShortTextSession;
		let (mut world, menu) = world();
		world.resource_mut::<ShortTextModal>().session = Some(ShortTextSession {
			key: "name",
			source: menu,
			value: String::new(),
			original: String::new(),
			max_len: 16,
			shift: false,
		});
		dispatch(&mut world)?;
		assert!(world.resource::<Hits>().0.is_empty());
		Ok(())
	}
}
