//! `MenuNavPad` → [`MenuNavImpulse`] on the focused [`UiTarget`].

use bevy::prelude::*;
use maybraid_controllers_core::{
	CaptureKind, CaptureToken, InputRouting, InputSource, ScopeOwner, UiTarget,
};
use menu_components::MenuNavPad;
use menu_components::{
	HudMenu, HudOverlayMenu, MenuModal, MenuModalId, MenuModalStack, ShortTextModal, ShortTextPad,
	TextMenu, TextMenuInputLock,
};

use crate::controller::MenuController;

/// Tokens this menu controller holds on [`InputRouter`]. Releasing one cannot
/// release another owner's capture.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuNavCapture {
	pub token: Option<CaptureToken>,
	pub awaiting: Option<CaptureToken>,
	pub text: Option<CaptureToken>,
}

/// One modal owns directional nav, or none does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExclusiveNav {
	/// Overlay catalog (or no modal). Parent controllers may handle stick / keys.
	Unclaimed,
	/// A nav-owning layer is on top, but its HudMenu has not spawned yet.
	/// Dispatch must not fall through to the parent catalog.
	Awaiting,
	/// Route stick / D-pad / keyboard to this menu.
	Target(UiTarget),
}

impl ExclusiveNav {
	pub fn target(self) -> Option<UiTarget> {
		match self {
			Self::Target(target) => Some(target),
			Self::Unclaimed | Self::Awaiting => None,
		}
	}

	pub fn target_entity(self) -> Option<Entity> {
		self.target().map(UiTarget::entity)
	}

	/// Derive the exclusive nav target from the stack's top owning layer.
	pub fn resolve(
		_modal: &ShortTextModal,
		stack: &MenuModalStack,
		pads: &Query<
			Entity,
			(With<ShortTextPad>, With<HudMenu>, Without<menu_components::ScopedMenuSession>),
		>,
		roots: &Query<(Entity, &MenuModal), Without<menu_components::ScopedMenuSession>>,
		menus: &Query<Entity, With<HudMenu>>,
		children: &Query<&Children>,
	) -> Self {
		let Some(id) = stack.nav_owner() else {
			return Self::Unclaimed;
		};
		let target = match id {
			MenuModalId::SHORT_TEXT => pads.iter().next(),
			_ => roots.iter().filter(|(_, modal)| modal.id == id).find_map(|(root, _)| {
				std::iter::once(root)
					.chain(children.iter_descendants(root))
					.find(|entity| menus.contains(*entity))
			}),
		};
		match target {
			Some(entity) => Self::Target(UiTarget::new(entity)),
			None => Self::Awaiting,
		}
	}

	pub fn capture_owner(self) -> Option<ScopeOwner> {
		self.target().map(ScopeOwner::ui)
	}
}

/// Derive the exclusive nav target from the stack's top owning layer.
pub fn exclusive_nav_menu(
	modal: &ShortTextModal,
	stack: &MenuModalStack,
	pads: &Query<
		Entity,
		(With<ShortTextPad>, With<HudMenu>, Without<menu_components::ScopedMenuSession>),
	>,
	roots: &Query<(Entity, &MenuModal), Without<menu_components::ScopedMenuSession>>,
	menus: &Query<Entity, With<HudMenu>>,
	children: &Query<&Children>,
) -> ExclusiveNav {
	ExclusiveNav::resolve(modal, stack, pads, roots, menus, children)
}

pub fn refresh_primary_menu_focus(
	primary: Res<maybraid_virtual_pad_controller::PrimaryVirtualPad>,
	children: Query<&Children>,
	overlays: Query<Entity, With<HudOverlayMenu>>,
	hud_menus: Query<Entity, (With<HudMenu>, Without<HudOverlayMenu>)>,
	all_menus: Query<Entity, With<HudMenu>>,
	text_menus: Query<Entity, With<TextMenu>>,
	modal: Res<ShortTextModal>,
	stack: Res<MenuModalStack>,
	pads: Query<
		Entity,
		(With<ShortTextPad>, With<HudMenu>, Without<menu_components::ScopedMenuSession>),
	>,
	roots: Query<(Entity, &MenuModal), Without<menu_components::ScopedMenuSession>>,
	mut controllers: Query<(Entity, &mut MenuController)>,
	hosts: Query<(), With<menu_components::PrimaryMenuInput>>,
	mut router: InputRouting,
	mut menu_capture: ResMut<MenuNavCapture>,
	mut commands: Commands,
) {
	let exclusive = ExclusiveNav::resolve(&modal, &stack, &pads, &roots, &all_menus, &children);
	let mut owner = None;
	let mut awaiting_owner = None;
	for (root, mut controller) in &mut controllers {
		if controller.source.is_some() || !hosts.contains(root) {
			continue;
		}
		awaiting_owner.get_or_insert(ScopeOwner::ui(UiTarget::new(root)));
		owner = exclusive.capture_owner();
		controller.blocked = modal.dismissed || stack.consumed();
		controller.context_blocked = modal.is_open() || stack.owns_nav();
		controller.focus = match exclusive {
			ExclusiveNav::Target(menu) => {
				if !menu.is_under(root, &children) {
					for (modal_root, modal) in &roots {
						if Some(modal.id) == stack.nav_owner() {
							commands.entity(root).add_child(modal_root);
						}
					}
				}
				Some(menu)
			}
			ExclusiveNav::Awaiting => None,
			ExclusiveNav::Unclaimed => {
				MenuController::resolve(root, &children, &overlays, &hud_menus, &text_menus)
					.map(UiTarget::new)
			}
		};
		if owner.is_none() {
			if let Some(focus) = controller.focus {
				owner = Some(ScopeOwner::ui(focus));
			}
		}
	}
	router.replace_owned_capture(
		&mut menu_capture.token,
		InputSource::virtual_pad(primary.0),
		owner,
		CaptureKind::MenuNav,
	);
	router.replace_owned_capture(
		&mut menu_capture.awaiting,
		InputSource::virtual_pad(primary.0),
		if matches!(exclusive, ExclusiveNav::Awaiting) { awaiting_owner } else { None },
		CaptureKind::Overlay,
	);
}

// Explicit primary host adapter: the root owns its frame and repeat state.
use maybraid_virtual_pad_controller::PrimaryVirtualPad;

/// Keep a previously selected default root, or select the sole eligible root.
/// Ambiguous default controllers do not receive primary input.
pub fn sync_primary_menu_host(
	primary: Res<PrimaryVirtualPad>,
	mut controllers: Query<(
		Entity,
		&mut MenuController,
		Option<&menu_components::PrimaryMenuInput>,
	)>,
	mut nav: ResMut<MenuNavPad>,
	mut commands: Commands,
) {
	let mut selected_hosts = controllers
		.iter()
		.filter(|(_, controller, input)| controller.source.is_none() && input.is_some())
		.map(|(entity, ..)| entity);
	let existing = selected_hosts.next().filter(|_| selected_hosts.next().is_none());
	let selected = existing.or_else(|| {
		let mut eligible = controllers
			.iter()
			.filter(|(_, controller, _)| controller.source.is_none())
			.map(|(entity, ..)| entity);
		eligible.next().filter(|_| eligible.next().is_none())
	});
	nav.events.clear();
	for (entity, mut controller, input) in &mut controllers {
		if Some(entity) == selected {
			if input.is_none_or(|input| input.source != primary.0) {
				controller.streams.clear();
				commands
					.entity(entity)
					.try_insert(menu_components::PrimaryMenuInput::new(primary.0));
			}
		} else if input.is_some() {
			controller.streams.clear();
			commands.entity(entity).try_remove::<menu_components::PrimaryMenuInput>();
		}
	}
}

/// No frame survives into the next delivery pass without fresh authorization.
pub fn clear_primary_menu_frame(
	mut hosts: Query<&mut menu_components::PrimaryMenuInput>,
	mut nav: ResMut<MenuNavPad>,
) {
	nav.events.clear();
	for mut host in &mut hosts {
		host.frame = None;
	}
}

pub fn sync_text_menu_capture(
	primary: Res<maybraid_virtual_pad_controller::PrimaryVirtualPad>,
	hosts: Query<(Entity, &menu_components::PrimaryMenuInput)>,
	lock: Res<TextMenuInputLock>,
	modal: Res<ShortTextModal>,
	mut router: InputRouting,
	mut menu_capture: ResMut<MenuNavCapture>,
) {
	let owner = hosts
		.single()
		.ok()
		.filter(|(_, host)| host.source == primary.0)
		.map(|(entity, _)| ScopeOwner::ui(UiTarget::new(entity)));
	router.replace_owned_capture(
		&mut menu_capture.text,
		InputSource::virtual_pad(primary.0),
		if lock.0 && !modal.is_open() { owner } else { None },
		CaptureKind::TextEntry,
	);
}
