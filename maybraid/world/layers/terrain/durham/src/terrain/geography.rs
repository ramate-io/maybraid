//! Read-only geographic facts from stored stamp and watershed records.
//!
//! This query never admits terrain, constructs geometry, or mutates storage.
//! It names each authored source once. Derived [`HydroComplexCell`] bags are
//! not sources.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use lod::gen::{Id, Version};
use lod::hcsg::{HcsgNode, HcsgStorage};
use procedural_common::Bounds2;

use crate::terrain::stamps::{
	CanyonHighPassStampCell, CanyonLowPassStampCell, MassifHighPassStampCell,
	MassifLowPassStampCell, PlateauHighPassStampCell, PlateauLowPassStampCell,
	PocketWaterHighPassStampCell, PocketWaterLowPassStampCell, RollingHighPassStampCell,
	RollingLowPassStampCell, ValleyHighPassStampCell, ValleyLowPassStampCell,
};
use crate::terrain::watersheds::{PocketWater, PocketWatersHighPass, PocketWatersLowPass};

/// Landform or hydrology family that authored a stored record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeographicFamily {
	Massif,
	Plateau,
	Canyon,
	Rolling,
	Valley,
	PocketWaterStamp,
	Watershed,
}

/// Occupancy band that produced the authored record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeographicBand {
	HighPass,
	LowPass,
}

/// Terrain vocabulary for an authored geographic source.
///
/// A massif stamp is an authored massif contribution, not proof that the
/// blended landscape contains one connected range. A streams graph is a bag of
/// corridors, not a river-connectivity graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeographicFeatureKind {
	Massif,
	Plateau,
	Canyon,
	Rolling,
	Valley,
	PocketWater,
	Lake,
	Bog,
	Stream,
	StreamsGraph,
}

/// Stable identity of one authored geographic source.
///
/// Distinguishes family, band, and the source cell. Several derived terrain or
/// hydro-complex cells may contain the same lake; they reference this identity
/// rather than acquiring names of their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GeographicFeatureId {
	pub family: GeographicFamily,
	pub band: GeographicBand,
	pub source: Id,
}

/// Conservative descriptor of one stored geographic source.
///
/// `bounds` is the stamp or watershed leaf AABB in XZ. That is a first
/// approximation, not a connected-feature hull. `anchor` is a representative
/// point for language selection and label placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeographicFeature {
	pub id: GeographicFeatureId,
	pub revision: Version,
	pub kind: GeographicFeatureKind,
	pub bounds: Bounds2,
	pub anchor: Vec2,
}

/// Stored geographic sources whose leaf bounds overlap `region`; see
/// [`crate::terrain::TerrainStorage::geographic_features_overlapping`].
pub(crate) fn geographic_features_overlapping(
	storage: &HcsgStorage,
	region: Bounds2,
) -> Vec<GeographicFeature> {
	{
		let mut out = Vec::new();
		push_stamp_features(
			&mut out,
			region,
			storage,
			GeographicFamily::Massif,
			GeographicBand::HighPass,
			GeographicFeatureKind::Massif,
			|stamp: &MassifHighPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
		);
		push_stamp_features(
			&mut out,
			region,
			storage,
			GeographicFamily::Massif,
			GeographicBand::LowPass,
			GeographicFeatureKind::Massif,
			|stamp: &MassifLowPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
		);
		push_stamp_features(
			&mut out,
			region,
			storage,
			GeographicFamily::Plateau,
			GeographicBand::HighPass,
			GeographicFeatureKind::Plateau,
			|stamp: &PlateauHighPassStampCell| {
				(!stamp.modulations.is_empty()).then_some(stamp.cell)
			},
		);
		push_stamp_features(
			&mut out,
			region,
			storage,
			GeographicFamily::Plateau,
			GeographicBand::LowPass,
			GeographicFeatureKind::Plateau,
			|stamp: &PlateauLowPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
		);
		push_stamp_features(
			&mut out,
			region,
			storage,
			GeographicFamily::Canyon,
			GeographicBand::HighPass,
			GeographicFeatureKind::Canyon,
			|stamp: &CanyonHighPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
		);
		push_stamp_features(
			&mut out,
			region,
			storage,
			GeographicFamily::Canyon,
			GeographicBand::LowPass,
			GeographicFeatureKind::Canyon,
			|stamp: &CanyonLowPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
		);
		push_stamp_features(
			&mut out,
			region,
			storage,
			GeographicFamily::Rolling,
			GeographicBand::HighPass,
			GeographicFeatureKind::Rolling,
			|stamp: &RollingHighPassStampCell| {
				(!stamp.modulations.is_empty()).then_some(stamp.cell)
			},
		);
		push_stamp_features(
			&mut out,
			region,
			storage,
			GeographicFamily::Rolling,
			GeographicBand::LowPass,
			GeographicFeatureKind::Rolling,
			|stamp: &RollingLowPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
		);
		push_stamp_features(
			&mut out,
			region,
			storage,
			GeographicFamily::Valley,
			GeographicBand::HighPass,
			GeographicFeatureKind::Valley,
			|stamp: &ValleyHighPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
		);
		push_stamp_features(
			&mut out,
			region,
			storage,
			GeographicFamily::Valley,
			GeographicBand::LowPass,
			GeographicFeatureKind::Valley,
			|stamp: &ValleyLowPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
		);
		push_stamp_features(
			&mut out,
			region,
			storage,
			GeographicFamily::PocketWaterStamp,
			GeographicBand::HighPass,
			GeographicFeatureKind::PocketWater,
			|stamp: &PocketWaterHighPassStampCell| {
				(!stamp.modulations.is_empty()).then_some(stamp.cell)
			},
		);
		push_stamp_features(
			&mut out,
			region,
			storage,
			GeographicFamily::PocketWaterStamp,
			GeographicBand::LowPass,
			GeographicFeatureKind::PocketWater,
			|stamp: &PocketWaterLowPassStampCell| {
				(!stamp.modulations.is_empty()).then_some(stamp.cell)
			},
		);
		push_watershed_features::<PocketWatersHighPass>(&mut out, region, storage);
		push_watershed_features::<PocketWatersLowPass>(&mut out, region, storage);
		out
	}
}

fn push_stamp_features<T: HcsgNode>(
	out: &mut Vec<GeographicFeature>,
	region: Bounds2,
	storage: &HcsgStorage,
	family: GeographicFamily,
	band: GeographicBand,
	kind: GeographicFeatureKind,
	cell_if_occupied: impl Fn(&T) -> Option<Aabb3d>,
) {
	for (id, entry) in storage.store::<T>().into_iter().flat_map(|store| store.iter()) {
		let Some(cell) = cell_if_occupied(&entry.value) else {
			continue;
		};
		let bounds = bounds2_from_aabb(cell);
		if !xz_overlaps(region, bounds) {
			continue;
		}
		out.push(GeographicFeature {
			id: GeographicFeatureId { family, band, source: id },
			revision: entry.version,
			kind,
			bounds,
			anchor: bounds.center(),
		});
	}
}

fn push_watershed_features<T>(
	out: &mut Vec<GeographicFeature>,
	region: Bounds2,
	storage: &HcsgStorage,
) where
	T: AuthoredPocketWaters + HcsgNode,
{
	for (id, entry) in storage.store::<T>().into_iter().flat_map(|store| store.iter()) {
		let authored = entry.value.authored();
		let Some((kind, bounds, anchor)) = authored_geography(authored, entry.value.cell()) else {
			continue;
		};
		if !xz_overlaps(region, bounds) {
			continue;
		};
		out.push(GeographicFeature {
			id: GeographicFeatureId {
				family: GeographicFamily::Watershed,
				band: entry.value.band(),
				source: id,
			},
			revision: entry.version,
			kind,
			bounds,
			anchor,
		});
	}
}

trait AuthoredPocketWaters {
	fn authored(&self) -> &PocketWater;
	fn cell(&self) -> Aabb3d;
	fn band(&self) -> GeographicBand;
}

impl AuthoredPocketWaters for PocketWatersHighPass {
	fn authored(&self) -> &PocketWater {
		&self.authored
	}

	fn cell(&self) -> Aabb3d {
		self.cell
	}

	fn band(&self) -> GeographicBand {
		GeographicBand::HighPass
	}
}

impl AuthoredPocketWaters for PocketWatersLowPass {
	fn authored(&self) -> &PocketWater {
		&self.authored
	}

	fn cell(&self) -> Aabb3d {
		self.cell
	}

	fn band(&self) -> GeographicBand {
		GeographicBand::LowPass
	}
}

fn authored_geography(
	authored: &PocketWater,
	_leaf: Aabb3d,
) -> Option<(GeographicFeatureKind, Bounds2, Vec2)> {
	match authored {
		PocketWater::Empty => None,
		PocketWater::Lake(lake) => Some((GeographicFeatureKind::Lake, lake.bounds, lake.center)),
		PocketWater::Bog(bog) => Some((GeographicFeatureKind::Bog, bog.bounds, bog.center)),
		PocketWater::Stream(stream) => {
			let anchor = stream
				.path
				.get(stream.path.len() / 2)
				.copied()
				.unwrap_or_else(|| stream.bounds.center());
			Some((GeographicFeatureKind::Stream, stream.bounds, anchor))
		}
		PocketWater::StreamsGraph(graph) => {
			Some((GeographicFeatureKind::StreamsGraph, graph.bounds, graph.bounds.center()))
		}
	}
}

fn bounds2_from_aabb(cell: Aabb3d) -> Bounds2 {
	Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z)
}

fn xz_overlaps(a: Bounds2, b: Bounds2) -> bool {
	a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::terrain::index::TerrainStorage;
	use crate::terrain::stamps::MassifHighPassStampCell;
	use crate::terrain::watersheds::{HydroComplexCell, PocketWatersHighPass, WatershedBandPass};
	use std::sync::Arc;
	use terrain_stamps::{CircleRegion, Region2D, RegionAffineModulation, StampModulation};
	use terrain_watersheds::{HydroComplex, Lake, LakeParams};

	fn cell(min_x: f32, min_z: f32, max_x: f32, max_z: f32) -> Aabb3d {
		Aabb3d::from_min_max(
			bevy::math::Vec3::new(min_x, -1.0, min_z),
			bevy::math::Vec3::new(max_x, 1.0, max_z),
		)
	}

	fn dummy_modulation() -> StampModulation {
		StampModulation::Affine(RegionAffineModulation::new(
			Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius: 8.0 }),
			1.0,
			0.0,
			4.0,
			8.0,
		))
	}

	fn insert_stamp(
		store: &mut HcsgStorage,
		id: Id,
		stamp_cell: Aabb3d,
		occupied: bool,
	) -> Version {
		store.insert(
			id,
			MassifHighPassStampCell {
				cell: stamp_cell,
				modulations: if occupied { vec![dummy_modulation()] } else { Vec::new() },
			},
			stamp_cell,
		)
	}

	fn insert_lake(store: &mut HcsgStorage, id: Id, lake_cell: Aabb3d, lake: Lake) -> Version {
		store.insert(
			id,
			PocketWatersHighPass {
				cell: lake_cell,
				band: WatershedBandPass::High,
				authored: PocketWater::Lake(lake),
			},
			lake_cell,
		)
	}

	#[test]
	fn empty_stamp_modulations_are_not_geographic_sources() -> anyhow::Result<()> {
		let mut store = HcsgStorage::default();
		let occupied = cell(0.0, 0.0, 100.0, 100.0);
		let empty = cell(200.0, 200.0, 300.0, 300.0);
		let version = insert_stamp(&mut store, Id::from_cell(occupied), occupied, true);
		insert_stamp(&mut store, Id::from_cell(empty), empty, false);

		let found: Vec<_> = store
			.geographic_features_overlapping(Bounds2::from_xz(-10.0, -10.0, 400.0, 400.0))
			.collect();
		anyhow::ensure!(found.len() == 1, "expected one occupied stamp, got {}", found.len());
		anyhow::ensure!(found[0].kind == GeographicFeatureKind::Massif);
		anyhow::ensure!(found[0].id.family == GeographicFamily::Massif);
		anyhow::ensure!(found[0].id.band == GeographicBand::HighPass);
		anyhow::ensure!(found[0].id.source == Id::from_cell(occupied));
		anyhow::ensure!(found[0].revision == version);
		Ok(())
	}

	#[test]
	fn one_authored_lake_is_named_once_across_derived_hydro_cells() -> anyhow::Result<()> {
		let mut store = HcsgStorage::default();
		let lake_cell = cell(0.0, 0.0, 400.0, 400.0);
		let lake_bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let lake = Lake::from_bounds(lake_bounds, 7, LakeParams::default(), None)
			.ok_or_else(|| anyhow::anyhow!("authored lake"))?;
		let lake_id = Id::from_cell(lake_cell);
		let lake_version = insert_lake(&mut store, lake_id, lake_cell, lake);

		let left = cell(0.0, 0.0, 200.0, 400.0);
		let right = cell(200.0, 0.0, 400.0, 400.0);
		for derived in [left, right] {
			store.insert(
				Id::from_cell(derived),
				HydroComplexCell {
					cell: derived,
					complex: Arc::new(HydroComplex::new(
						Bounds2::from_xz(
							derived.min.x,
							derived.min.z,
							derived.max.x,
							derived.max.z,
						),
						1,
					)),
				},
				derived,
			);
		}

		let found: Vec<_> = store
			.geographic_features_overlapping(Bounds2::from_xz(-10.0, -10.0, 410.0, 410.0))
			.collect();
		let lakes: Vec<_> = found
			.iter()
			.filter(|feature| feature.kind == GeographicFeatureKind::Lake)
			.collect();
		anyhow::ensure!(lakes.len() == 1, "derived hydro cells must not mint extra lakes");
		anyhow::ensure!(lakes[0].id.source == lake_id);
		anyhow::ensure!(lakes[0].id.family == GeographicFamily::Watershed);
		anyhow::ensure!(lakes[0].revision == lake_version);
		Ok(())
	}

	#[test]
	fn query_does_not_admit_or_mutate_storage() -> anyhow::Result<()> {
		let store = HcsgStorage::default();
		let before = store.latest_version();
		let count = store
			.geographic_features_overlapping(Bounds2::from_xz(0.0, 0.0, 10.0, 10.0))
			.count();
		anyhow::ensure!(count == 0);
		anyhow::ensure!(store.latest_version() == before);
		Ok(())
	}
}
