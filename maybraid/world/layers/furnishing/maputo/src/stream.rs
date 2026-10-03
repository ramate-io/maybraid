//! Camera keep and 50 m cell generate. Present stays in [`crate::present`].

use bevy::ecs::system::StaticSystemParam;
use bevy::prelude::*;
use layer_stack::{GenerationMode, GenerationModeSystems};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion};
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};
use lod::LodGenerateKeepRegion;
use lod::LodSceneRefreshRegion;
use urbanization_layer_model::UrbanizationGenerationSystems;

use crate::cell::{
	xz_radius_aabb, FurnitureCellExtent, FURNITURE_GENERATE_RADIUS, FURNITURE_PRESENT_RADIUS,
};
use crate::index::FurnitureIndex;
use crate::present::{FurnitureLodChan, FurnitureRefresh};
use crate::slots::FurnitureSlots;
use furnishing_layer_model::FurnishingGenerationSystems;

/// One 50 m host begin / rebuild per frame so fulfill can drain kits.
const FURNITURE_GENERATE_CELLS_PER_FRAME: usize = 1;

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct FurnitureGenerateBullseye {
	pub(crate) radius_m: f32,
	enabled: bool,
}

impl Default for FurnitureGenerateBullseye {
	fn default() -> Self {
		Self { radius_m: FURNITURE_GENERATE_RADIUS, enabled: true }
	}
}

impl LodRefreshRegions for FurnitureGenerateBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		refresh_status(self.enabled, self.radius_m, lod_ref)
	}
}

#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct FurniturePresentBullseye {
	pub(crate) radius_m: f32,
	enabled: bool,
}

impl Default for FurniturePresentBullseye {
	fn default() -> Self {
		Self { radius_m: FURNITURE_PRESENT_RADIUS, enabled: true }
	}
}

impl LodRefreshRegions for FurniturePresentBullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		refresh_status(self.enabled, self.radius_m, lod_ref)
	}
}

fn refresh_status(enabled: bool, radius: f32, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
	if !enabled {
		return LodRefreshRegionsStatus::Unchanged;
	}
	let previous =
		FurnitureCellExtent::cell_index_containing(lod_ref.previous_transform.translation);
	let current = FurnitureCellExtent::cell_index_containing(lod_ref.current_transform.translation);
	if previous == current {
		LodRefreshRegionsStatus::Unchanged
	} else {
		LodRefreshRegionsStatus::Changed(xz_radius_aabb(
			lod_ref.current_transform.translation,
			radius,
		))
	}
}

/// Drive generate / present keep from the camera. Crossing a 50 m cell re-emits regions.
fn stream_furniture_keep(
	camera: Query<&Transform, With<Camera3d>>,
	mut generate: ResMut<FurnitureGenerateBullseye>,
	mut present: ResMut<FurniturePresentBullseye>,
	mut generate_keep: ResMut<LodGenerateKeepRegion<FurnitureLodChan>>,
	mut present_keep: ResMut<LodPresentKeepRegion<FurnitureLodChan>>,
	mut present_regions: MessageWriter<LodPresentRegion<FurnitureLodChan>>,
	mut refresh_regions: MessageWriter<LodSceneRefreshRegion<FurnitureRefresh>>,
	mut previous_cell: Local<Option<(i32, i32)>>,
) {
	let Ok(camera) = camera.single() else {
		return;
	};
	generate.enabled = true;
	present.enabled = true;
	let generate_aabb = xz_radius_aabb(camera.translation, generate.radius_m);
	let present_aabb = xz_radius_aabb(camera.translation, present.radius_m);
	generate_keep.region = Some(generate_aabb);
	present_keep.region = Some(present_aabb);
	let current = FurnitureCellExtent::cell_index_containing(camera.translation);
	if previous_cell.as_ref() == Some(&current) {
		return;
	}
	present_regions.write(LodPresentRegion::new(present_aabb));
	refresh_regions.write(LodSceneRefreshRegion::new(present_aabb));
	*previous_cell = Some(current);
}

/// Materialize occupied 50 m cells from [`FurnitureSlots`].
fn generate_furniture_cells<G: FurnitureSlots>(
	read: StaticSystemParam<G::Read>,
	generate_keep: Res<LodGenerateKeepRegion<FurnitureLodChan>>,
	present_keep: Res<LodPresentKeepRegion<FurnitureLodChan>>,
	mut index: ResMut<FurnitureIndex>,
) {
	let Some(region) = generate_keep.region.or(present_keep.region) else {
		return;
	};
	index.refresh_slots(G::overlapping_tracked(&read, region), region, |id| {
		G::world_slots(&read, id)
	});
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	index.generate_cells(region, &lod_ref, FURNITURE_GENERATE_CELLS_PER_FRAME);
}

pub(crate) fn register_furniture_generate(app: &mut App) {
	app.init_resource::<FurnitureIndex>()
		.init_resource::<FurnitureGenerateBullseye>()
		.init_resource::<FurniturePresentBullseye>()
		.init_resource::<LodGenerateKeepRegion<FurnitureLodChan>>()
		.init_resource::<LodPresentKeepRegion<FurnitureLodChan>>();
}

/// Keep and generate for `Mode`, after urbanization has stored developments.
pub fn install_furnishing_stream<Mode, G>(app: &mut App)
where
	Mode: GenerationMode,
	G: FurnitureSlots,
{
	app.add_systems(
		Update,
		(stream_furniture_keep, generate_furniture_cells::<G>)
			.chain()
			.in_set(GenerationModeSystems::<Mode>::default())
			.in_set(FurnishingGenerationSystems)
			.after(UrbanizationGenerationSystems),
	);
}
