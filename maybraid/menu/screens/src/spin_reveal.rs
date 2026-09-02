//! Spin-and-reveal screen for starter clothing (picture or camera slot).

use bevy::prelude::*;
use bevy::scene::prelude::{Scene, bsn};
use crozon_character_items::InventoryItem;
use maybraid_menu_controller::MenuController;
use menu_components::{
	MENU_CLEAR, SPIN_REVEAL_SECS, SpinRevealSlot, TextCursorColumn, TextMenuPlugin,
	republish_menu_activate,
};

use crate::MenuScreen;
use crate::input::add_menu_input;
use crate::show::take_menu_show_request;

/// Queue a spin-and-reveal spawn. Pair with [`SpinRevealItems`].
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowSpinReveal;

/// Items to reveal, in order.
#[derive(Resource, Clone, Debug)]
pub struct SpinRevealItems(pub Vec<InventoryItem>);

/// Marker on the spawned spin-and-reveal root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct SpinRevealScreen;

/// Advance the current card, or finish after the last reveal.
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum SpinRevealChoice {
	#[default]
	Advance,
}

/// Fired once every item has been revealed and the player continues.
#[derive(Message, Clone, Debug)]
pub struct SpinRevealFinished {
	pub items: Vec<InventoryItem>,
}

#[derive(Resource, Debug, Clone)]
struct SpinRevealPhase {
	index: usize,
	spinning: bool,
	elapsed: f32,
	dirty: bool,
}

impl SpinRevealPhase {
	fn start() -> Self {
		Self { index: 0, spinning: true, elapsed: 0.0, dirty: true }
	}
}

pub fn request_show_spin_reveal(commands: &mut Commands, items: Vec<InventoryItem>) {
	commands.insert_resource(SpinRevealItems(items));
	commands.spawn(RequestShowSpinReveal);
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpinRevealSystems {
	Apply,
}

pub struct SpinRevealScreenPlugin;

impl Plugin for SpinRevealScreenPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.add_plugins(TextMenuPlugin::<SpinRevealChoice>::default())
			.add_message::<SpinRevealFinished>()
			.add_systems(
				Update,
				(apply_show_spin_reveal, tick_spin_reveal, finish_spin_reveal, rebuild_spin_reveal)
					.chain()
					.in_set(SpinRevealSystems::Apply),
			);
	}
}

fn apply_show_spin_reveal(
	mut commands: Commands,
	requests: Query<Entity, With<RequestShowSpinReveal>>,
	existing: Query<Entity, With<MenuScreen>>,
	items: Option<Res<SpinRevealItems>>,
) {
	if !take_menu_show_request(&mut commands, &requests, &existing) {
		return;
	}
	if items.is_none() {
		return;
	}
	commands.insert_resource(SpinRevealPhase::start());
}

fn tick_spin_reveal(time: Res<Time>, mut phase: Option<ResMut<SpinRevealPhase>>) {
	let Some(phase) = phase.as_mut() else {
		return;
	};
	if !phase.spinning {
		return;
	}
	phase.elapsed += time.delta_secs();
	if phase.elapsed >= SPIN_REVEAL_SECS {
		phase.spinning = false;
		phase.dirty = true;
	}
}

fn rebuild_spin_reveal(
	mut commands: Commands,
	items: Option<Res<SpinRevealItems>>,
	mut phase: Option<ResMut<SpinRevealPhase>>,
	screens: Query<Entity, With<SpinRevealScreen>>,
) {
	let Some(phase) = phase.as_mut() else {
		return;
	};
	if !phase.dirty {
		return;
	}
	let Some(items) = items else {
		return;
	};
	for entity in &screens {
		commands.entity(entity).despawn();
	}
	if items.0.is_empty() {
		phase.dirty = false;
		return;
	}
	let index = phase.index.min(items.0.len().saturating_sub(1));
	let revealed = !phase.spinning;
	let last = index + 1 >= items.0.len();
	let action = if revealed && last { "Continue" } else { "Next" };
	commands.spawn_scene(spin_scene(&items.0[index], revealed, action));
	phase.dirty = false;
}

fn spin_scene(item: &InventoryItem, revealed: bool, action: &'static str) -> impl Scene + 'static {
	let material = item.material();
	let subtitle = format!("{} · {}", material.id.label(), material.color.label());
	let mut slot = SpinRevealSlot::camera(item.label(), subtitle, item.path());
	slot.revealed = revealed;
	let children: Vec<Box<dyn Scene>> = vec![
		Box::new(slot.scene()),
		Box::new(TextCursorColumn::untitled([(action, SpinRevealChoice::Advance)]).scene()),
	];
	bsn! {
		SpinRevealScreen
		MenuScreen
		MenuController
		BackgroundColor(MENU_CLEAR)
		Node {
			width: percent(100),
			height: percent(100),
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			flex_direction: FlexDirection::Column,
			row_gap: px(32),
		}
		Pickable::IGNORE
		on(republish_menu_activate::<SpinRevealChoice>)
		Children [ {children} ]
	}
}

fn finish_spin_reveal(
	mut commands: Commands,
	mut choices: MessageReader<SpinRevealChoice>,
	mut finished: MessageWriter<SpinRevealFinished>,
	items: Option<Res<SpinRevealItems>>,
	mut phase: Option<ResMut<SpinRevealPhase>>,
) {
	if !choices.read().any(|choice| *choice == SpinRevealChoice::Advance) {
		return;
	}
	let Some(phase) = phase.as_mut() else {
		return;
	};
	let Some(items) = items else {
		return;
	};
	if phase.spinning {
		phase.spinning = false;
		phase.dirty = true;
		return;
	}
	if phase.index + 1 < items.0.len() {
		phase.index += 1;
		phase.spinning = true;
		phase.elapsed = 0.0;
		phase.dirty = true;
		return;
	}
	finished.write(SpinRevealFinished { items: items.0.clone() });
	commands.remove_resource::<SpinRevealPhase>();
}

impl SpinRevealScreen {
	pub fn scene(item: &InventoryItem, revealed: bool) -> impl Scene + 'static {
		spin_scene(item, revealed, "Next")
	}
}
