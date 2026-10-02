//! [`Richmond<G>`]: urbanization over ground `G`.

use std::marker::PhantomData;

use bevy::ecs::system::{Res, ResMut, SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::App;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, Id, OriginalId, SpatialIndex, TrackedId};
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;
use urbanization_cells::{
	DevelopmentLeaf, UrbanizationExtent, UrbanizationIndex, UrbanizationKind,
};
use urbanization_layer_model::{
	UrbanSource, Urbanization, UrbanizationGenerationCore, UrbanizationModel,
};
use layer_stack::RequireLayer;

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

	/// #720 wart: generate development cells for `bounds` before the grove sample.
	fn prepare(
		prepare: &mut SystemParamItem<'_, '_, Self::Prepare>,
		bounds: Aabb3d,
		lod_ref: &LodRef,
	) {
		let ids = <DevelopmentCell as GenerationScheme<DevelopmentIndex<'_, '_, G>>>::original_ids_for(
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

/// [#925](https://github.com/ramate-io/maybraid/issues/925) moves the mob-only
/// methods (`urbanization_selection`, `ensure_selected` / `Select`,
/// `urbanization_leaves`, `development_cells`, `leaf_bounds`, `cell_bounds`,
/// `Selection`, `Kind`) into Barking-owned traits.
impl<G: RichmondGround> UrbanSource for Richmond<G> {
	type Leaf = DevelopmentLeaf;
	type Cell = DevelopmentCell;
	type Built = BuiltDevelopment;
	type Kind = UrbanizationKind;
	type Selection = NoiseParams;
	type Select = ResMut<'static, UrbanizationIndex>;

	fn built<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a BuiltDevelopment> {
		read.developments.developments_overlapping(region)
	}

	fn built_overlapping<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<(Id, lod::gen::Version, &'a BuiltDevelopment)> {
		read.developments.developments_overlapping_tracked(region)
	}

	fn leaves<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a DevelopmentLeaf> {
		read.urbanization.filled_leaves_overlapping(region)
	}

	fn cells<'a>(
		read: &'a SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<&'a DevelopmentCell> {
		read.developments.filled_cells_overlapping(region)
	}

	fn selection(
		read: &SystemParamItem<'_, '_, Self::Read>,
	) -> (NoiseParams, Option<UrbanizationKind>) {
		(read.urbanization.noise, read.urbanization.kind)
	}

	fn leaf_aabb(leaf: &DevelopmentLeaf) -> Aabb3d {
		leaf.bounds
	}

	fn cell_aabb(cell: &DevelopmentCell) -> Aabb3d {
		cell.cell
	}

	fn select(select: &mut SystemParamItem<'_, '_, Self::Select>, region: Aabb3d) {
		let noise = select.noise;
		for extent in UrbanizationExtent::cells_overlapping(region) {
			select.ensure_selected(extent, noise);
		}
	}
}
