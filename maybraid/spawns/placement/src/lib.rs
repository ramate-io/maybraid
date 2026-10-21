//! Destination → floor / clearance / support. GenerationScheme is not
//! involved in collider readiness.

pub mod pending;
pub mod poi;
pub mod probe;
pub mod request;
pub mod support;
pub mod terrain;

pub use pending::{PendingPlacement, PlacementTick, PlacementWork};
pub use poi::interior_building_poi;
pub use probe::{observe_placement_physics, register_placement_physics, INTERIOR_FLOOR_TOLERANCE};
pub use request::{
	ActorClearance, CancelPlacement, PlacementFailed, PlacementPolicy, PlacementReady,
	ResolvePlacement,
};
pub use support::{PlacementSupport, SupportReadiness};
pub use terrain::PlacementPose;

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
	/// Destination + policy → pending work, or an explicit failure.
	/// Unresolved POIs do not become a silent floor at a stale coordinate.
	pub fn realize(
		self,
		registry: &PoiRegistry,
		surface: &DurhamSurface,
	) -> Result<PendingPlacement, PlacementFailed> {
		match PlacementPose::resolve(
			self.destination,
			registry,
			surface,
			self.fallback_at,
			self.clearance,
		) {
			Ok(pose) => match pose.reject_blocked(false) {
				Ok(pose) => Ok(PendingPlacement::new(self.identity, self.destination, pose)),
				Err(reason) => Err(PlacementFailed::new(self.identity, reason)),
			},
			Err(reason) => Err(PlacementFailed::new(self.identity, reason)),
		}
	}
}

impl PlacementPose {
	/// Resolve a picker / resume destination against the current registry and
	/// terrain. Unresolved POIs fail rather than treating a stale coordinate
	/// as a floor.
	pub fn resolve(
		destination: ArrivalDestination,
		registry: &PoiRegistry,
		surface: &DurhamSurface,
		fallback_at: Vec3,
		clearance: ActorClearance,
	) -> Result<Self, maybraid_spawns_player::SpawnFailureReason> {
		match destination {
			ArrivalDestination::Poi(id) => Self::resolve_poi(id, registry, surface, clearance),
			ArrivalDestination::Fallback => {
				Ok(Self::outdoor_at(surface, fallback_at.xz(), clearance))
			}
			ArrivalDestination::Resume(at) | ArrivalDestination::DeathInPlace(at) => {
				Ok(Self::outdoor_at(surface, at.xz(), clearance))
			}
		}
	}

	pub fn resolve_poi(
		id: PoiId,
		registry: &PoiRegistry,
		surface: &DurhamSurface,
		clearance: ActorClearance,
	) -> Result<Self, maybraid_spawns_player::SpawnFailureReason> {
		let Some(record) = registry.get(id) else {
			return Err(maybraid_spawns_player::SpawnFailureReason::DestinationUnresolved);
		};
		Ok(match PlacementPolicy::for_poi(record) {
			PlacementPolicy::InteriorFloor => Self::interior_at(record.position, clearance),
			PlacementPolicy::OutdoorGround => {
				Self::outdoor_at(surface, record.position.xz(), clearance)
			}
			PlacementPolicy::Airborne => Self::airborne_at(
				world_player::player_position_above_surface(record.position),
				clearance,
			),
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
	use durham::{BaseTerrainNoise, TerrainCellLayout, TerrainConfig, WorldBaseTerrain};
	use lod::hcsg::HcsgStorage;
	use maybraid_spawns_player::{
		SpawnFailureReason, SpawnIdentity, SpawnOwner, SpawnRequestId, SpawnSessionId,
	};
	use poi_intelligence::PoiId;

	fn identity() -> SpawnIdentity {
		SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1)))
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
					let request = ResolvePlacement::new(
						identity(),
						ArrivalDestination::Poi(PoiId(99)),
						ActorClearance::HUMANOID,
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
	fn resolve_uses_the_request_clearance() -> anyhow::Result<()> {
		let mut world = surface_world();
		world
			.run_system_once(
				|registry: Res<PoiRegistry>, surface: DurhamSurface| -> anyhow::Result<()> {
					let clearance = ActorClearance::new(2.4);
					let request = ResolvePlacement::new(
						identity(),
						ArrivalDestination::Fallback,
						clearance,
						Vec3::new(3.0, 4.0, 5.0),
					);
					let pending = request.realize(&registry, &surface).expect("fallback");
					assert_eq!(pending.clearance, clearance);
					assert_eq!(pending.pose.expect("pose").clearance, clearance);
					Ok(())
				},
			)
			.map_err(|error| anyhow::anyhow!("{error:?}"))??;
		Ok(())
	}

	#[test]
	fn blocked_clearance_is_an_explicit_failure() {
		let pose = PlacementPose::airborne_at(Vec3::Y, ActorClearance::HUMANOID);
		assert_eq!(
			pose.reject_blocked(true),
			Err(maybraid_spawns_player::SpawnFailureReason::Clearance)
		);
	}
}
