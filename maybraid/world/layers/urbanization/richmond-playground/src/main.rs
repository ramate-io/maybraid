use std::path::{Path, PathBuf};

use bevy::prelude::*;
use chico::{
	BumpOutPresentationPlugin, BumpOutRing, CanopyBumpOut, ChicoPresentationPlugin,
	GroveNeighborhood, MediumCanopyBumpOut,
};
use durham::{Durham, DurhamTerrainConfig, DurhamWindow, WaterPresentationPlugin};
use layer_stack::{Generate, GenerationModePlugin};
use lod::hcsg::shared::HcsgBoundsPlugin;
use maputo::{FurnitureNeighborhood, MaputoPresentationPlugin};
use richmond::{Richmond, RichmondConfig, RichmondPresentationPlugin, UrbanizationStreamSpec};
use richmond_playground::{
	DevelopmentsOnTerrainPlugin, PendingStartupCommand, PlaygroundCommand, PlaygroundConfig,
	PlaygroundMode,
};
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;

type Ground = OnTerrain<Durham>;
type Urban = Urbanization<Richmond<Ground>>;

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../assets")
}

fn main() {
	let startup = PlaygroundCommand::parse_startup().unwrap_or_else(|e| {
		eprintln!("{e}");
		std::process::exit(2);
	});
	let playground = PlaygroundConfig::default();
	let urban = RichmondConfig {
		focus_development: startup.focus_development,
		urbanization: Some(UrbanizationStreamSpec::default()),
		..RichmondConfig::default()
	};

	let assets_path = assets_root();
	let mut app = App::new();
	app.add_plugins(
		DefaultPlugins
			.set(WindowPlugin {
				primary_window: Some(Window {
					title: "Richmond Developments on Terrain".into(),
					resolution: (1280, 720).into(),
					..default()
				}),
				..default()
			})
			.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() }),
	);
	app.insert_resource(PendingStartupCommand(startup.command));
	app.add_plugins(GenerationModePlugin::<PlaygroundMode>::initial());
	app.add_plugins(Generate::<PlaygroundMode, Ground>::new(DurhamTerrainConfig::fine_patch(
		playground.terrain_radius,
	)));
	app.add_plugins(Generate::<PlaygroundMode, Urban>::new(urban));
	app.add_plugins((
		HcsgBoundsPlugin::<DurhamWindow>::default(),
		HcsgBoundsPlugin::<FurnitureNeighborhood>::default(),
		HcsgBoundsPlugin::<GroveNeighborhood>::default(),
		HcsgBoundsPlugin::<BumpOutRing<CanopyBumpOut>>::default(),
		HcsgBoundsPlugin::<BumpOutRing<MediumCanopyBumpOut>>::default(),
	));
	app.add_plugins(WaterPresentationPlugin::<DurhamWindow>::default());
	app.add_plugins(RichmondPresentationPlugin::<DurhamWindow, Ground>::default());
	app.add_plugins(MaputoPresentationPlugin::<FurnitureNeighborhood, Urban>::default());
	app.add_plugins(ChicoPresentationPlugin::<GroveNeighborhood, Urban>::default());
	app.add_plugins((
		BumpOutPresentationPlugin::<BumpOutRing<CanopyBumpOut>, CanopyBumpOut, Urban>::default(),
		BumpOutPresentationPlugin::<
			BumpOutRing<MediumCanopyBumpOut>,
			MediumCanopyBumpOut,
			Urban,
		>::default(),
	));
	app.add_plugins(DevelopmentsOnTerrainPlugin { config: playground, commands: true })
		.run();
}
