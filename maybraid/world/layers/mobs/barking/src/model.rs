//! [`Barking<G>`]: mobs over ground `G`.

use std::marker::PhantomData;

use bevy::prelude::{App, IntoScheduleConfigs, Update, With, World};
use layer_stack::{LayerGenerationCore, RequireLayer};
use lod::gen::LodGenerateBudget;
use lod::{
	LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems, LodPresentRegionPlugin,
	LodPresentSystems, LodViewer,
};
use mob_layer_model::{MobGeneration, MobGenerationSystems, MobModel, Mobs};
use terrain_layer_model::TerrainModel;
use vegetation_layer_model::{Vegetation, VegetationModel};

use crate::config::BarkingConfig;
use crate::index::{MobCell, MobIndex};
use crate::present::install_barking_presentation;
use crate::sample::{BarkingEnvironment, ForestSelection};
use crate::stream::{
	stream_mob_present, sync_mob_models, sync_mob_plant_hosts, MobCellWrites, MobGenerateBullseye,
	MobLodChan, MobPresentBullseye,
};

/// Mob model over ground `G`.
pub struct Barking<G>(PhantomData<fn() -> G>);

impl<V> MobModel for Barking<Vegetation<V>>
where
	V: VegetationModel + ForestSelection,
	Vegetation<V>: TerrainModel,
	V::Ground: BarkingEnvironment,
{
	type Ground = Vegetation<V>;
	type Cell = MobCell;
	type Writes = MobCellWrites<'static>;

	fn require_generation(app: &App) {
		app.require_layer::<LayerGenerationCore<Mobs<Self>>, Mobs<Self>>();
	}
}

impl<V> MobGeneration for Barking<Vegetation<V>>
where
	V: VegetationModel + ForestSelection,
	Vegetation<V>: TerrainModel,
	V::Ground: BarkingEnvironment,
{
	const LABEL: &'static str = "barking";
	type Config = BarkingConfig;

	fn install_generation(app: &mut App) {
		app.init_resource::<MobIndex>()
			.init_resource::<MobGenerateBullseye>()
			.init_resource::<MobPresentBullseye>()
			.init_resource::<LodGenerateBudget<MobLodChan>>()
			.add_plugins(LodGenerateRegionPlugin::<
				MobGenerateBullseye,
				With<LodViewer>,
				MobLodChan,
			>::default())
			.add_plugins(LodGeneratePlugin::<MobCell, MobIndex, MobLodChan, With<LodViewer>>::default())
			.add_plugins(LodPresentRegionPlugin::<
				MobPresentBullseye,
				With<LodViewer>,
				MobLodChan,
			>::default())
			.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain))
			.add_systems(
				Update,
				(
					sync_mob_models::<V, V::Ground>,
					sync_mob_plant_hosts::<V::Ground>,
				)
					.chain()
					.in_set(MobGenerationSystems)
					.after(LodGenerateSystems::Produce)
					.before(LodGenerateSystems::Drain),
			)
			.add_systems(
				Update,
				stream_mob_present
					.in_set(MobGenerationSystems)
					.before(LodGenerateSystems::Produce)
					.before(LodPresentSystems::Produce),
			);
	}

	fn apply_generation(world: &mut World, config: &BarkingConfig) {
		*world.resource_mut::<LodGenerateBudget<MobLodChan>>() =
			LodGenerateBudget::new(config.generate_budget);
	}

	fn clear_generation(world: &mut World) {
		world.resource_mut::<MobIndex>().clear();
	}

	fn install_presentation(app: &mut App) {
		layer_stack::install_lod_present_gate::<Mobs<Self>, MobLodChan>(app);
		install_barking_presentation::<Vegetation<V>>(app);
	}
}
