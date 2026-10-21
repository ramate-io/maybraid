//! Destination → floor / clearance / support. GenerationScheme is not
//! involved in collider readiness.

pub mod pending;
pub mod poi;
pub mod request;
pub mod support;
pub mod terrain;

pub use pending::{PendingPlacement, PlacementWork};
pub use poi::interior_building_poi;
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
use world_player::player_position_above_surface;

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
		.add_systems(Update, (cancel_placements, poll_placement_support).chain());
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
		match PlacementPose::resolve(self.destination, registry, surface, self.fallback_at) {
			Ok(pose) => Ok(PendingPlacement::new(self.identity, self.destination, pose)),
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
	) -> Result<Self, maybraid_spawns_core::SpawnFailureReason> {
		match destination {
			ArrivalDestination::Poi(id) => Self::resolve_poi(id, registry, surface),
			ArrivalDestination::Fallback => Ok(Self::outdoor_at(
				surface,
				fallback_at.xz(),
				crate::request::ActorClearance::HUMANOID,
			)),
			ArrivalDestination::Resume(at) | ArrivalDestination::DeathInPlace(at) => {
				Ok(Self::outdoor_at(surface, at.xz(), crate::request::ActorClearance::HUMANOID))
			}
		}
	}

	pub fn resolve_poi(
		id: PoiId,
		registry: &PoiRegistry,
		surface: &DurhamSurface,
	) -> Result<Self, maybraid_spawns_core::SpawnFailureReason> {
		let Some(record) = registry.get(id) else {
			return Err(maybraid_spawns_core::SpawnFailureReason::DestinationUnresolved);
		};
		Ok(match PlacementPolicy::for_poi(record) {
			PlacementPolicy::InteriorFloor => Self::interior_at(record.position),
			PlacementPolicy::OutdoorGround => Self::outdoor_at(
				surface,
				record.position.xz(),
				crate::request::ActorClearance::HUMANOID,
			),
			PlacementPolicy::Airborne => {
				Self::airborne_at(player_position_above_surface(record.position))
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
	support: Res<PlacementSupport>,
	mut ready: MessageWriter<PlacementReady>,
) {
	for outcome in work.poll(*support) {
		ready.write(outcome);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use durham::{BaseTerrainNoise, TerrainCellLayout, TerrainConfig, WorldBaseTerrain};
	use lod::hcsg::HcsgStorage;
	use maybraid_spawns_core::{
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
}
