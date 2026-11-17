//! The world's layer stack: terrain, urbanization, vegetation, mobs and
//! furnishing and language, generated on the shared HCSG runtime and presented around the
//! viewer while Discovery streams.

use barking::{
	BarkingPresentationPlugin, HomeostaticMobLifecycle, HomeostaticMobSystems,
	HomeostaticMobsPlugin, MobNeighborhood, MobScenePresentationPlugin,
};
use bevy::prelude::*;
use chico::{
	BumpOutPresentationPlugin, BumpOutRing, CanopyBumpOut, ChicoPresentationPlugin,
	GroveNeighborhood, MediumCanopyBumpOut,
};
use durham::{
	BackgroundStream, Durham, DurhamWorldPlugin, FarStream, NearStream, StreamPresentationPlugin,
	StreamRing, Water,
};
use geneva::{GenevaPlugin, LanguageNeighborhood};
use layer_stack::{ActiveGenerationMode, GenerationModePlugin};
use lod::hcsg::{
	register_session_seed, request_hcsg_session_restart, HcsgBoundsPlugin, HcsgSessionRestarted,
};
use maputo::{FurnitureNeighborhood, MaputoPresentationPlugin};
use maybraid_game_mode_discover::{Discovery, InDiscovery};
use maybraid_session::{SessionCapabilities, SessionCapabilitySystems};
use richmond::{
	AuthoredDevelopments, BuiltPresentationPlugin, DevelopmentNeighborhood, PaddedTerrain,
	RichmondConfig,
};
use terrain_layer_model::OnTerrain;
use terrain_layer_model::TerrainStreaming;
use urbanization_layer_model::Urbanization;

type Ground = OnTerrain<Durham>;
type Urban = Urbanization<richmond::Richmond<Ground>>;

/// Stream `R`'s cells while Discovery streams.
type Ring<R> = InDiscovery<StreamRing<R>>;

/// The world seed every layer derives from.
const WORLD_SEED: u32 = 42;

/// Every world layer at world defaults.
///
/// Add after [`crate::WorldMaterialRefPlugin`], which installs the unique world
/// material resolver, and after `PlayerCameraPlugin`, since mob presentation
/// adds `PlayerPlugin` when missing.
pub struct WorldLayersPlugin;

impl Plugin for WorldLayersPlugin {
	fn build(&self, app: &mut App) {
		let richmond = RichmondConfig::world_defaults();
		app.add_plugins((GenerationModePlugin::<Discovery>::initial(), DurhamWorldPlugin {
			seed: WORLD_SEED,
		}))
		.insert_resource(crate::poi::PaddedTerrainPoiGround)
		.insert_resource(richmond.development_config())
		.insert_resource(richmond.urbanization_selection())
		.init_resource::<AuthoredDevelopments>()
		.add_plugins((
			HcsgBoundsPlugin::<Ring<NearStream>>::default(),
			HcsgBoundsPlugin::<Ring<FarStream>>::default(),
			HcsgBoundsPlugin::<Ring<BackgroundStream>>::default(),
			HcsgBoundsPlugin::<InDiscovery<DevelopmentNeighborhood>>::default(),
			HcsgBoundsPlugin::<InDiscovery<GroveNeighborhood>>::default(),
			HcsgBoundsPlugin::<InDiscovery<BumpOutRing<CanopyBumpOut>>>::default(),
			HcsgBoundsPlugin::<InDiscovery<BumpOutRing<MediumCanopyBumpOut>>>::default(),
			HcsgBoundsPlugin::<InDiscovery<MobNeighborhood>>::default(),
			HcsgBoundsPlugin::<InDiscovery<FurnitureNeighborhood>>::default(),
			HcsgBoundsPlugin::<InDiscovery<LanguageNeighborhood>>::default(),
		))
		.add_plugins((
			StreamPresentationPlugin::<Ring<NearStream>, NearStream, PaddedTerrain<Ground>>::default(),
			StreamPresentationPlugin::<Ring<FarStream>, FarStream, PaddedTerrain<Ground>>::default(),
			StreamPresentationPlugin::<
				Ring<BackgroundStream>,
				BackgroundStream,
				PaddedTerrain<Ground>,
			>::default(),
			StreamPresentationPlugin::<Ring<NearStream>, NearStream, Water>::default(),
			StreamPresentationPlugin::<Ring<FarStream>, FarStream, Water>::default(),
			StreamPresentationPlugin::<Ring<BackgroundStream>, BackgroundStream, Water>::default(),
		))
		.add_plugins((
			BuiltPresentationPlugin::<InDiscovery<DevelopmentNeighborhood>, Ground>::default(),
			ChicoPresentationPlugin::<InDiscovery<GroveNeighborhood>, Urban>::default(),
			BumpOutPresentationPlugin::<
				InDiscovery<BumpOutRing<CanopyBumpOut>>,
				CanopyBumpOut,
				Urban,
			>::default(),
			BumpOutPresentationPlugin::<
				InDiscovery<BumpOutRing<MediumCanopyBumpOut>>,
				MediumCanopyBumpOut,
				Urban,
			>::default(),
			MaputoPresentationPlugin::<InDiscovery<FurnitureNeighborhood>, Urban>::default(),
			BarkingPresentationPlugin::<InDiscovery<MobNeighborhood>, Urban>::default(),
			MobScenePresentationPlugin,
			HomeostaticMobsPlugin::<world_player::Player>::default(),
			GenevaPlugin::<InDiscovery<LanguageNeighborhood>, Urban>::default(),
		))
		.add_systems(
			Update,
			sync_homeostatic_lifetime
				.after(SessionCapabilitySystems)
				.after(HcsgSessionRestarted)
				.before(HomeostaticMobSystems::Lifecycle),
		)
		.add_systems(
			OnEnter(ActiveGenerationMode::of::<Discovery>()),
			request_hcsg_session_restart,
		);
		register_session_seed(app, reset_homeostatic_session);
	}
}

fn reset_homeostatic_session(mut lifecycle: ResMut<HomeostaticMobLifecycle>) {
	lifecycle.request_reset();
}

/// Installed participation owns population lifetime. Pause only suspends input,
/// and deliberately does not uninstall these capabilities.
fn sync_homeostatic_lifetime(
	session: Option<Res<SessionCapabilities>>,
	streaming: Res<TerrainStreaming<Durham>>,
	mode: Res<State<ActiveGenerationMode>>,
	mut lifecycle: ResMut<HomeostaticMobLifecycle>,
) {
	let installed = session
		.as_deref()
		.is_none_or(|session| session.gameplay().is_installed() && session.streaming_enabled());
	let enabled = installed && streaming.enabled && mode.get().is::<Discovery>();
	if lifecycle.enabled != enabled {
		lifecycle.enabled = enabled;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use barking::homeostatic::reset_homeostatic_mobs;
	use barking::{HomeostaticCellIndex, HomeostaticMob, PlayerHomeostaticCell};
	use damage::Health;
	use maybraid_controllers_core::InputRouter;
	use maybraid_session::WorldGameplayEnabled;
	use mob_intelligence::{MemberOf, Mob};

	#[test]
	fn pause_keeps_population_but_home_retires_it_with_the_player_still_present(
	) -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(TerrainStreaming::<Durham>::new(true))
			.insert_resource(State::new(ActiveGenerationMode::of::<Discovery>()))
			.init_resource::<HomeostaticMobLifecycle>()
			.insert_resource(PlayerHomeostaticCell {
				current: Some(HomeostaticCellIndex(0, 0)),
				generation: 1,
			})
			.add_systems(Update, (sync_homeostatic_lifetime, reset_homeostatic_mobs).chain())
			.add_observer(barking::hcsg::retire_members_with_their_mob);
		// This installs the actual shared worker/session/late-retirement runtime.
		register_session_seed(&mut app, reset_homeostatic_session);
		let mut session = SessionCapabilities::default();
		session.install_play(true);
		app.insert_resource(session);
		let player = app.world_mut().spawn(world_player::Player).id();
		let host = app.world_mut().spawn(Mob::new(12.0)).id();
		app.world_mut().entity_mut(host).insert(HomeostaticMob {
			home_cell: HomeostaticCellIndex(0, 0),
			mob_entity: host,
			generation: 1,
		});
		let health = Health { current: 37.0, max: 100.0 };
		let body = app.world_mut().spawn((MemberOf { mob: host, slot: 0 }, health)).id();
		let mut router = InputRouter::default();
		for paused in [false, true, false] {
			router.set_paused(paused);
			let session = *app.world().resource::<SessionCapabilities>();
			assert!(session.streaming_enabled());
			assert_eq!(session.gameplay_enabled(&router), !paused);
			app.insert_resource(WorldGameplayEnabled(session.gameplay_enabled(&router)));
			app.update();
			assert!(app.world().get_entity(host).is_ok());
			assert_eq!(app.world().get::<Health>(body), Some(&health));
		}
		app.world_mut().resource_mut::<SessionCapabilities>().install_menu_shell();
		assert!(!app.world().resource::<SessionCapabilities>().streaming_enabled());
		app.update();
		assert!(app.world().get_entity(player).is_ok());
		assert!(app.world().get_entity(host).is_err());
		assert!(app.world().get_entity(body).is_err());
		assert_eq!(app.world().resource::<PlayerHomeostaticCell>().current, None);
		Ok(())
	}
}
