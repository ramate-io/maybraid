//! [`Richmond<G>`]: urbanization over ground `G`.

use std::marker::PhantomData;

use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::App;
use lod::gen::{Id, Version};
use lod::hcsg::HcsgStorage;
use lod::lod_ref::LodRef;
use terrain_layer_model::TerrainModel;
use urbanization_layer_model::UrbanizationModel;

use crate::built::Built;
use crate::developments::RichmondDevelopment;
use crate::ground::RichmondGround;
use crate::padded::{PaddedTerrain, TerrainWithPads};
use crate::{BuiltDevelopment, PadComplex};

/// Urbanization model over ground `G`.
pub struct Richmond<G>(PhantomData<fn() -> G>);

impl<G> UrbanizationModel for Richmond<G>
where
	G: RichmondGround + TerrainModel,
{
	type Ground = G;
	type Pads = PadComplex;
	type Surface = TerrainWithPads;
	type Built = BuiltDevelopment;
	type Read = bevy::ecs::system::Res<'static, HcsgStorage>;
	type Prepare = bevy::ecs::system::ResMut<'static, HcsgStorage>;

	fn pads(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> PadComplex {
		RichmondDevelopment::<G>::merged_pads(read, region)
	}

	fn pads_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> PadComplex {
		RichmondDevelopment::<G>::merged_pads(
			read,
			Aabb3d::from_min_max(
				Vec3::new(xz.x - 0.5, 0.0, xz.y - 0.5),
				Vec3::new(xz.x + 0.5, 0.0, xz.y + 0.5),
			),
		)
	}

	fn surface(
		read: &SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<Self::Surface> {
		read.get::<PaddedTerrain<G>>(id).map(|padded| padded.surface.clone())
	}

	fn surface_ids(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Vec<Id> {
		read.overlapping::<PaddedTerrain<G>>(region)
	}

	fn overlay_surface(
		read: &SystemParamItem<'_, '_, Self::Read>,
		bounds: Aabb3d,
	) -> Option<Self::Surface> {
		PaddedTerrain::<G>::best_overlapping(read, bounds)
	}

	fn built(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<Self::Built> {
		Built::<G>::overlapping(read, region)
			.into_iter()
			.map(|(_, _, built)| built)
			.collect()
	}

	fn built_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(Id, Version, Self::Built)> {
		Built::<G>::overlapping(read, region)
	}

	fn prepare(_prepare: &mut SystemParamItem<'_, '_, Self::Prepare>, _bounds: Aabb3d, _lod_ref: &LodRef) {}

	fn require_generation(_app: &App) {}
}
