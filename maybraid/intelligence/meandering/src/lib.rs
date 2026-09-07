//! Nearby POI selection over retained local knowledge.

use bevy::prelude::*;
use poi_intelligence::{
	begin_poi_goal, choose_poi, KnownPoi, PoiGoal, PoiGoalCompleted, PoiGoalState,
	PoiIntelligenceUser, PoiKnowledge, PoiSource, PoiSystems, PoiVisitPolicy, PoiVisitState,
};

/// Chooses nearby destinations and delegates travel to [`PoiGoal`].
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct MeanderingIntelligenceUser {
	pub radius: f32,
	pub visit_policy: PoiVisitPolicy,
	pub selection_interval: f32,
	/// Seconds to remain at a reached POI before the goal completes.
	pub linger_secs: f32,
	/// Higher-order grant. When false, this brain does not start new POI goals.
	pub enabled: bool,
	/// Slot-derived tilt so pack-mates with the same table do not share a max.
	pub selection_salt: u64,
	next_selection_at: f32,
}

impl Default for MeanderingIntelligenceUser {
	fn default() -> Self {
		Self {
			radius: 200.0,
			visit_policy: PoiVisitPolicy::default(),
			selection_interval: 0.25,
			linger_secs: 4.0,
			enabled: true,
			selection_salt: 0,
			next_selection_at: 0.0,
		}
	}
}

impl MeanderingIntelligenceUser {
	pub fn new(radius: f32) -> Self {
		Self { radius: radius.max(0.0), ..default() }
	}

	pub fn salt_for_slot(slot: u16) -> u64 {
		u64::from(slot).wrapping_add(1).wrapping_mul(0x9E37_79B9)
	}
}

type MeanderingSelection<'a> = (
	Entity,
	&'a GlobalTransform,
	&'a mut MeanderingIntelligenceUser,
	&'a PoiIntelligenceUser,
	&'a mut PoiKnowledge,
	&'a mut PoiVisitState,
	Option<&'a mut PoiGoalState>,
);

/// Installs meandering selection after shared POI discovery.
pub struct MeanderingIntelligencePlugin;

impl Plugin for MeanderingIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(record_meandering_completions, select_meandering_goals)
				.chain()
				.in_set(PoiSystems::Select),
		);
	}
}

pub fn record_meandering_completions(
	time: Res<Time>,
	mut completed: MessageReader<PoiGoalCompleted>,
	mut users: Query<&mut PoiVisitState, With<MeanderingIntelligenceUser>>,
) {
	let now = time.elapsed_secs();
	for event in completed.read() {
		if let Ok(mut visits) = users.get_mut(event.user) {
			visits.complete(event.target, now);
		}
	}
}

pub fn select_meandering_goals(
	time: Res<Time>,
	mut users: Query<MeanderingSelection<'_>, Without<PoiGoal>>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	for (entity, transform, mut meandering, learner, mut knowledge, mut visits, mut state) in
		&mut users
	{
		if !meandering.enabled {
			continue;
		}
		if now < meandering.next_selection_at {
			continue;
		}
		meandering.next_selection_at = now + meandering.selection_interval.max(0.05);
		let at = transform.translation();
		let radius = meandering.radius.max(0.0);
		let all: Vec<_> = knowledge.matching(&learner.interests).copied().collect();
		if let PoiVisitPolicy::Cycle { roster_size, .. } = meandering.visit_policy {
			visits.reconcile_cycle(roster_size, |id| all.iter().any(|known| known.id == id));
		}
		let cycle_is_full = matches!(
			meandering.visit_policy,
			PoiVisitPolicy::Cycle { roster_size, .. }
				if roster_size > 0 && visits.cycle_roster().len() >= roster_size
		);
		let candidates: Vec<_> = if cycle_is_full {
			all
		} else {
			all.into_iter()
				.filter(|known| {
					visits.cycle_roster().contains(&known.id)
						|| xz_distance(at, known.position) <= radius + known.arrival_radius
				})
				.collect()
		};
		let candidates = not_already_there(at, candidates);
		if candidates.is_empty() {
			continue;
		}
		let salt = meandering.selection_salt;
		let Some(id) =
			choose_poi(&mut visits, meandering.visit_policy, &candidates, now, |known| {
				meandering_score(known, at, radius, &learner.interests, salt)
			})
		else {
			continue;
		};
		let Some(known) = knowledge.get(id).copied() else {
			continue;
		};
		knowledge.include_source(id, PoiSource::OBJECTIVE);
		begin_poi_goal(
			&mut commands,
			entity,
			known,
			now,
			meandering.linger_secs,
			state.as_deref_mut(),
			meandering.selection_salt,
		);
	}
}

fn meandering_score(
	known: KnownPoi,
	at: Vec3,
	radius: f32,
	interests: &poi_intelligence::PoiInterests,
	salt: u64,
) -> f32 {
	let interest = interests.weight(known.kind).unwrap_or(0.0);
	let normalized_distance = xz_distance(at, known.position) / radius.max(1.0);
	let base = interest * known.salience * known.confidence / (1.0 + normalized_distance);
	if salt == 0 {
		return base;
	}
	let mixed = poi_intelligence::mix_seed(salt ^ known.id.0);
	let fine = 0.92 + 0.16 * ((mixed >> 40) as f32 / (1_u32 << 24) as f32);
	let parity = if (known.id.0 ^ salt) & 1 == 0 { 1.15 } else { 0.85 };
	base * fine * parity
}

fn xz_distance(a: Vec3, b: Vec3) -> f32 {
	a.xz().distance(b.xz())
}

/// Skip POIs the mover is already standing in so completing one does not
/// immediately re-issue the same goal. If every known destination is here,
/// wait; discovery can still add another.
fn not_already_there(at: Vec3, candidates: Vec<KnownPoi>) -> Vec<KnownPoi> {
	candidates
		.into_iter()
		.filter(|known| xz_distance(at, known.position) > known.arrival_radius)
		.collect()
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use super::*;
	use poi_intelligence::{
		PoiId, PoiIntelligenceUser, PoiInterests, PoiKind, PoiKnowledge, PoiObservation,
		PoiVisitState,
	};

	#[test]
	fn score_prefers_nearer_equal_pois() -> anyhow::Result<()> {
		let kind = PoiKind::new("test/place");
		let interests = PoiInterests::one(kind);
		let known = |id, x| KnownPoi {
			id: PoiId(id),
			entity: None,
			kind,
			position: Vec3::X * x,
			arrival_radius: 1.0,
			salience: 1.0,
			confidence: 1.0,
			sources: PoiSource::LOCAL_SCAN,
			first_observed_at: 0.0,
			last_observed_at: 0.0,
		};
		assert!(
			meandering_score(known(1, 10.0), Vec3::ZERO, 200.0, &interests, 0)
				> meandering_score(known(2, 100.0), Vec3::ZERO, 200.0, &interests, 0)
		);
		Ok(())
	}

	#[test]
	fn skips_pois_the_mover_already_occupies() -> anyhow::Result<()> {
		let kind = PoiKind::new("test/place");
		let here = KnownPoi {
			id: PoiId(1),
			entity: None,
			kind,
			position: Vec3::ZERO,
			arrival_radius: 2.0,
			salience: 1.0,
			confidence: 1.0,
			sources: PoiSource::LOCAL_SCAN,
			first_observed_at: 0.0,
			last_observed_at: 0.0,
		};
		let away = KnownPoi { id: PoiId(2), position: Vec3::X * 10.0, ..here };
		assert!(not_already_there(Vec3::ZERO, vec![here]).is_empty());
		assert_eq!(not_already_there(Vec3::ZERO, vec![here, away]).len(), 1);
		assert_eq!(not_already_there(Vec3::ZERO, vec![here, away])[0].id, PoiId(2));
		Ok(())
	}

	#[test]
	fn slot_salt_splits_equal_meander_candidates() -> anyhow::Result<()> {
		let kind = PoiKind::new("test/place");
		let interests = PoiInterests::one(kind);
		let known = |id, x| KnownPoi {
			id: PoiId(id),
			entity: None,
			kind,
			position: Vec3::X * x,
			arrival_radius: 1.0,
			salience: 1.0,
			confidence: 1.0,
			sources: PoiSource::LOCAL_SCAN,
			first_observed_at: 0.0,
			last_observed_at: 0.0,
		};
		let near = known(4, 20.0);
		let other = known(7, 20.0);
		let policy = PoiVisitPolicy::default();
		let first = choose_poi(&mut PoiVisitState::default(), policy, &[near, other], 0.0, |poi| {
			meandering_score(
				poi,
				Vec3::ZERO,
				200.0,
				&interests,
				MeanderingIntelligenceUser::salt_for_slot(0),
			)
		});
		let second =
			choose_poi(&mut PoiVisitState::default(), policy, &[near, other], 0.0, |poi| {
				meandering_score(
					poi,
					Vec3::ZERO,
					200.0,
					&interests,
					MeanderingIntelligenceUser::salt_for_slot(1),
				)
			});
		assert_ne!(first, second);
		Ok(())
	}

	#[test]
	fn different_salts_write_different_live_points() -> anyhow::Result<()> {
		let kind = PoiKind::new("test/place");
		let interests = PoiInterests::one(kind);
		let knowledge = || {
			let mut knowledge = PoiKnowledge::default();
			let mut observation =
				PoiObservation::external(Entity::PLACEHOLDER, PoiId(4), kind, Vec3::X * 40.0);
			observation.arrival_radius = 8.0;
			knowledge.observe(observation, 0.0);
			knowledge
		};
		let mut world = World::new();
		world.init_resource::<Time>();
		let a = world
			.spawn((
				GlobalTransform::IDENTITY,
				MeanderingIntelligenceUser {
					selection_salt: MeanderingIntelligenceUser::salt_for_slot(0),
					linger_secs: 1.0,
					..MeanderingIntelligenceUser::new(200.0)
				},
				PoiIntelligenceUser::new(interests.clone()),
				knowledge(),
				PoiVisitState::default(),
			))
			.id();
		let b = world
			.spawn((
				GlobalTransform::IDENTITY,
				MeanderingIntelligenceUser {
					selection_salt: MeanderingIntelligenceUser::salt_for_slot(1),
					linger_secs: 1.0,
					..MeanderingIntelligenceUser::new(200.0)
				},
				PoiIntelligenceUser::new(interests),
				knowledge(),
				PoiVisitState::default(),
			))
			.id();
		assert!(world.run_system_once(select_meandering_goals).is_ok());
		let point_a = world.get::<PoiGoal>(a).map(|goal| goal.location.point);
		let point_b = world.get::<PoiGoal>(b).map(|goal| goal.location.point);
		assert!(point_a.is_some() && point_b.is_some());
		if let (Some(point_a), Some(point_b)) = (point_a, point_b) {
			assert_ne!(point_a.xz(), point_b.xz());
			assert_ne!(point_a.xz(), (Vec3::X * 40.0).xz());
		}
		Ok(())
	}
}
