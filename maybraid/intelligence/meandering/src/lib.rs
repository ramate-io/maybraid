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
			next_selection_at: 0.0,
		}
	}
}

impl MeanderingIntelligenceUser {
	pub fn new(radius: f32) -> Self {
		Self { radius: radius.max(0.0), ..default() }
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
		let Some(id) =
			choose_poi(&mut visits, meandering.visit_policy, &candidates, now, |known| {
				meandering_score(known, at, radius, &learner.interests)
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
		);
	}
}

fn meandering_score(
	known: KnownPoi,
	at: Vec3,
	radius: f32,
	interests: &poi_intelligence::PoiInterests,
) -> f32 {
	let interest = interests.weight(known.kind).unwrap_or(0.0);
	let normalized_distance = xz_distance(at, known.position) / radius.max(1.0);
	interest * known.salience * known.confidence / (1.0 + normalized_distance)
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
	use super::*;
	use poi_intelligence::{PoiId, PoiInterests, PoiKind};

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
			meandering_score(known(1, 10.0), Vec3::ZERO, 200.0, &interests)
				> meandering_score(known(2, 100.0), Vec3::ZERO, 200.0, &interests)
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
}
