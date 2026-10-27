//! Addressed spawn-picker input. The host selects a primary picker explicitly;
//! each picker entity owns its capture and navigation repeat state.
use crate::arrival::WorldPlayerRespawnState;
use crate::request::{DestinationSelected, PlayerChoseRespawnPoi};
use bevy::prelude::*;
use maybraid_controllers_core::{
	CaptureKind, CaptureToken, InputRouting, InputSource, RouteBinding, ScopeOwner, UiPadEvent,
	UiTarget,
};
use maybraid_menu_controller::repeat::MenuRepeat;
use maybraid_virtual_pad_controller::{PrimaryVirtualPad, VirtualPadController};

/// Projection of the host's single discovery map, not controller input state.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpawnPickerSheet {
	pub open: bool,
	pub locked: bool,
	pub map_capture_owner: Option<UiTarget>,
}
impl SpawnPickerSheet {
	pub const fn from_map(open: bool, close_locked: bool) -> Self {
		Self { open, locked: close_locked, map_capture_owner: None }
	}
	pub const fn owns_pending(self, pending: bool) -> bool {
		self.open && self.locked && pending
	}
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpawnPickerCapture {
	pub token: Option<CaptureToken>,
}
impl SpawnPickerCapture {
	pub fn release(&mut self, router: &mut InputRouting) {
		if let Some(token) = self.token.take() {
			router.release_capture(token);
		}
	}
}

#[derive(Component, Clone, Debug)]
#[require(SpawnPickerCapture)]
pub struct SpawnPickerController {
	pub source: InputSource,
	pub focus: bool,
	binding: Option<RouteBinding>,
	sequence: Option<u64>,
	repeat: MenuRepeat,
	await_release: bool,
}
impl SpawnPickerController {
	pub fn new(source: InputSource) -> Self {
		Self {
			source,
			focus: false,
			binding: None,
			sequence: None,
			repeat: Default::default(),
			await_release: false,
		}
	}
	pub fn owner(entity: Entity) -> ScopeOwner {
		ScopeOwner::ui(UiTarget::new(entity))
	}
	pub fn confirm(pending: &crate::arrival::PendingPlayerRespawn) -> PlayerChoseRespawnPoi {
		PlayerChoseRespawnPoi::new(pending.identity(), pending.highlighted)
	}
}

/// The discovery host presently exposes one spawn sheet. Its input source is
/// explicit and remains separate from controller-owned repeat/capture state.
pub fn refresh_spawn_picker_focus(
	sheet: Res<SpawnPickerSheet>,
	state: Res<WorldPlayerRespawnState>,
	primary: Option<Res<PrimaryVirtualPad>>,
	mut pickers: Query<(Entity, &mut SpawnPickerController, &mut SpawnPickerCapture)>,
	mut commands: Commands,
	mut router: InputRouting,
) {
	let Some(primary) = primary else {
		return;
	};
	let source = InputSource::virtual_pad(primary.0);
	let wanted = sheet.owns_pending(state.pending.is_some());
	if pickers.is_empty() {
		commands.spawn(SpawnPickerController::new(source));
		return;
	}
	for (entity, mut picker, mut capture) in &mut pickers {
		if picker.source != source {
			continue;
		}
		let blocked = router.top_capture_for(source).is_some_and(|layer| {
			layer.owner() != SpawnPickerController::owner(entity)
				&& Some(layer.owner()) != sheet.map_capture_owner.map(ScopeOwner::ui)
		});
		picker.focus = wanted && !blocked;
		router.replace_owned_capture(
			&mut capture.token,
			source,
			(wanted && !blocked).then(|| SpawnPickerController::owner(entity)),
			CaptureKind::MenuNav,
		);
	}
}

pub fn dispatch_spawn_picker(
	event: On<UiPadEvent>,
	mut pickers: Query<&mut SpawnPickerController>,
	pads: Query<&VirtualPadController>,
	mut commands: Commands,
) {
	let Ok(mut picker) = pickers.get_mut(event.entity) else {
		return;
	};
	if !picker.focus || picker.source != event.input.source() {
		return;
	}
	let frame = event.input.payload();
	let binding = event.input.binding();
	if picker.binding == binding
		&& picker.sequence.is_some_and(|sequence| frame.sequence <= sequence)
	{
		return;
	}
	if picker.binding != binding {
		picker.await_release = picker.sequence.is_some();
		picker.repeat = Default::default();
		picker.binding = binding;
	}
	picker.sequence = Some(frame.sequence);
	if picker.await_release {
		use maybraid_virtual_pad_controller::PadButton;
		if frame.pad.pressed(PadButton::A)
			|| frame.pad.pressed(PadButton::Start)
			|| frame.pad.move_stick.length_squared() > 0.01
			|| frame.pad.dpad.length_squared() > 0.01
		{
			return;
		}
		picker.await_release = false;
	}
	let Ok(pad) = pads.get(frame.source) else {
		return;
	};
	picker.repeat.derive(&frame.pad, &pad.config, frame.delta.as_secs_f32());
	let nav = picker.repeat.events.clone();
	if nav.is_empty() {
		return;
	}
	event.input.consume_frame(&mut commands);
	event.input.clone().map(|_| nav).queue(&mut commands, |world, input| {
		world.resource_scope(|world, registry: Mut<poi_intelligence::PoiRegistry>| {
			let mut state = world.resource_mut::<WorldPlayerRespawnState>();
			let Some(pending) = state.pending.as_mut() else {
				return;
			};
			if pending.apply_menu_nav(&registry, input.payload()) {
				let choice = SpawnPickerController::confirm(pending);
				world.write_message(choice);
				world
					.write_message(DestinationSelected::new(choice.identity, choice.destination()));
			}
		});
	});
}

pub fn release_spawn_picker(
	mut pickers: Query<(&mut SpawnPickerController, &mut SpawnPickerCapture)>,
	mut router: InputRouting,
) {
	for (mut picker, mut capture) in &mut pickers {
		picker.focus = false;
		capture.release(&mut router);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_controllers_core::InputRouter;
	use maybraid_virtual_pad_controller::{PadButton, PadDevices, VirtualPad, VirtualPadEvent};

	fn picker_world() -> (World, Entity, Entity) {
		let mut world = World::new();
		world.init_resource::<InputRouter>();
		world.init_resource::<poi_intelligence::PoiRegistry>();
		world.init_resource::<WorldPlayerRespawnState>();
		world.init_resource::<Messages<PlayerChoseRespawnPoi>>();
		world.init_resource::<Messages<DestinationSelected>>();
		world.add_observer(dispatch_spawn_picker);
		world.resource_mut::<WorldPlayerRespawnState>().pending =
			Some(crate::arrival::PendingPlayerRespawn::test(Vec3::ZERO, false));
		let source = world.spawn(VirtualPadController::new(PadDevices::keyboard_mouse())).id();
		let mut controller = SpawnPickerController::new(InputSource::virtual_pad(source));
		controller.focus = true;
		let picker = world.spawn(controller).id();
		world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::virtual_pad(source),
			SpawnPickerController::owner(picker),
			CaptureKind::MenuNav,
		);
		(world, source, picker)
	}

	fn frame(
		world: &mut World,
		source: Entity,
		picker: Entity,
		sequence: u64,
		pressed: bool,
	) -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		if pressed {
			pad.hold_digital(PadButton::A);
		}
		pad.finish_digital();
		let input = world
			.resource::<InputRouter>()
			.route_ui(
				InputSource::virtual_pad(source),
				VirtualPadEvent {
					source,
					sequence,
					delta: std::time::Duration::from_millis(16),
					pad,
				},
			)
			.ok_or_else(|| anyhow::anyhow!("picker capture"))?;
		world.trigger(UiPadEvent { entity: picker, input });
		world.flush();
		Ok(())
	}

	#[test]
	fn confirm_is_addressed_and_duplicate_frames_are_ignored() -> anyhow::Result<()> {
		let (mut world, source, picker) = picker_world();
		frame(&mut world, source, picker, 1, true)?;
		frame(&mut world, source, picker, 1, true)?;
		assert_eq!(world.resource_mut::<Messages<PlayerChoseRespawnPoi>>().drain().count(), 1);
		Ok(())
	}

	#[test]
	fn unrelated_source_cannot_confirm_primary_picker() -> anyhow::Result<()> {
		let (mut world, _, picker) = picker_world();
		let other = world.spawn(VirtualPadController::new(PadDevices::default())).id();
		world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::virtual_pad(other),
			SpawnPickerController::owner(picker),
			CaptureKind::MenuNav,
		);
		frame(&mut world, other, picker, 1, true)?;
		assert_eq!(world.resource_mut::<Messages<PlayerChoseRespawnPoi>>().drain().count(), 0);
		Ok(())
	}

	#[test]
	fn retired_capture_requires_release_before_confirm() -> anyhow::Result<()> {
		let (mut world, source, picker) = picker_world();
		frame(&mut world, source, picker, 1, false)?;
		let owner = world.spawn_empty().id();
		let token = world.resource_mut::<InputRouter>().acquire_capture(
			InputSource::virtual_pad(source),
			ScopeOwner::ui(UiTarget::new(owner)),
			CaptureKind::Overlay,
		);
		world.resource_mut::<InputRouter>().release_capture(token);
		frame(&mut world, source, picker, 2, true)?;
		assert_eq!(world.resource_mut::<Messages<PlayerChoseRespawnPoi>>().drain().count(), 0);
		frame(&mut world, source, picker, 3, false)?;
		frame(&mut world, source, picker, 4, true)?;
		assert_eq!(world.resource_mut::<Messages<PlayerChoseRespawnPoi>>().drain().count(), 1);
		Ok(())
	}
}
