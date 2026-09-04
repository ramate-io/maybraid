//! Hide actuator: move to a nearby low-vantage, low-occupancy pocket.

mod candidate;

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use evasion_intelligence::{EvasionIntelligenceUser, EvasionSystems};
use lod_avian::PhysicsInteractionLayer;
use movement_intelligence::{
	MovementIntelligence, MovementIntelligenceSystems, MovementLocation, MovementObjective,
	ReplanMovement,
};
use spotting_intelligence::SpotSubject;
use spotting_intelligence_avian::clear_segment;

pub use candidate::{occupancy_at, pick_hide, HideCandidate, HideOccupant};

const REFRESH_DISTANCE: f32 = 0.8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HidingSettings {
	pub horizon: f32,
	pub occupancy_radius: f32,
	pub azimuths: u32,
	pub standoffs: [f32; 2],
}

impl Default for HidingSettings {
	fn default() -> Self {
		Self { horizon: 14.0, occupancy_radius: 2.4, azimuths: 8, standoffs: [4.0, 8.0] }
	}
}

/// Claimed hide pocket so other hiders can treat it as occupied.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HideClaim {
	pub point: Vec3,
}

/// Per-user hide policy. Writes [`MovementObjective::Reach`] only while the
/// evasion signal is hide.
#[derive(Component, Clone, Debug)]
pub struct HidingUser {
	pub settings: HidingSettings,
	driving: bool,
}

impl HidingUser {
	pub fn new(settings: HidingSettings) -> Self {
		Self { settings, driving: false }
	}

	pub fn samples(&self, from: Vec3) -> Vec<Vec3> {
		let mut points = Vec::new();
		for radius in self.settings.standoffs {
			if radius > self.settings.horizon {
				continue;
			}
			points.extend(
				MovementLocation::ring_around(from, from.y, radius, self.settings.azimuths, 0.4)
					.into_iter()
					.map(|location| location.point),
			);
		}
		points
	}
}

impl Default for HidingUser {
	fn default() -> Self {
		Self::new(HidingSettings::default())
	}
}

pub struct HidingPlugin;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HidingSystems {
	Write,
}

impl Plugin for HidingPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			HidingSystems::Write
				.after(EvasionSystems::Rank)
				.before(MovementIntelligenceSystems::Replan),
		)
		.add_systems(Update, write_hide_objectives.in_set(HidingSystems::Write));
	}
}

pub fn write_hide_objectives(
	spatial: SpatialQuery,
	mut users: Query<(
		Entity,
		&Transform,
		&EvasionIntelligenceUser,
		&mut HidingUser,
		&mut MovementIntelligence,
	)>,
	subjects: Query<(Entity, &Transform, Option<&HideClaim>), With<SpotSubject>>,
	mut commands: Commands,
) {
	let occupants: Vec<HideOccupant> = subjects
		.iter()
		.flat_map(|(entity, transform, claim)| {
			let body = HideOccupant { entity, point: transform.translation };
			claim.map_or_else(
				|| vec![body],
				|claim| vec![body, HideOccupant { entity, point: claim.point }],
			)
		})
		.collect();
	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);

	for (entity, transform, evasion, mut hiding, mut movement) in &mut users {
		if !evasion.signal.is_hide() {
			let was_driving = hiding.driving;
			hiding.driving = false;
			commands.entity(entity).remove::<HideClaim>();
			if was_driving && evasion.signal.is_idle() {
				hold_in_place(entity, transform.translation, &mut movement, &mut commands);
			}
			continue;
		}
		let Some(contact) = evasion.best_contact() else {
			if hiding.driving {
				hiding.driving = false;
				commands.entity(entity).remove::<HideClaim>();
				hold_in_place(entity, transform.translation, &mut movement, &mut commands);
			}
			continue;
		};
		let from = transform.translation;
		let threat = contact.position;
		let samples = hiding.samples(from);
		let Some(point) = pick_hide(
			from,
			threat,
			&samples,
			entity,
			&occupants,
			hiding.settings.occupancy_radius,
			|candidate| !clear_segment(threat, candidate, &spatial, &filter),
		) else {
			continue;
		};
		let next = MovementObjective::Reach(MovementLocation::new(
			point,
			movement.ability.agent_radius.max(0.4),
		));
		hiding.driving = true;
		commands.entity(entity).insert(HideClaim { point });
		if !should_replan(movement.objective, next) {
			continue;
		}
		movement.objective = next;
		commands.entity(entity).insert(ReplanMovement);
	}
}

fn hold_in_place(
	entity: Entity,
	at: Vec3,
	movement: &mut MovementIntelligence,
	commands: &mut Commands,
) {
	movement.objective =
		MovementObjective::Reach(MovementLocation::new(at, movement.ability.agent_radius));
	movement.adopt_plan(Vec::new());
	commands.entity(entity).remove::<ReplanMovement>();
}

fn should_replan(current: MovementObjective, next: MovementObjective) -> bool {
	if std::mem::discriminant(&current) != std::mem::discriminant(&next) {
		return true;
	}
	let a = current.location().point;
	let b = next.location().point;
	Vec2::new(a.x, a.z).distance(Vec2::new(b.x, b.z)) >= REFRESH_DISTANCE
		|| (a.y - b.y).abs() >= REFRESH_DISTANCE
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn samples_stay_inside_the_horizon() -> anyhow::Result<()> {
		let hiding = HidingUser::new(HidingSettings {
			horizon: 6.0,
			occupancy_radius: 2.0,
			azimuths: 4,
			standoffs: [4.0, 8.0],
		});
		let samples = hiding.samples(Vec3::ZERO);
		assert_eq!(samples.len(), 4);
		assert!(samples.iter().all(|point| point.length() <= 6.0 + 1e-3));
		Ok(())
	}
}
