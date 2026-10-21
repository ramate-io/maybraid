//! Persistent player-arrival state. Messages announce transitions; this
//! resource survives generation that outlives message retention.

use bevy::prelude::*;
use maybraid_input::MenuNav;
use maybraid_spawns_core::{
	SpawnCancelReason, SpawnIdentity, SpawnOutcome, SpawnOwner, SpawnRequestId, SpawnSessionId,
};
use poi_intelligence::{
	NearbyFallback, NearbyQuery, PoiId, PoiInterest, PoiInterests, PoiKind, PoiRegistry,
};
use world_player::RespawnOrigin;

use crate::request::{ArrivalDestination, ArrivalReason, BeginPlayerArrival, DestinationSelected};

const STICK_ALIGN: f32 = 0.2;

/// Steps of one player arrival. Construction and activation are later phases.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArrivalPhase {
	#[default]
	ChoosingDestination,
	ResolvingPlacement,
	AwaitingSupport,
	SpawningActor,
	ActivatingActor,
}

impl ArrivalPhase {
	pub const fn is_choosing(self) -> bool {
		matches!(self, Self::ChoosingDestination)
	}

	pub const fn awaiting_support(self) -> bool {
		matches!(self, Self::AwaitingSupport)
	}
}

/// World-player downed duration, nearby POI scan, and replacement interests.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct WorldPlayerRespawnConfig {
	pub delay_secs: f32,
	pub poi_radius: f32,
	pub fallback: NearbyFallback,
	pub interests: PoiInterests,
}

impl Default for WorldPlayerRespawnConfig {
	fn default() -> Self {
		Self {
			delay_secs: 4.0,
			poi_radius: 320.0,
			fallback: NearbyFallback::new(60.0, 100.0),
			interests: default_player_respawn_interests(),
		}
	}
}

impl WorldPlayerRespawnConfig {
	/// Nearest POI outside the fallback ring's inner radius.
	pub fn nearby_query(&self) -> NearbyQuery {
		NearbyQuery::nearest_beyond(self.poi_radius, self.fallback.min_radius)
	}

	/// Cover the overhead view so labeled groves at the edge stay selectable.
	pub fn picker_query(&self, map_height: f32) -> NearbyQuery {
		let radius = (map_height * 1.2).max(self.poi_radius);
		NearbyQuery::nearest_beyond(radius, self.fallback.min_radius)
	}
}

/// One life waiting to be placed. Identity is required so a late result from
/// a previous life cannot confirm this one.
#[derive(Debug)]
pub struct PendingPlayerRespawn {
	pub request: SpawnRequestId,
	pub owner: SpawnOwner,
	pub phase: ArrivalPhase,
	pub reason: ArrivalReason,
	pub timer: Timer,
	pub death_at: Vec3,
	pub origin: RespawnOrigin,
	pub candidates: Vec<PoiId>,
	pub highlighted: Option<PoiId>,
	/// World XZ of [`Self::highlighted`]. Flick heading is measured from here.
	pub highlighted_at: Option<Vec2>,
	pub map_opened: bool,
	/// Stick is home; the next throw is one flick.
	pub stick_resting: bool,
	/// First Discovery life: no death glaze or bones.
	pub first_life: bool,
	/// Ring placement used when the registry has nothing selectable.
	pub fallback_at: Vec3,
	pub registry_revision: u64,
	/// Reused by candidate refresh while the respawn map is open.
	pub picker_nearby_scratch: Vec<PoiId>,
}

impl PendingPlayerRespawn {
	pub fn begin(
		identity: SpawnIdentity,
		reason: ArrivalReason,
		origin: RespawnOrigin,
		delay_secs: f32,
	) -> Self {
		let at = reason.origin_at();
		Self {
			request: identity.request,
			owner: identity.owner,
			phase: if reason.skips_picker() {
				ArrivalPhase::ResolvingPlacement
			} else {
				ArrivalPhase::ChoosingDestination
			},
			reason,
			timer: Timer::from_seconds(delay_secs.max(0.0), TimerMode::Once),
			death_at: at,
			origin,
			candidates: Vec::new(),
			highlighted: None,
			highlighted_at: None,
			map_opened: false,
			stick_resting: true,
			first_life: reason.first_life(),
			fallback_at: at,
			registry_revision: 0,
			picker_nearby_scratch: Vec::new(),
		}
	}

	pub fn test(death_at: Vec3, first_life: bool) -> Self {
		let reason = if first_life {
			ArrivalReason::FirstSpawn { at: death_at }
		} else {
			ArrivalReason::Death { at: death_at }
		};
		Self::begin(
			SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1))),
			reason,
			RespawnOrigin::began(None, false),
			0.0,
		)
		.opened()
	}

	fn opened(mut self) -> Self {
		self.map_opened = true;
		self
	}

	pub fn identity(&self) -> SpawnIdentity {
		SpawnIdentity::new(self.request, self.owner)
	}

	pub fn advance(&mut self, phase: ArrivalPhase) {
		self.phase = phase;
	}

	/// Highlight step / flick from UI-owned [`MenuNav`]. Returns whether Select confirmed.
	pub fn apply_menu_nav(&mut self, registry: &PoiRegistry, events: &[MenuNav]) -> bool {
		let mut confirm = false;
		for event in events {
			match *event {
				MenuNav::Select => confirm = true,
				MenuNav::Back => {}
				nav => {
					if let Some(dir) = menu_nav_dir(nav) {
						self.step_highlight(registry, dir);
					}
				}
			}
		}
		if self.highlighted.is_none() {
			if let Some(nearest) = nearest_candidate(&self.candidates, registry, self.death_at.xz())
			{
				self.set_highlighted(nearest, registry);
			}
		}
		confirm
	}

	pub fn step_highlight(&mut self, registry: &PoiRegistry, dir: Vec2) {
		if self.candidates.is_empty() {
			return;
		}
		let Some(from) = self.current_highlight_xz(registry) else {
			if let Some(nearest) = nearest_candidate(&self.candidates, registry, self.death_at.xz())
			{
				self.set_highlighted(nearest, registry);
			}
			return;
		};
		if let Some(next) =
			next_candidate_in_direction(&self.candidates, registry, from, self.highlighted, dir)
		{
			self.set_highlighted(next, registry);
		}
	}

	pub fn current_highlight_xz(&self, registry: &PoiRegistry) -> Option<Vec2> {
		self.highlighted_at.or_else(|| {
			self.highlighted
				.and_then(|id| registry.get(id))
				.map(|record| record.position.xz())
		})
	}

	pub fn set_highlighted(&mut self, id: PoiId, registry: &PoiRegistry) {
		self.highlighted = Some(id);
		self.highlighted_at = registry.get(id).map(|record| record.position.xz());
	}

	pub fn validate_highlight(&mut self, registry: &PoiRegistry) {
		let still_valid = self
			.highlighted
			.filter(|id| self.candidates.contains(id) && registry.get(*id).is_some());
		if let Some(id) = still_valid {
			self.highlighted_at = registry.get(id).map(|record| record.position.xz());
			return;
		}
		if let Some(nearest) = nearest_candidate(&self.candidates, registry, self.death_at.xz()) {
			self.set_highlighted(nearest, registry);
		} else {
			self.highlighted = None;
			self.highlighted_at = None;
		}
	}
}

/// Persistent arrivals for the current play session.
#[derive(Resource, Debug)]
pub struct WorldPlayerRespawnState {
	pub pending: Option<PendingPlayerRespawn>,
	pub last_poi: Option<PoiId>,
	/// This world load already queued or skipped the first-life picker.
	pub first_spawn_offered: bool,
	activated: Option<SpawnIdentity>,
	session: SpawnSessionId,
	next_request: SpawnRequestId,
}

impl Default for WorldPlayerRespawnState {
	fn default() -> Self {
		Self {
			pending: None,
			last_poi: None,
			first_spawn_offered: false,
			activated: None,
			session: SpawnSessionId::new(1),
			next_request: SpawnRequestId::new(1),
		}
	}
}

impl WorldPlayerRespawnState {
	pub fn with_pending(pending: PendingPlayerRespawn) -> Self {
		Self { pending: Some(pending), ..Self::default() }
	}

	pub fn session(&self) -> SpawnSessionId {
		self.session
	}

	pub fn owner(&self) -> SpawnOwner {
		SpawnOwner::new(self.session)
	}

	pub fn pending_identity(&self) -> Option<SpawnIdentity> {
		self.pending.as_ref().map(PendingPlayerRespawn::identity)
	}

	/// Begin a new arrival. A previous pending request is superseded.
	pub fn begin_arrival(
		&mut self,
		reason: ArrivalReason,
		origin: RespawnOrigin,
		delay_secs: f32,
	) -> SpawnIdentity {
		let identity = SpawnIdentity::new(self.next_request, self.owner());
		self.next_request = self.next_request.next();
		self.pending = Some(PendingPlayerRespawn::begin(identity, reason, origin, delay_secs));
		identity
	}

	pub fn begin_from(
		&mut self,
		request: BeginPlayerArrival,
		delay_secs: f32,
	) -> Option<SpawnIdentity> {
		if !request.owner.owns(self.session) {
			return None;
		}
		Some(self.begin_arrival(request.reason, request.origin, delay_secs))
	}

	pub fn accepts_identity(&self, identity: SpawnIdentity) -> bool {
		self.pending_identity().is_some_and(|pending| pending.matches(identity))
			&& identity.owner.owns(self.session)
	}

	pub fn last_activated(&self) -> Option<SpawnIdentity> {
		self.activated
	}

	/// Finish a matching arrival exactly once. Returns false for a stale or
	/// already-activated identity so a late `ActorSpawned` cannot take over.
	pub fn complete_arrival(&mut self, identity: SpawnIdentity) -> bool {
		if !self.accepts_identity(identity) {
			return false;
		}
		if self.activated.is_some_and(|activated| activated.matches(identity)) {
			return false;
		}
		self.activated = Some(identity);
		self.pending = None;
		true
	}

	pub fn accept_destination(
		&mut self,
		selected: DestinationSelected,
	) -> Option<ArrivalDestination> {
		if !self.accepts_identity(selected.identity) {
			return None;
		}
		if let Some(pending) = self.pending.as_mut() {
			pending.advance(ArrivalPhase::ResolvingPlacement);
		}
		Some(selected.destination)
	}

	pub fn cancel_pending(&mut self, reason: SpawnCancelReason) -> Option<SpawnOutcome> {
		let pending = self.pending.take()?;
		Some(SpawnOutcome::cancelled(pending.identity(), reason))
	}

	/// Clear per-session spawn latches so the next world load can pick again.
	/// Bumps the session so a late result cannot activate the new life.
	pub fn end_session(&mut self) {
		self.session = self.session.next();
		self.first_spawn_offered = false;
		self.last_poi = None;
		self.pending = None;
		self.activated = None;
	}
}

/// Clear per-session spawn latches so the next world load (or character) can pick again.
pub fn reset_first_spawn_offer(mut state: ResMut<WorldPlayerRespawnState>) {
	state.end_session();
}

/// North-up map looks down with world +Z as screen up, so screen-right is world −X.
pub fn menu_nav_dir(nav: MenuNav) -> Option<Vec2> {
	match nav {
		MenuNav::Up => Some(map_flick_dir(Vec2::Y)),
		MenuNav::Down => Some(map_flick_dir(Vec2::NEG_Y)),
		MenuNav::Left => Some(map_flick_dir(Vec2::NEG_X)),
		MenuNav::Right => Some(map_flick_dir(Vec2::X)),
		MenuNav::Select | MenuNav::Back => None,
	}
}

/// North-up map looks down with world +Z as screen up, so screen-right is world −X.
pub fn map_flick_dir(stick: Vec2) -> Vec2 {
	Vec2::new(-stick.x, stick.y)
}

pub fn next_candidate_in_direction(
	candidates: &[PoiId],
	registry: &PoiRegistry,
	from: Vec2,
	current: Option<PoiId>,
	dir: Vec2,
) -> Option<PoiId> {
	let points: Vec<_> = candidates
		.iter()
		.filter_map(|id| registry.get(*id).map(|record| (*id, record.position.xz())))
		.collect();
	nearest_in_direction(from, current, dir, &points)
}

pub fn nearest_in_direction(
	from: Vec2,
	current: Option<PoiId>,
	dir: Vec2,
	points: &[(PoiId, Vec2)],
) -> Option<PoiId> {
	let dir = dir.normalize_or_zero();
	if dir == Vec2::ZERO {
		return None;
	}
	points
		.iter()
		.filter_map(|(id, xz)| {
			if Some(*id) == current {
				return None;
			}
			let delta = *xz - from;
			let dist = delta.length();
			if dist < 0.5 {
				return None;
			}
			let align = (delta / dist).dot(dir);
			if align < STICK_ALIGN {
				return None;
			}
			// Prefer a well-aimed far mark over a closer POI that is only loosely
			// in the cone, so a flick toward an on-screen grove can reach it.
			let score = dist / (align * align);
			Some((*id, score, dist, align))
		})
		.min_by(|a, b| {
			a.1.total_cmp(&b.1)
				.then_with(|| a.2.total_cmp(&b.2))
				.then_with(|| b.3.total_cmp(&a.3))
				.then_with(|| a.0.cmp(&b.0))
		})
		.map(|(id, _, _, _)| id)
}

fn default_player_respawn_interests() -> PoiInterests {
	PoiInterests::new([
		PoiInterest::new(PoiKind::new("mobs/saloon"), 1.7),
		PoiInterest::new(PoiKind::new("mobs/local"), 1.6),
		PoiInterest::new(PoiKind::new("mobs/urban"), 1.55),
		PoiInterest::new(PoiKind::new("mobs/vegetation"), 0.4),
	])
}

pub fn nearest_candidate(
	candidates: &[PoiId],
	registry: &PoiRegistry,
	focus: Vec2,
) -> Option<PoiId> {
	candidates
		.iter()
		.filter_map(|id| registry.get(*id).copied())
		.min_by(|a, b| {
			a.position
				.xz()
				.distance(focus)
				.total_cmp(&b.position.xz().distance(focus))
				.then_with(|| a.id.cmp(&b.id))
		})
		.map(|record| record.id)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ending_the_session_rejects_the_previous_arrival() {
		let mut state = WorldPlayerRespawnState::default();
		let identity = state.begin_arrival(
			ArrivalReason::Death { at: Vec3::ZERO },
			RespawnOrigin::began(None, false),
			0.0,
		);
		state.end_session();
		assert!(!state.accepts_identity(identity));
		assert!(state.pending.is_none());
		assert_ne!(state.session(), identity.owner.session);
	}

	#[test]
	fn a_stale_destination_does_not_advance_a_new_arrival() {
		let mut state = WorldPlayerRespawnState::default();
		let first = state.begin_arrival(
			ArrivalReason::FirstSpawn { at: Vec3::ZERO },
			RespawnOrigin::began(None, false),
			0.0,
		);
		state.end_session();
		let _next = state.begin_arrival(
			ArrivalReason::Death { at: Vec3::X },
			RespawnOrigin::began(None, false),
			0.0,
		);
		assert!(state
			.accept_destination(DestinationSelected::new(first, ArrivalDestination::Fallback))
			.is_none());
		assert!(state.pending.as_ref().is_some_and(|pending| pending.phase.is_choosing()));
	}
}
