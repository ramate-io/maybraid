//! Richmond's nodes in [`HcsgStorage`] and the reads layers make of them.

use bevy::math::bounding::Aabb3d;
use bevy::math::{DVec3, Vec3};
use lod::gen::{Id, Version};
use lod::hcsg::HcsgStorage;
use procedural_common::Bounds2;

use crate::artifact::BuiltDevelopment;
use crate::built::Built;
use crate::cell::DEVELOPMENT_CELL_SIZE;
use crate::developments::site::DevelopmentSite;
use crate::developments::RichmondDevelopment;
use crate::ground::RichmondGround;
use crate::pad::PadComplex;
use crate::padded::{PaddedTerrain, TerrainWithPads};

/// [`HcsgStorage`] group holding every node Richmond derives from its seeded
/// roots ([`crate::DevelopmentConfig`], [`crate::AuthoredDevelopments`]).
pub struct RichmondNodes;

/// Half height of development columns. Development cells are flat XZ
/// tiles, so their stored bounds span every terrain elevation.
const COLUMN_HALF_HEIGHT: f32 = 100_000.0;

/// `cell` extended over every elevation, so storage overlap is XZ-only.
pub fn column_bounds(cell: Aabb3d) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(cell.min.x, -COLUMN_HALF_HEIGHT, cell.min.z),
		Vec3::new(cell.max.x, COLUMN_HALF_HEIGHT, cell.max.z),
	)
}

/// Inclusive XZ overlap.
pub(crate) fn overlaps_xz(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.z <= b.max.z && a.max.z >= b.min.z
}

/// Strict XZ overlap: cells that only share an edge do not overlap.
fn overlaps_xz_strictly(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x < b.max.x && a.max.x > b.min.x && a.min.z < b.max.z && a.max.z > b.min.z
}

/// Configures Richmond's stores over ground `G`.
///
/// Nodes that read `G`'s cells also join `G::Nodes`, so a ground rebuild
/// drops them with the cells they were generated from.
pub fn register_richmond_nodes<G: RichmondGround>(storage: &mut HcsgStorage) {
	let columns = DVec3::new(
		DEVELOPMENT_CELL_SIZE as f64,
		2.0 * COLUMN_HALF_HEIGHT as f64,
		DEVELOPMENT_CELL_SIZE as f64,
	);
	storage
		.configure::<DevelopmentSite>(columns)
		.add_to_group::<RichmondNodes, DevelopmentSite>();
	storage
		.configure::<RichmondDevelopment<G>>(columns)
		.add_to_group::<RichmondNodes, RichmondDevelopment<G>>()
		.add_to_group::<G::Nodes, RichmondDevelopment<G>>();
	storage
		.configure::<Built<G>>(columns)
		.add_to_group::<RichmondNodes, Built<G>>()
		.add_to_group::<G::Nodes, Built<G>>();
	storage
		.add_to_group::<RichmondNodes, PaddedTerrain<G>>()
		.add_to_group::<G::Nodes, PaddedTerrain<G>>();
}

/// Richmond reads over [`HcsgStorage`], for ground `G`.
pub trait RichmondStorage {
	/// Pad nodes of stored filled developments affecting `region`, merged
	/// into one sample-time blend pass.
	///
	/// One complex matters for overlapping pads: sequential modulation
	/// would let later ease skirts smear earlier exact terraces.
	fn merged_pads<G: RichmondGround>(&self, region: Aabb3d) -> PadComplex;

	/// Finest stored padded surface with the greatest XZ overlap with `region`.
	fn padded_terrain_for<G: RichmondGround>(&self, region: Aabb3d) -> Option<&TerrainWithPads>;

	/// Stored built developments overlapping `region` on XZ, with versions.
	fn built_overlapping<G: RichmondGround>(
		&self,
		region: Aabb3d,
	) -> Vec<(Id, Version, &BuiltDevelopment)>;
}

impl RichmondStorage for HcsgStorage {
	fn merged_pads<G: RichmondGround>(&self, region: Aabb3d) -> PadComplex {
		let bounds = Bounds2::from_xz(region.min.x, region.min.z, region.max.x, region.max.z);
		let nodes = self
			.overlapping::<RichmondDevelopment<G>>(column_bounds(region))
			.into_iter()
			.filter_map(|id| self.get::<RichmondDevelopment<G>>(id))
			.filter(|development| {
				development.is_filled() && overlaps_xz_strictly(region, development.cell())
			})
			.flat_map(RichmondDevelopment::pad_complexes)
			.flat_map(|complex| complex.pads.iter())
			.filter(|node| node.correction_intersects(bounds))
			.cloned()
			.collect();
		PadComplex::from_nodes(nodes)
	}

	fn padded_terrain_for<G: RichmondGround>(&self, region: Aabb3d) -> Option<&TerrainWithPads> {
		let mut best: Option<(f32, f32, &TerrainWithPads)> = None;
		for id in self.overlapping::<PaddedTerrain<G>>(column_bounds(region)) {
			let Some(entry) = self.entry::<PaddedTerrain<G>>(id) else {
				continue;
			};
			let overlap_x = (region.max.x.min(entry.bounds.max.x)
				- region.min.x.max(entry.bounds.min.x))
			.max(0.0);
			let overlap_z = (region.max.z.min(entry.bounds.max.z)
				- region.min.z.max(entry.bounds.min.z))
			.max(0.0);
			let overlap = overlap_x * overlap_z;
			if overlap <= 1e-3 {
				continue;
			}
			let span = (entry.bounds.max.x - entry.bounds.min.x)
				.max(entry.bounds.max.z - entry.bounds.min.z);
			if best.is_none_or(|(best_overlap, best_span, _)| {
				overlap > best_overlap || (overlap == best_overlap && span < best_span)
			}) {
				best = Some((overlap, span, &entry.value.surface));
			}
		}
		best.map(|(_, _, terrain)| terrain)
	}

	fn built_overlapping<G: RichmondGround>(
		&self,
		region: Aabb3d,
	) -> Vec<(Id, Version, &BuiltDevelopment)> {
		self.overlapping::<Built<G>>(column_bounds(region))
			.into_iter()
			.filter_map(|id| {
				let entry = self.entry::<Built<G>>(id)?;
				overlaps_xz(region, entry.bounds).then_some((
					id,
					entry.version,
					&entry.value.development,
				))
			})
			.collect()
	}
}
