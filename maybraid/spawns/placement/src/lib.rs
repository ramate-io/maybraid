//! Destination → floor / clearance / support. GenerationScheme is not
//! involved in collider readiness.

pub mod pending;
pub mod poi;
pub mod probe;
pub mod request;
pub mod support;
pub mod terrain;

pub use pending::{PendingPlacement, PlacementTick, PlacementWork};
pub use probe::{observe_placement_physics, register_placement_physics};
pub use request::{
	ActorClearance, CancelPlacement, PlacementFailed, PlacementPolicy, PlacementReady,
	PlacementSupportHit, ResolvePlacement, SupportPolicy,
};
pub use support::{column_presentation_ready, column_volume, PlacementSupport, SupportReadiness};
pub use terrain::{
	PlacementPose, PlacementSearch, INTERIOR_LOWER_MARGIN, INTERIOR_UPPER_MARGIN,
	OUTDOOR_LOWER_MARGIN, OUTDOOR_UPPER_MARGIN,
};

/// Previous interior tolerance; the unified probe uses [`INTERIOR_UPPER_MARGIN`].
pub const INTERIOR_FLOOR_TOLERANCE: f32 = INTERIOR_UPPER_MARGIN;

use bevy::prelude::*;
use durham::DurhamSurface;
use maybraid_spawns_player::ArrivalDestination;
use poi_intelligence::{PoiId, PoiRegistry};

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlacementSystems {
	Observe,
	Poll,
}

#[derive(Resource, Default)]
struct PlacementRuntime;

/// Headless-safe placement messages and pending work.
pub fn register_placement_runtime(app: &mut App) {
	if app.world().contains_resource::<PlacementRuntime>() {
		return;
	}
	app.init_resource::<PlacementRuntime>()
		.init_resource::<PlacementWork>()
		.init_resource::<PlacementSupport>()
		.add_message::<ResolvePlacement>()
		.add_message::<PlacementReady>()
		.add_message::<PlacementFailed>()
		.add_message::<CancelPlacement>()
		.configure_sets(Update, PlacementSystems::Observe.before(PlacementSystems::Poll))
		.add_systems(
			Update,
			(cancel_placements, poll_placement_support)
				.chain()
				.in_set(PlacementSystems::Poll),
		);
}

pub struct PlacementPlugin;

impl Plugin for PlacementPlugin {
	fn build(&self, app: &mut App) {
		register_placement_runtime(app);
	}
}

impl ResolvePlacement {
	/// Destination + policy → pending search column, or an explicit failure.
	/// Unresolved POIs do not become a silent floor at a stale coordinate.
	pub fn realize(
		self,
		registry: &PoiRegistry,
		surface: &DurhamSurface,
	) -> Result<PendingPlacement, PlacementFailed> {
		match PlacementSearch::resolve(
			self.destination,
			registry,
			surface,
			self.fallback_at,
			self.hull,
		) {
			Ok(search) => {
				if !search.bounds_valid() {
					return Err(PlacementFailed::new(
						self.identity,
						self.attempt,
						maybraid_spawns_player::SpawnFailureReason::InvalidBounds,
					));
				}
				Ok(PendingPlacement::new(self.identity, self.attempt, self.destination, search))
			}
			Err(reason) => Err(PlacementFailed::new(self.identity, self.attempt, reason)),
		}
	}
}

impl PlacementSearch {
	/// Resolve a picker / resume destination into a bounded column.
	/// Generated heights define the interval; the probe reads colliders.
	pub fn resolve(
		destination: ArrivalDestination,
		registry: &PoiRegistry,
		surface: &DurhamSurface,
		fallback_at: Vec3,
		hull: characters::LocomotionCapsule,
	) -> Result<Self, maybraid_spawns_player::SpawnFailureReason> {
		match destination {
			ArrivalDestination::Poi(id) => Self::resolve_poi(id, registry, hull),
			ArrivalDestination::Fallback => Ok(Self::outdoor_at(surface, fallback_at.xz(), hull)),
			ArrivalDestination::Resume(at) | ArrivalDestination::DeathInPlace(at) => {
				Ok(Self::outdoor_at(surface, at.xz(), hull))
			}
		}
	}

	pub fn resolve_poi(
		id: PoiId,
		registry: &PoiRegistry,
		hull: characters::LocomotionCapsule,
	) -> Result<Self, maybraid_spawns_player::SpawnFailureReason> {
		let Some(record) = registry.get(id) else {
			return Err(maybraid_spawns_player::SpawnFailureReason::DestinationUnresolved);
		};
		Ok(match PlacementPolicy::for_poi(record) {
			PlacementPolicy::InteriorFloor => Self::interior_at(record.position, hull),
			// `PoiRecord.position.y` is generated from the presented support
			// surface. Use it as the search hint so a padded ground below or
			// above raw Durham remains the intended placement band; collider
			// validation still decides whether support is ready.
			PlacementPolicy::OutdoorGround => {
				Self::outdoor_hint(record.position.xz(), record.position.y, hull)
			}
			PlacementPolicy::Airborne => {
				Self::airborne_at(record.position + Vec3::Y * hull.spawn_height(), hull)
			}
		})
	}
}

fn cancel_placements(mut work: ResMut<PlacementWork>, mut cancels: MessageReader<CancelPlacement>) {
	for cancel in cancels.read() {
		let _ = work.cancel(cancel.identity);
	}
}

fn poll_placement_support(
	mut work: ResMut<PlacementWork>,
	mut ready: MessageWriter<PlacementReady>,
	mut failed: MessageWriter<PlacementFailed>,
) {
	for tick in work.poll_observed() {
		match tick {
			PlacementTick::Ready(outcome) => {
				ready.write(outcome);
			}
			PlacementTick::Failed(outcome) => {
				failed.write(outcome);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use characters::LocomotionCapsule;
	use durham::{BaseTerrainNoise, TerrainCellLayout, TerrainConfig, WorldBaseTerrain};
	use lod::hcsg::HcsgStorage;
	use maybraid_spawns_player::{
		PlacementAttempt, SpawnFailureReason, SpawnIdentity, SpawnOwner, SpawnRequestId,
		SpawnSessionId,
	};
	use poi_intelligence::{Poi, PoiId, PoiKind};

	fn identity() -> SpawnIdentity {
		SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1)))
	}

	fn attempt() -> PlacementAttempt {
		PlacementAttempt::new(1)
	}

	fn surface_world() -> World {
		let mut world = World::new();
		world.init_resource::<HcsgStorage>();
		world.init_resource::<TerrainCellLayout>();
		world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
			&TerrainConfig::new(42),
		)));
		world.init_resource::<PoiRegistry>();
		world
	}

	#[test]
	fn register_placement_runtime_is_idempotent() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_placement_runtime(&mut app);
		register_placement_runtime(&mut app);
		assert!(app.world().contains_resource::<PlacementWork>());
	}

	#[test]
	fn an_unresolved_poi_fails_instead_of_using_a_coordinate() -> anyhow::Result<()> {
		let mut world = surface_world();
		world
			.run_system_once(
				|registry: Res<PoiRegistry>, surface: DurhamSurface| -> anyhow::Result<()> {
					let request = ResolvePlacement::humanoid(
						identity(),
						attempt(),
						ArrivalDestination::Poi(PoiId(99)),
						Vec3::new(3.0, 4.0, 5.0),
					);
					let failed = request.realize(&registry, &surface).expect_err("unresolved");
					assert_eq!(failed.reason, SpawnFailureReason::DestinationUnresolved);
					assert!(failed.belongs_to(identity()));
					Ok(())
				},
			)
			.map_err(|error| anyhow::anyhow!("{error:?}"))??;
		Ok(())
	}

	#[test]
	fn resolve_uses_the_request_hull() -> anyhow::Result<()> {
		let mut world = surface_world();
		world
			.run_system_once(
				|registry: Res<PoiRegistry>, surface: DurhamSurface| -> anyhow::Result<()> {
					let hull = LocomotionCapsule::HUMANOID.with_half_height(1.6);
					let request = ResolvePlacement::new(
						identity(),
						attempt(),
						ArrivalDestination::Fallback,
						hull,
						Vec3::new(3.0, 4.0, 5.0),
					);
					let pending = request.realize(&registry, &surface).expect("fallback");
					assert_eq!(pending.hull, hull);
					assert_eq!(pending.search.hull, hull);
					assert!(pending.search.bounds_valid());
					assert!(pending.accepted.is_none(), "collider geometry accepts the pose");
					Ok(())
				},
			)
			.map_err(|error| anyhow::anyhow!("{error:?}"))??;
		Ok(())
	}

	#[test]
	fn poi_resolution_uses_authored_floor_and_generated_terrain_heights() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<PoiRegistry>();
		world
			.run_system_once(|mut registry: ResMut<PoiRegistry>| -> anyhow::Result<()> {
				let kind = PoiKind::new("same/poi-kind");
				registry.upsert(
					Entity::from_bits(1),
					Poi::new(PoiId(1), kind).with_floor_support(),
					Vec3::new(8.0, 24.0, 6.0),
					true,
					false,
				)?;
				registry.upsert(
					Entity::from_bits(2),
					Poi::new(PoiId(2), kind),
					Vec3::new(8.0, 24.0, 6.0),
					true,
					false,
				)?;
				registry.upsert(
					Entity::from_bits(3),
					Poi::new(PoiId(3), kind),
					Vec3::new(8.0, -24.0, 6.0),
					true,
					false,
				)?;
				let floor =
					PlacementSearch::resolve_poi(PoiId(1), &registry, LocomotionCapsule::HUMANOID)
						.map_err(|reason| anyhow::anyhow!("floor placement failed: {reason:?}"))?;
				let terrain =
					PlacementSearch::resolve_poi(PoiId(2), &registry, LocomotionCapsule::HUMANOID)
						.map_err(|reason| {
							anyhow::anyhow!("terrain placement failed: {reason:?}")
						})?;
				assert_eq!(floor.policy, PlacementPolicy::InteriorFloor);
				assert_eq!(floor.min_surface_y, 24.0 - INTERIOR_LOWER_MARGIN);
				assert_eq!(terrain.policy, PlacementPolicy::OutdoorGround);
				assert_eq!(terrain.min_surface_y, 24.0 - OUTDOOR_LOWER_MARGIN);
				let below =
					PlacementSearch::resolve_poi(PoiId(3), &registry, LocomotionCapsule::HUMANOID)
						.map_err(|reason| anyhow::anyhow!("{reason:?}"))?;
				assert_eq!(below.min_surface_y, -24.0 - OUTDOOR_LOWER_MARGIN);
				Ok(())
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))??;
		Ok(())
	}

	#[test]
	fn inverted_bounds_are_an_explicit_failure() {
		let search = PlacementSearch::new(
			Vec2::ZERO,
			4.0,
			1.0,
			LocomotionCapsule::HUMANOID,
			PlacementPolicy::OutdoorGround,
		);
		assert!(!search.bounds_valid());
	}
}
