//! Keep-ring tiles and names after vegetation and urbanization cells exist.

use bevy::ecs::system::StaticSystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use language_layer_model::LanguageGenerationSystems;
use layer_stack::{GenerationMode, GenerationModeSystems};
use lod::LodGenerateKeepRegion;
use urbanization_layer_model::UrbanizationGenerationSystems;
use vegetation_layer_model::VegetationGenerationSystems;

use crate::index::{origin_keep, LanguageIndex, LanguageWorldSeed};
use crate::present::{LanguageLodChan, LanguageOverlay};
use crate::sources::NamedWorld;
const LANGUAGE_GENERATE_RADIUS: f32 = 40_000.0;

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct LanguageGenerateBullseye {
	pub(crate) radius_m: f32,
	enabled: bool,
}

impl Default for LanguageGenerateBullseye {
	fn default() -> Self {
		Self { radius_m: LANGUAGE_GENERATE_RADIUS, enabled: true }
	}
}

fn xz_radius_aabb(origin: Vec3, radius: f32) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(origin.x - radius, -1.0, origin.z - radius),
		Vec3::new(origin.x + radius, 1.0, origin.z + radius),
	)
}

fn stream_language_keep(
	camera: Query<&Transform, With<Camera3d>>,
	mut generate: ResMut<LanguageGenerateBullseye>,
	mut generate_keep: ResMut<LodGenerateKeepRegion<LanguageLodChan>>,
) {
	let Ok(camera) = camera.single() else {
		return;
	};
	generate.enabled = true;
	generate_keep.region = Some(xz_radius_aabb(camera.translation, generate.radius_m));
}

fn generate_language_region<W: NamedWorld>(
	read: StaticSystemParam<W::Read>,
	seed: Res<LanguageWorldSeed>,
	generate_keep: Res<LodGenerateKeepRegion<LanguageLodChan>>,
	mut index: ResMut<LanguageIndex>,
	mut overlay: ResMut<LanguageOverlay>,
) {
	let region = generate_keep.region.unwrap_or_else(origin_keep);
	let mut features = W::groves_overlapping(&read, region);
	features.extend(W::geography_overlapping(&read, region));
	features.extend(W::urban_overlapping(&read, region));
	let places = W::places_overlapping(&read, region);
	index.assign_keep(seed.0, region, &features, &places);
	*overlay = LanguageOverlay::from_index(&index);
}

pub(crate) fn register_language_generate(app: &mut App) {
	app.init_resource::<LanguageIndex>()
		.init_resource::<LanguageWorldSeed>()
		.init_resource::<LanguageOverlay>()
		.init_resource::<LanguageGenerateBullseye>()
		.init_resource::<LodGenerateKeepRegion<LanguageLodChan>>();
}

/// Keep and name assignment for `Mode`, after vegetation and urbanization.
pub fn install_language_stream<Mode, W>(app: &mut App)
where
	Mode: GenerationMode,
	W: NamedWorld,
{
	app.add_systems(
		Update,
		(stream_language_keep, generate_language_region::<W>)
			.chain()
			.in_set(GenerationModeSystems::<Mode>::default())
			.in_set(LanguageGenerationSystems)
			.after(VegetationGenerationSystems)
			.after(UrbanizationGenerationSystems),
	);
}

pub(crate) fn apply_origin_tiles(world: &mut World) {
	let seed = world.get_resource::<LanguageWorldSeed>().map(|seed| seed.0).unwrap_or(0);
	{
		let mut index = world.get_resource_or_insert_with(LanguageIndex::default);
		index.ensure_tiles(seed, origin_keep());
	}
	let snapshot = world.resource::<LanguageIndex>().clone();
	if let Some(mut overlay) = world.get_resource_mut::<LanguageOverlay>() {
		*overlay = LanguageOverlay::from_index(&snapshot);
	}
}
