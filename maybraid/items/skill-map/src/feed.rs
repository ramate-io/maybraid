//! A repeating, timed sequence driven by the same Laufer/Tanz flick messages.

use bevy::prelude::*;
use maybraid_character_controller::ActorControlStatus;
use maybraid_controllers_core::{ActorRoutingEvent, GestureEffect, RoutingKind};

use crate::user::{SkillMapHeld, SkillMapSteerLock, SkillMapUser};
use crate::SkillMapEnabled;

pub const FEED_STEP_SECS: f32 = 0.75;
pub const FEED_WINDOW_SECS: f32 = 0.22;
pub const FEED_PAUSE_SECS: f32 = 1.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedDirection {
	Left,
	Up,
	Right,
	Down,
}

impl FeedDirection {
	pub const fn vector(self) -> Vec2 {
		match self {
			Self::Left => Vec2::NEG_X,
			Self::Up => Vec2::Y,
			Self::Right => Vec2::X,
			Self::Down => Vec2::NEG_Y,
		}
	}

	pub const fn label(self) -> &'static str {
		match self {
			Self::Left => "LEFT",
			Self::Up => "UP",
			Self::Right => "RIGHT",
			Self::Down => "DOWN",
		}
	}

	fn matches(self, throw: Vec2) -> bool {
		// A 35-degree cone accepts natural stick imprecision without accepting diagonals.
		throw.is_finite()
			&& throw.length_squared() >= crate::controller::FLICK_MIN_PEAK.powi(2)
			&& self.vector().dot(throw.normalize_or_zero()) >= 0.819_152
	}
}

pub const FEED_STEPS: [FeedDirection; 8] = [
	FeedDirection::Left,
	FeedDirection::Up,
	FeedDirection::Right,
	FeedDirection::Down,
	FeedDirection::Left,
	FeedDirection::Right,
	FeedDirection::Up,
	FeedDirection::Down,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FeedStepStatus {
	#[default]
	Pending,
	Hit,
	Missed,
}

/// Kept on the map session; equipping another map drops this run entirely.
#[derive(Component, Clone, Debug, Default)]
pub struct FeedState {
	started: bool,
	elapsed: f32,
	cycle: u32,
	steps: [FeedStepStatus; FEED_STEPS.len()],
	frozen: bool,
	next_first_hit: bool,
}

impl FeedState {
	pub const fn step_time(index: usize) -> f32 {
		index as f32 * FEED_STEP_SECS
	}

	pub const fn sequence_end() -> f32 {
		Self::step_time(FEED_STEPS.len() - 1) + FEED_WINDOW_SECS
	}

	pub const fn cycle_duration() -> f32 {
		Self::sequence_end() + FEED_PAUSE_SECS
	}

	pub const fn started(&self) -> bool {
		self.started
	}

	/// Begin (or restart) the phrase on the overlay expand edge.
	pub fn activate(&mut self) {
		*self = Self { started: true, ..Self::default() };
	}

	pub const fn elapsed(&self) -> f32 {
		self.elapsed
	}

	pub const fn next_first_hit(&self) -> bool {
		self.next_first_hit
	}

	pub const fn cycle(&self) -> u32 {
		self.cycle
	}

	pub fn in_pause(&self) -> bool {
		self.started && self.elapsed > Self::sequence_end()
	}

	pub fn step_status(&self, index: usize) -> FeedStepStatus {
		self.steps[index]
	}

	pub fn hits(&self) -> usize {
		self.steps.iter().filter(|status| **status == FeedStepStatus::Hit).count()
	}

	pub fn advance(&mut self, delta: f32) {
		if !self.started || self.frozen || !delta.is_finite() || delta <= 0.0 {
			return;
		}
		self.elapsed += delta;
		if self.elapsed >= Self::cycle_duration() {
			let cycles = (self.elapsed / Self::cycle_duration()).floor() as u32;
			self.cycle = self.cycle.saturating_add(cycles);
			self.elapsed %= Self::cycle_duration();
			self.steps.fill(FeedStepStatus::Pending);
			if cycles == 1 && self.next_first_hit {
				self.steps[0] = FeedStepStatus::Hit;
			}
			self.next_first_hit = false;
		}
		for (index, status) in self.steps.iter_mut().enumerate() {
			if *status == FeedStepStatus::Pending
				&& self.elapsed > Self::step_time(index) + FEED_WINDOW_SECS
			{
				*status = FeedStepStatus::Missed;
			}
		}
	}

	/// Returns a hit only once per prompt. Bad timing/direction never advances the clock.
	pub fn strike(&mut self, throw: Vec2) -> bool {
		if self.frozen {
			return false;
		}
		if self.in_pause() {
			// The repeat's first prompt has the same early allowance as every
			// other prompt. Carry that hit across the upcoming cycle boundary.
			if Self::cycle_duration() - self.elapsed <= FEED_WINDOW_SECS
				&& !self.next_first_hit
				&& FEED_STEPS[0].matches(throw)
			{
				self.next_first_hit = true;
				return true;
			}
			return false;
		}
		if !self.started {
			return false;
		}
		for (index, direction) in FEED_STEPS.iter().enumerate() {
			if self.steps[index] == FeedStepStatus::Pending
				&& (self.elapsed - Self::step_time(index)).abs() <= FEED_WINDOW_SECS
				&& direction.matches(throw)
			{
				self.steps[index] = FeedStepStatus::Hit;
				return true;
			}
		}
		false
	}
}

/// Rising RB hold starts a fresh phrase. Release pauses via [`tick_feeds`];
/// the LB+RB chord never sets [`SkillMapHeld`], so it does not start the clock.
pub fn start_feeds_on_steer_hold(
	users: Query<(&SkillMapUser, &SkillMapHeld), Changed<SkillMapHeld>>,
	mut feeds: Query<&mut FeedState>,
) {
	for (user, held) in &users {
		if !held.0 {
			continue;
		}
		if let Ok(mut feed) = feeds.get_mut(user.maps) {
			feed.activate();
		}
	}
}

pub fn tick_feeds(
	time: Res<Time>,
	enabled: Res<SkillMapEnabled>,
	users: Query<(&SkillMapUser, &SkillMapHeld, &SkillMapSteerLock, Option<&ActorControlStatus>)>,
	mut feeds: Query<&mut FeedState>,
) {
	if !enabled.0 {
		return;
	}
	for (user, held, lock, status) in &users {
		if held.0 && !lock.locked() && status.is_none_or(|status| status.is_delivered()) {
			if let Ok(mut feed) = feeds.get_mut(user.maps) {
				feed.advance(time.delta_secs());
			}
		}
	}
}

/// Covers Tanz too: its gesture history is owned by the Tanz recognizer, not the
/// ordinary skill-map flick controller.
pub fn apply_feed_routing(
	event: On<ActorRoutingEvent>,
	users: Query<(&SkillMapUser, Option<&ActorControlStatus>)>,
	mut feeds: Query<&mut FeedState>,
) {
	let Ok((user, status)) = users.get(event.entity) else { return };
	if !event.status.assigned
		&& status
			.and_then(|status| status.source())
			.is_some_and(|source| source != event.status.source)
	{
		return;
	}
	let Ok(mut feed) = feeds.get_mut(user.maps) else { return };
	match event.event.gesture() {
		GestureEffect::Cancel => *feed = FeedState::default(),
		GestureEffect::Freeze => feed.frozen = true,
		GestureEffect::None => {
			if matches!(
				event.event.kind(),
				RoutingKind::Assigned | RoutingKind::Resumed | RoutingKind::Uncaptured
			) {
				feed.frozen = false;
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::feed::*;

	#[test]
	fn activate_starts_the_clock_without_a_flick() {
		let mut feed = FeedState::default();
		feed.advance(30.0);
		assert!(!feed.strike(Vec2::X));
		assert!(!feed.strike(Vec2::ZERO));
		assert!(!feed.started());
		assert_eq!(feed.elapsed(), 0.0);
		feed.activate();
		assert!(feed.started());
		assert_eq!(feed.elapsed(), 0.0);
		assert!(feed.strike(Vec2::NEG_X));
		assert_eq!(feed.hits(), 1);
		assert!(!feed.strike(Vec2::NEG_X));
		feed.advance(0.4);
		feed.activate();
		assert_eq!(feed.elapsed(), 0.0);
		assert_eq!(feed.hits(), 0);
		assert!(feed.started());
	}

	#[test]
	fn timing_and_direction_are_both_required() {
		let mut feed = FeedState::default();
		feed.activate();
		assert!(feed.strike(Vec2::NEG_X));
		feed.advance(FEED_STEP_SECS - FEED_WINDOW_SECS - 0.01);
		assert!(!feed.strike(Vec2::Y));
		feed.advance(0.02);
		assert!(!feed.strike(Vec2::NEG_Y));
		assert!(!feed.strike(Vec2::new(1.0, 1.0)));
		assert!(feed.strike(Vec2::new(0.15, 0.9)));
		feed.advance(FEED_STEP_SECS + FEED_WINDOW_SECS * 2.0);
		assert!(!feed.strike(Vec2::X));
		assert_eq!(feed.step_status(2), FeedStepStatus::Missed);
	}

	#[test]
	fn all_eight_hits_repeat_after_a_one_and_a_half_second_rest() {
		let mut feed = FeedState::default();
		feed.activate();
		for (index, direction) in FEED_STEPS.iter().enumerate() {
			if index > 0 {
				feed.advance(FEED_STEP_SECS);
			}
			assert!(feed.strike(direction.vector()));
		}
		assert_eq!(feed.hits(), 8);
		feed.advance(FEED_WINDOW_SECS + 0.01);
		assert!(feed.in_pause());
		assert!(!feed.strike(Vec2::NEG_X));
		feed.advance(FEED_PAUSE_SECS - 0.02);
		assert!(feed.in_pause());
		feed.advance(0.02);
		assert_eq!(feed.cycle(), 1);
		assert!(!feed.in_pause());
		assert_eq!(feed.hits(), 0);
		assert!(feed.strike(Vec2::NEG_X));
	}

	#[test]
	fn repeat_opening_accepts_early_input_without_double_claiming() {
		let mut feed = FeedState::default();
		feed.activate();
		assert!(feed.strike(Vec2::NEG_X));
		feed.advance(FeedState::cycle_duration() - FEED_WINDOW_SECS - 0.01);
		assert!(!feed.strike(Vec2::NEG_X));
		feed.advance(0.02);
		assert!(feed.strike(Vec2::NEG_X));
		assert!(!feed.strike(Vec2::NEG_X));
		feed.advance(FEED_WINDOW_SECS);
		assert_eq!(feed.cycle(), 1);
		assert_eq!(feed.step_status(0), FeedStepStatus::Hit);
		assert!(!feed.strike(Vec2::NEG_X));
	}

	#[test]
	fn disabled_unheld_and_unavailable_actors_freeze_the_clock() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<SkillMapEnabled>();
		world.resource_mut::<Time>().advance_by(std::time::Duration::from_millis(100));
		let mut feed = FeedState::default();
		feed.activate();
		assert!(feed.strike(Vec2::NEG_X));
		let session = world.spawn(feed).id();
		let user = world
			.spawn((
				SkillMapUser::mapping(session),
				SkillMapHeld(false),
				SkillMapSteerLock::default(),
			))
			.id();
		world
			.run_system_once(tick_feeds)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.entity_mut(user).insert(SkillMapHeld(true));
		world.resource_mut::<SkillMapEnabled>().0 = false;
		world
			.run_system_once(tick_feeds)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.resource_mut::<SkillMapEnabled>().0 = true;
		world.entity_mut(user).insert(ActorControlStatus::default());
		world
			.run_system_once(tick_feeds)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world
				.get::<FeedState>(session)
				.ok_or_else(|| anyhow::anyhow!("missing feed"))?
				.elapsed(),
			0.0
		);
		world.entity_mut(user).remove::<ActorControlStatus>();
		world
			.run_system_once(tick_feeds)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world
				.get::<FeedState>(session)
				.ok_or_else(|| anyhow::anyhow!("missing feed"))?
				.elapsed(),
			0.1
		);
		Ok(())
	}

	#[test]
	fn missed_notes_do_not_stall_and_frozen_time_does_not_advance() {
		let mut feed = FeedState::default();
		feed.activate();
		assert!(feed.strike(Vec2::NEG_X));
		feed.frozen = true;
		feed.advance(10.0);
		assert_eq!(feed.elapsed(), 0.0);
		feed.frozen = false;
		feed.advance(FEED_STEP_SECS * 2.0);
		assert_eq!(feed.step_status(1), FeedStepStatus::Missed);
		assert!(feed.strike(Vec2::X));
		feed.advance(FeedState::cycle_duration() * 3.0);
		assert_eq!(feed.cycle(), 3);
	}
}
