//! Top-right running score: the hit-marker points, downs, deaths, and streak.

use bevy::prelude::*;
use damage::{DamageApplied, HeadshotBand};
use menu_components::{spawn_hud_text_card, HudFonts, HUD_TEXT_CARD_FACE_PX, TEXT_YELLOW};
use player::Player;

use crate::vitals::{
	PLATE_BORDER, PLATE_FILL, PLATE_PAD_X, PLATE_PAD_Y, PLATE_RADIUS, PLATE_STROKE, VITALS_INSET,
};
use crate::{hit_points, CombatHudVisible, DOWN_POINTS};

const SCORE_WIDTH: f32 = 168.0;
const SCORE_LINE_PX: f32 = 13.0;
const SCORE_LINE: Color = Color::srgb(0.95, 0.96, 0.98);

/// The player's running tally, scored as the hit markers award it. A mode
/// keeps score by inserting this; inserting a fresh one resets it, and
/// removing it hides the panel.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CombatScore {
	pub points: u32,
	pub downs: u32,
	pub deaths: u32,
	/// Downs since the player last died.
	pub streak: u32,
	pub best_streak: u32,
}

impl CombatScore {
	pub fn record_hit(&mut self, points: u8) {
		self.points = self.points.saturating_add(u32::from(points));
	}

	pub fn record_down(&mut self) {
		self.record_hit(DOWN_POINTS);
		self.downs = self.downs.saturating_add(1);
		self.streak = self.streak.saturating_add(1);
		self.best_streak = self.best_streak.max(self.streak);
	}

	pub fn record_death(&mut self) {
		self.deaths = self.deaths.saturating_add(1);
		self.streak = 0;
	}

	fn headline(&self) -> String {
		format!("Score  {}", self.points)
	}

	fn tally(&self) -> String {
		format!("Downs  {}    Deaths  {}", self.downs, self.deaths)
	}

	fn streaks(&self) -> String {
		format!("Streak  {}    Best  {}", self.streak, self.best_streak)
	}
}

#[derive(Component)]
pub(crate) struct CombatScoreRoot;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatScoreLine {
	Headline,
	Tally,
	Streaks,
}

impl CombatScoreLine {
	fn text(self, score: &CombatScore) -> String {
		match self {
			Self::Headline => score.headline(),
			Self::Tally => score.tally(),
			Self::Streaks => score.streaks(),
		}
	}
}

pub(crate) fn spawn_combat_score(parent: &mut ChildSpawnerCommands, fonts: &HudFonts) {
	parent
		.spawn((
			Name::new("combat-score"),
			CombatScoreRoot,
			Node {
				position_type: PositionType::Absolute,
				right: Val::Px(VITALS_INSET),
				top: Val::Px(VITALS_INSET),
				width: Val::Px(SCORE_WIDTH),
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::Stretch,
				row_gap: Val::Px(4.0),
				padding: UiRect::axes(Val::Px(PLATE_PAD_X), Val::Px(PLATE_PAD_Y)),
				border: UiRect::all(Val::Px(PLATE_BORDER)),
				border_radius: BorderRadius::all(Val::Px(PLATE_RADIUS)),
				..default()
			},
			BackgroundColor(PLATE_FILL),
			BorderColor::all(PLATE_STROKE),
			Visibility::Hidden,
			Pickable::IGNORE,
		))
		.with_children(|plate| {
			let score = CombatScore::default();
			spawn_hud_text_card(plate, (), |card| {
				card.spawn((
					CombatScoreLine::Headline,
					Text::new(score.headline()),
					fonts.item(HUD_TEXT_CARD_FACE_PX),
					TextColor(TEXT_YELLOW),
					Pickable::IGNORE,
				));
			});
			for line in [CombatScoreLine::Tally, CombatScoreLine::Streaks] {
				plate.spawn((
					line,
					Text::new(line.text(&score)),
					fonts.body(SCORE_LINE_PX),
					TextColor(SCORE_LINE),
					Pickable::IGNORE,
				));
			}
		});
}

/// Runs between damage apply and down, while a downed player still carries
/// [`Player`].
pub(crate) fn ingest_combat_score(
	score: Option<ResMut<CombatScore>>,
	mut hits: MessageReader<DamageApplied>,
	players: Query<Entity, With<Player>>,
	targets: Query<(&GlobalTransform, Option<&HeadshotBand>)>,
) {
	let (Some(mut score), Ok(player)) = (score, players.single()) else {
		hits.clear();
		return;
	};
	for hit in hits.read() {
		if hit.target == player {
			if hit.remaining <= 0.0 {
				score.record_death();
			}
			continue;
		}
		if hit.source != Some(player) {
			continue;
		}
		score.record_hit(hit_points(targets.get(hit.target).ok(), hit.point));
		if hit.remaining <= 0.0 {
			score.record_down();
		}
	}
}

pub(crate) fn sync_combat_score(
	hud: Res<CombatHudVisible>,
	score: Option<Res<CombatScore>>,
	mut roots: Query<&mut Visibility, With<CombatScoreRoot>>,
	mut lines: Query<(&CombatScoreLine, &mut Text)>,
) {
	let shown = hud.0 && score.is_some();
	for mut visibility in &mut roots {
		let wanted = if shown { Visibility::Visible } else { Visibility::Hidden };
		if *visibility != wanted {
			*visibility = wanted;
		}
	}
	let Some(score) = score.filter(|_| shown) else {
		return;
	};
	for (line, mut text) in &mut lines {
		let wanted = line.text(&score);
		if text.0 != wanted {
			text.0 = wanted;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{HEAD_POINTS, HIT_POINTS};

	#[test]
	fn downs_build_a_streak_that_a_death_ends() {
		let mut score = CombatScore::default();
		score.record_hit(HIT_POINTS);
		score.record_hit(HEAD_POINTS);
		score.record_down();
		score.record_down();
		score.record_death();
		score.record_down();
		assert_eq!(score.points, u32::from(HIT_POINTS + HEAD_POINTS + 3 * DOWN_POINTS));
		assert_eq!((score.downs, score.deaths), (3, 1));
		assert_eq!((score.streak, score.best_streak), (1, 2));
		assert_eq!(score.headline(), "Score  18");
		assert_eq!(score.tally(), "Downs  3    Deaths  1");
		assert_eq!(score.streaks(), "Streak  1    Best  2");
	}
}
