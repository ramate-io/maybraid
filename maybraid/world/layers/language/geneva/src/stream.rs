//! Keep-ring tiles and nearby names after vegetation and urbanization cells exist.

use bevy::ecs::system::StaticSystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::*;
use language_layer_model::LanguageGenerationSystems;
use layer_stack::{GenerationMode, GenerationModeSystems};
use lod::LodGenerateKeepRegion;
use urbanization_layer_model::UrbanizationGenerationSystems;
use vegetation_layer_model::VegetationGenerationSystems;

use crate::index::{
	origin_keep, LanguageIndex, LanguageSourceDeps, LanguageWorldSeed, SourceClass,
};
use crate::present::LanguageLodChan;
use crate::sources::{NamedWorld, NamingRegion, NAME_WINDOW_QUANT_M};

const LANGUAGE_GENERATE_RADIUS: f32 = 40_000.0;
/// Nearby entity naming. Language tiles keep the broad generate window.
pub(crate) const LANGUAGE_NAME_RADIUS: f32 = 400.0;
const LANGUAGE_NAME_HYSTERESIS: f32 = 80.0;
/// Modest per-frame assignment budget. Overlay rebuild is presentation's job.
const ASSIGN_BUDGET: usize = 32;

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

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct LanguageNamingBullseye {
	pub(crate) origin: Vec2,
	pub(crate) collect_radius: f32,
	pub(crate) retain_radius: f32,
}

impl Default for LanguageNamingBullseye {
	fn default() -> Self {
		Self {
			origin: Vec2::ZERO,
			collect_radius: collect_radius(),
			retain_radius: retain_radius(),
		}
	}
}

impl LanguageNamingBullseye {
	fn region(self) -> NamingRegion {
		NamingRegion::around(self.origin, self.collect_radius, self.retain_radius)
	}
}

fn collect_radius() -> f32 {
	LANGUAGE_NAME_RADIUS + NAME_WINDOW_QUANT_M * 0.5
}

fn retain_radius() -> f32 {
	collect_radius() + LANGUAGE_NAME_HYSTERESIS
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
	mut naming: ResMut<LanguageNamingBullseye>,
	mut generate_keep: ResMut<LodGenerateKeepRegion<LanguageLodChan>>,
) {
	let origin = camera.single().map(|transform| transform.translation).unwrap_or(Vec3::ZERO);
	generate.enabled = true;
	generate_keep.region = Some(xz_radius_aabb(origin, generate.radius_m));
	naming.origin = Vec2::new(origin.x, origin.z);
	naming.collect_radius = collect_radius();
	naming.retain_radius = retain_radius();
}

fn generate_language_region<W: NamedWorld>(
	read: StaticSystemParam<W::Read>,
	seed: Res<LanguageWorldSeed>,
	generate_keep: Res<LodGenerateKeepRegion<LanguageLodChan>>,
	naming: Res<LanguageNamingBullseye>,
	mut index: ResMut<LanguageIndex>,
) {
	let tile_region = generate_keep.region.unwrap_or_else(origin_keep);
	let naming_region = naming.region();
	index.ensure_tiles(seed.0, tile_region);

	let deps = LanguageSourceDeps::from_windows(
		W::source_revisions(&read),
		seed.0,
		tile_region,
		naming_region.origin,
	);
	match index.source_deps() {
		Some(prev) if prev == deps => {}
		Some(prev) if prev.seed == deps.seed => {
			let naming_moved = !prev.naming_window_match(deps);
			if naming_moved || prev.revisions.forest != deps.revisions.forest {
				let snapshot = W::groves_snapshot(&read, naming_region, &index);
				index.queue_feature_snapshot(
					seed.0,
					tile_region,
					snapshot,
					SourceClass::Vegetation,
				);
			}
			if naming_moved || prev.revisions.terrain != deps.revisions.terrain {
				let snapshot = W::geography_snapshot(&read, naming_region, &index);
				index.queue_feature_snapshot(seed.0, tile_region, snapshot, SourceClass::Geography);
			}
			if naming_moved || prev.revisions.urban != deps.revisions.urban {
				let snapshot = W::urban_snapshot(&read, naming_region, &index);
				index.queue_feature_snapshot(seed.0, tile_region, snapshot, SourceClass::Urban);
			}
			if naming_moved || prev.revisions.places != deps.revisions.places {
				let snapshot = W::places_snapshot(&read, naming_region, &index);
				index.queue_place_snapshot(seed.0, tile_region, snapshot);
			}
			index.note_source_deps(deps);
		}
		_ => {
			let groves = W::groves_snapshot(&read, naming_region, &index);
			index.queue_feature_snapshot(seed.0, tile_region, groves, SourceClass::Vegetation);
			let geography = W::geography_snapshot(&read, naming_region, &index);
			index.queue_feature_snapshot(seed.0, tile_region, geography, SourceClass::Geography);
			let urban = W::urban_snapshot(&read, naming_region, &index);
			index.queue_feature_snapshot(seed.0, tile_region, urban, SourceClass::Urban);
			let places = W::places_snapshot(&read, naming_region, &index);
			index.queue_place_snapshot(seed.0, tile_region, places);
			index.note_source_deps(deps);
		}
	}
	index.assign_budgeted(seed.0, ASSIGN_BUDGET);
}

pub(crate) fn register_language_generate(app: &mut App) {
	app.init_resource::<LanguageIndex>()
		.init_resource::<LanguageWorldSeed>()
		.init_resource::<LanguageGenerateBullseye>()
		.init_resource::<LanguageNamingBullseye>()
		.init_resource::<LodGenerateKeepRegion<LanguageLodChan>>();
}

/// Keep and name assignment for `Mode`, after vegetation and urbanization.
///
/// `GlobalTransform` reads use last frame's PostUpdate propagation. Language
/// assignment is in `Update`, so parented POIs need a frame of transform sync.
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
	let mut index = world.get_resource_or_insert_with(LanguageIndex::default);
	index.ensure_tiles(seed, origin_keep());
}
