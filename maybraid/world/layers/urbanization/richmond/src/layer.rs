//! [`Richmond<G>`]: urbanization over ground `G`.

use std::marker::PhantomData;

use bevy::ecs::system::{Res, ResMut, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::App;
use layer_stack::{LayerGenerationCore, RequireLayer};
use lod::gen::{Id, OriginalId, Version};
use lod::hcsg::HcsgStorage;
use lod::lod_ref::LodRef;
use urbanization_layer_model::{Urbanization, UrbanizationModel};

use crate::developments::RichmondDevelopment;
use crate::ground::RichmondGround;
use crate::padded::{PaddedTerrain, TerrainWithPads};
use crate::storage::{column_bounds, RichmondStorage};
use crate::{BuiltDevelopment, PadComplex};

/// Urbanization model over ground `G`.
pub struct Richmond<G>(PhantomData<fn() -> G>);

impl<G: RichmondGround> UrbanizationModel for Richmond<G> {
	type Ground = G;
	type Pads = PadComplex;
	type Surface = TerrainWithPads;
	type Built = BuiltDevelopment;
	type Read = Res<'static, HcsgStorage>;
	type Prepare = ResMut<'static, HcsgStorage>;

	fn pads(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> PadComplex {
		read.merged_pads::<G>(region)
	}

	fn pads_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> PadComplex {
		read.merged_pads::<G>(Aabb3d::from_min_max(
			Vec3::new(xz.x - 0.5, 0.0, xz.y - 0.5),
			Vec3::new(xz.x + 0.5, 0.0, xz.y + 0.5),
		))
	}

	fn surface<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<&'a TerrainWithPads> {
		read.get::<PaddedTerrain<G>>(id).map(|padded| &padded.surface)
	}

	fn surface_ids(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Vec<Id> {
		read.overlapping::<PaddedTerrain<G>>(region)
	}

	fn overlay_surface<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		bounds: Aabb3d,
	) -> Option<&'a TerrainWithPads> {
		read.padded_terrain_for::<G>(bounds)
	}

	fn built<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a BuiltDevelopment> {
		read.built_overlapping::<G>(region)
			.into_iter()
			.map(|(_, _, built)| built)
			.collect()
	}

	fn built_overlapping<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(Id, Version, &'a BuiltDevelopment)> {
		read.built_overlapping::<G>(region)
	}

	/// #720 wart: generate the developments over `bounds` before the grove sample.
	fn prepare(
		prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		bounds: Aabb3d,
		_lod_ref: &LodRef,
	) {
		let storage = &mut **prepare;
		for OriginalId(id) in
			storage.original_ids_for::<RichmondDevelopment<G>>(column_bounds(bounds))
		{
			storage.get_or_generate::<RichmondDevelopment<G>>(id);
		}
	}

	fn require_generation(app: &App) {
		app.require_layer::<LayerGenerationCore<Urbanization<Self>>, Urbanization<Self>>();
	}
}
