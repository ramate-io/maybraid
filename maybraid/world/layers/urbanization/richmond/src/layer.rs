//! [`Richmond<G>`]: urbanization over ground `G`.

use std::marker::PhantomData;

use bevy::ecs::system::{Res, SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::App;
use layer_stack::RequireLayer;
use lod::gen::{
	GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex, TrackedId, Version,
};
use lod::lod_ref::LodRef;
use urbanization_cells::UrbanizationIndex;
use urbanization_layer_model::{Urbanization, UrbanizationGenerationCore, UrbanizationModel};

use crate::development::DevelopmentCell;
use crate::ground::RichmondGround;
use crate::index::{DevelopmentEntryStore, DevelopmentIndex, PaddedStoreView};
use crate::padded::TerrainWithPads;
use crate::{BuiltDevelopment, PadComplex};

/// Urbanization model over ground `G`.
pub struct Richmond<G>(PhantomData<fn() -> G>);

/// Resources behind [`Richmond<G>`]'s [`UrbanizationModel::Read`].
#[derive(SystemParam)]
pub struct RichmondRead<'w> {
	pub developments: Res<'w, DevelopmentEntryStore>,
	pub urbanization: Res<'w, UrbanizationIndex>,
}

impl RichmondRead<'_> {
	fn pads_at(&self, xz: Vec2) -> PadComplex {
		self.developments.merged_pad_complex(Aabb3d::from_min_max(
			Vec3::new(xz.x - 0.5, -10_000.0, xz.y - 0.5),
			Vec3::new(xz.x + 0.5, 10_000.0, xz.y + 0.5),
		))
	}
}

impl<G: RichmondGround> UrbanizationModel for Richmond<G> {
	type Ground = G;
	type Pads = PadComplex;
	type Surface = TerrainWithPads;
	type Built = BuiltDevelopment;
	type Read = RichmondRead<'static>;
	type Prepare = DevelopmentIndex<'static, 'static, G>;

	fn pads(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> PadComplex {
		read.developments.merged_pad_complex(region)
	}

	fn pads_at(read: &SystemParamItem<'_, '_, Self::Read>, xz: Vec2) -> PadComplex {
		read.pads_at(xz)
	}

	fn surface<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		id: Id,
	) -> Option<&'a TerrainWithPads> {
		read.developments.padded(id)
	}

	fn surface_ids(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d) -> Vec<Id> {
		PaddedStoreView::new(&read.developments)
			.tracked_ids_for(region)
			.into_iter()
			.map(|TrackedId(id)| id)
			.collect()
	}

	fn overlay_surface<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		bounds: Aabb3d,
	) -> Option<&'a TerrainWithPads> {
		read.developments.padded_terrain_for(bounds)
	}

	fn built<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a BuiltDevelopment> {
		read.developments.developments_overlapping(region)
	}

	fn built_overlapping<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(Id, Version, &'a BuiltDevelopment)> {
		read.developments.developments_overlapping_tracked(region)
	}

	/// #720 wart: generate development cells for `bounds` before the grove sample.
	fn prepare(
		prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	) {
		let ids =
			<DevelopmentCell as GenerationScheme<DevelopmentIndex<'_, '_, G>>>::original_ids_for(
				prepare, bounds,
			);
		for OriginalId(development_id) in ids {
			let _ = GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate(
				prepare,
				development_id,
				lod_ref,
			);
		}
	}

	fn require_generation(app: &App) {
		app.require_layer::<UrbanizationGenerationCore<Self>, Urbanization<Self>>();
	}
}
