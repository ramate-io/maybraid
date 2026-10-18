//! Read-only cycle-slot strip: one highlight on the selected Lubikan or item.

use bevy::prelude::*;
use lubikans::{LubikanAccountId, LubikanLedger, LubikanOwner};
use menu_components::{HudFonts, TEXT_YELLOW};
use player::Player;

use crate::cycle_cue::{CombatHudEquippedQueue, CombatHudInventoryCycleCategory};
use crate::CombatHudVisible;

/// Tunable highlight chrome. Bright border + dark outline so the selected
/// chip stays readable on both dark and light backdrops.
pub(crate) const CYCLE_SLOT_POOL: usize = 8;
pub(crate) const CYCLE_SLOT_BORDER_PX: f32 = 2.0;
pub(crate) const CYCLE_SLOT_OUTLINE_PX: f32 = 2.0;
pub(crate) const CYCLE_SLOT_OUTLINE_OFFSET_PX: f32 = 2.0;
pub(crate) const CYCLE_SLOT_RADIUS_PX: f32 = 5.0;
pub(crate) const CYCLE_SLOT_MIN_WIDTH_PX: f32 = 44.0;
pub(crate) const CYCLE_SLOT_HEIGHT_PX: f32 = 22.0;
pub(crate) const CYCLE_SLOT_GAP_PX: f32 = 4.0;
pub(crate) const CYCLE_SLOT_PAD_X_PX: f32 = 6.0;
pub(crate) const CYCLE_SLOT_FONT_PX: f32 = 12.0;

pub(crate) const CYCLE_SLOT_SELECTED_BORDER: Color = Color::srgb(1.0, 0.92, 0.28);
pub(crate) const CYCLE_SLOT_SELECTED_OUTLINE: Color = Color::srgba(0.06, 0.07, 0.09, 0.94);
pub(crate) const CYCLE_SLOT_SELECTED_FILL: Color = Color::srgba(0.28, 0.26, 0.12, 0.88);
pub(crate) const CYCLE_SLOT_SELECTED_GLOW: Color = Color::srgba(1.0, 0.94, 0.45, 0.22);
pub(crate) const CYCLE_SLOT_NEUTRAL_BORDER: Color = Color::srgba(0.70, 0.72, 0.76, 0.55);
pub(crate) const CYCLE_SLOT_NEUTRAL_FILL: Color = Color::srgba(0.16, 0.17, 0.20, 0.70);
pub(crate) const CYCLE_SLOT_NEUTRAL_TEXT: Color = Color::srgb(0.72, 0.74, 0.78);

pub(crate) const CYCLE_SLOT_PULSE_HZ: f32 = 1.15;
pub(crate) const CYCLE_SLOT_PULSE_ALPHA_MIN: f32 = 0.72;
pub(crate) const CYCLE_SLOT_PULSE_ALPHA_MAX: f32 = 1.0;
pub(crate) const CYCLE_SLOT_BUMP_SECS: f32 = 0.16;
pub(crate) const CYCLE_SLOT_BUMP_SCALE: f32 = 1.08;

#[derive(Component)]
pub(crate) struct CombatHudCycleSlotRow;

#[derive(Component, Default)]
pub(crate) struct CombatHudCycleSlotBump {
	key: Option<u64>,
	until: f32,
}

/// One cyclable chip. `selected` is the last synced selection bit.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CombatHudCycleSlot {
	pub index: usize,
	pub key: u64,
	pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CycleSlotView {
	pub key: u64,
	pub label: String,
	pub selected: bool,
}

pub(crate) fn spawn_cycle_slot_row(parent: &mut ChildSpawnerCommands, fonts: &HudFonts) {
	let font = fonts.item(CYCLE_SLOT_FONT_PX);
	parent
		.spawn((
			Name::new("cycle-slots"),
			CombatHudCycleSlotRow,
			CombatHudCycleSlotBump::default(),
			Node {
				width: Val::Percent(100.0),
				min_height: Val::Px(CYCLE_SLOT_HEIGHT_PX),
				flex_direction: FlexDirection::Row,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				column_gap: Val::Px(CYCLE_SLOT_GAP_PX),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			for index in 0..CYCLE_SLOT_POOL {
				row.spawn((
					CombatHudCycleSlot { index, key: 0, selected: false },
					Node {
						min_width: Val::Px(CYCLE_SLOT_MIN_WIDTH_PX),
						max_width: Val::Px(64.0),
						height: Val::Px(CYCLE_SLOT_HEIGHT_PX),
						padding: UiRect::axes(Val::Px(CYCLE_SLOT_PAD_X_PX), Val::Px(2.0)),
						overflow: Overflow::clip(),
						border: UiRect::all(Val::Px(CYCLE_SLOT_BORDER_PX)),
						border_radius: BorderRadius::all(Val::Px(CYCLE_SLOT_RADIUS_PX)),
						justify_content: JustifyContent::Center,
						align_items: AlignItems::Center,
						display: Display::None,
						..default()
					},
					BackgroundColor(CYCLE_SLOT_NEUTRAL_FILL),
					BorderColor::all(CYCLE_SLOT_NEUTRAL_BORDER),
					Outline::new(
						Val::Px(CYCLE_SLOT_OUTLINE_PX),
						Val::Px(CYCLE_SLOT_OUTLINE_OFFSET_PX),
						Color::NONE,
					),
					UiTransform::IDENTITY,
					Text::new(""),
					font.clone(),
					TextColor(CYCLE_SLOT_NEUTRAL_TEXT),
					Visibility::Hidden,
					Pickable::IGNORE,
				));
			}
		});
}

pub(crate) fn cycle_slot_views(
	category: CombatHudInventoryCycleCategory,
	queue: Option<&CombatHudEquippedQueue>,
	ledger: Option<&LubikanLedger>,
) -> Vec<CycleSlotView> {
	if category.is_equipped_items() {
		if let Some(queue) = queue.filter(|queue| !queue.slots.is_empty()) {
			return equipped_item_views(queue);
		}
	}
	ledger.map(lubikan_views).unwrap_or_default()
}

fn lubikan_views(ledger: &LubikanLedger) -> Vec<CycleSlotView> {
	let selected = ledger.selection().resolved();
	ledger
		.cycle_order()
		.into_iter()
		.map(|id| CycleSlotView {
			key: id.to_bits(),
			label: slot_label(id),
			selected: id == selected,
		})
		.collect()
}

fn equipped_item_views(queue: &CombatHudEquippedQueue) -> Vec<CycleSlotView> {
	queue
		.slots
		.iter()
		.enumerate()
		.map(|(index, slot)| CycleSlotView {
			key: slot.key,
			label: slot.label.clone(),
			selected: index == queue.selected,
		})
		.collect()
}

fn slot_label(account: LubikanAccountId) -> String {
	account
		.definition()
		.well_known_label()
		.map(str::to_string)
		.unwrap_or_else(|| format!("L{}", account.definition().raw()))
}

pub(crate) fn pulse_alpha(elapsed: f32) -> f32 {
	let wave = (elapsed * CYCLE_SLOT_PULSE_HZ * std::f32::consts::TAU).sin() * 0.5 + 0.5;
	CYCLE_SLOT_PULSE_ALPHA_MIN + (CYCLE_SLOT_PULSE_ALPHA_MAX - CYCLE_SLOT_PULSE_ALPHA_MIN) * wave
}

pub(crate) fn bump_scale(remaining_frac: f32) -> f32 {
	1.0 + (CYCLE_SLOT_BUMP_SCALE - 1.0) * remaining_frac.clamp(0.0, 1.0)
}

/// True when the chip is using the selected chrome (not the neutral rest state).
#[cfg(test)]
pub(crate) fn cycle_slot_is_highlighted(border: &BorderColor, outline: &Outline) -> bool {
	outline.color.alpha() > 0.2 && border_looks_selected(border)
}

#[cfg(test)]
fn border_looks_selected(border: &BorderColor) -> bool {
	let color = border.left;
	color.alpha() > 0.4 && color.to_srgba().red > 0.8 && color.to_srgba().green > 0.7
}

#[allow(clippy::too_many_arguments)]
fn apply_slot_style(
	selected: bool,
	elapsed: f32,
	bump_frac: f32,
	border: &mut BorderColor,
	outline: &mut Outline,
	fill: &mut BackgroundColor,
	transform: &mut UiTransform,
	text: &mut TextColor,
) {
	if selected {
		let pulse = pulse_alpha(elapsed);
		*border = BorderColor::all(CYCLE_SLOT_SELECTED_BORDER.with_alpha(pulse));
		outline.color = CYCLE_SLOT_SELECTED_OUTLINE;
		fill.0 = CYCLE_SLOT_SELECTED_FILL.with_alpha(
			CYCLE_SLOT_SELECTED_FILL.alpha()
				+ CYCLE_SLOT_SELECTED_GLOW.alpha() * (pulse - CYCLE_SLOT_PULSE_ALPHA_MIN)
					/ (CYCLE_SLOT_PULSE_ALPHA_MAX - CYCLE_SLOT_PULSE_ALPHA_MIN).max(0.001),
		);
		text.0 = TEXT_YELLOW;
		transform.scale = Vec2::splat(bump_scale(bump_frac));
	} else {
		*border = BorderColor::all(CYCLE_SLOT_NEUTRAL_BORDER);
		outline.color = Color::NONE;
		fill.0 = CYCLE_SLOT_NEUTRAL_FILL;
		text.0 = CYCLE_SLOT_NEUTRAL_TEXT;
		transform.scale = Vec2::ONE;
	}
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn sync_cycle_slots(
	time: Res<Time>,
	hud: Res<CombatHudVisible>,
	category: Option<Res<CombatHudInventoryCycleCategory>>,
	queue: Option<Res<CombatHudEquippedQueue>>,
	players: Query<&LubikanLedger, (With<Player>, With<LubikanOwner>)>,
	mut rows: Query<&mut CombatHudCycleSlotBump, With<CombatHudCycleSlotRow>>,
	mut slots: Query<(
		&mut CombatHudCycleSlot,
		&mut Node,
		&mut BorderColor,
		&mut Outline,
		&mut BackgroundColor,
		&mut UiTransform,
		&mut Text,
		&mut TextColor,
		&mut Visibility,
	)>,
) {
	let views = if hud.0 {
		cycle_slot_views(
			category.as_deref().copied().unwrap_or_default(),
			queue.as_deref(),
			players.iter().next(),
		)
	} else {
		Vec::new()
	};
	let elapsed = time.elapsed_secs();
	let selected_key = views.iter().find(|view| view.selected).map(|view| view.key);
	let mut bump_frac = 0.0;
	for mut bump in &mut rows {
		if selected_key != bump.key {
			bump.key = selected_key;
			bump.until = elapsed + CYCLE_SLOT_BUMP_SECS;
		}
		if bump.until > elapsed && CYCLE_SLOT_BUMP_SECS > 0.0 {
			bump_frac = ((bump.until - elapsed) / CYCLE_SLOT_BUMP_SECS).clamp(0.0, 1.0);
		}
	}
	for (
		mut slot,
		mut node,
		mut border,
		mut outline,
		mut fill,
		mut transform,
		mut text,
		mut text_color,
		mut visibility,
	) in &mut slots
	{
		let Some(view) = views.get(slot.index) else {
			slot.selected = false;
			*visibility = Visibility::Hidden;
			node.display = Display::None;
			node.min_width = Val::Px(0.0);
			apply_slot_style(
				false,
				elapsed,
				0.0,
				&mut border,
				&mut outline,
				&mut fill,
				&mut transform,
				&mut text_color,
			);
			continue;
		};
		slot.key = view.key;
		slot.selected = view.selected;
		if text.0 != view.label {
			text.0 = view.label.clone();
		}
		node.display = Display::Flex;
		node.min_width = Val::Px(CYCLE_SLOT_MIN_WIDTH_PX);
		*visibility = Visibility::Visible;
		let this_bump = if view.selected { bump_frac } else { 0.0 };
		apply_slot_style(
			view.selected,
			elapsed,
			this_bump,
			&mut border,
			&mut outline,
			&mut fill,
			&mut transform,
			&mut text_color,
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pulse_stays_in_the_tunable_band() {
		for step in 0..16 {
			let alpha = pulse_alpha(step as f32 * 0.07);
			assert!(
				(CYCLE_SLOT_PULSE_ALPHA_MIN..=CYCLE_SLOT_PULSE_ALPHA_MAX).contains(&alpha),
				"{alpha}"
			);
		}
	}

	#[test]
	fn equipped_queue_views_mark_exactly_one_selected() {
		let queue = CombatHudEquippedQueue {
			slots: vec![
				crate::CombatHudEquippedSlot { key: 2, label: "a".into() },
				crate::CombatHudEquippedSlot { key: 5, label: "b".into() },
				crate::CombatHudEquippedSlot { key: 9, label: "c".into() },
			],
			selected: 1,
		};
		let views = cycle_slot_views(CombatHudInventoryCycleCategory::Weapons, Some(&queue), None);
		assert_eq!(views.iter().filter(|view| view.selected).count(), 1);
		assert_eq!(views[1].key, 5);
		assert!(views[1].selected);
	}
}
