//! Richmond's nodes in [`HcsgStorage`].
//!
//! Reads of stored nodes live on their schemes: [`RichmondDevelopment::merged_pads`],
//! [`PaddedTerrain::best_overlapping`], [`Built::overlapping`].

use bevy::math::bounding::Aabb3d;
use bevy::math::{DVec3, Vec3};
use durham::PlayableStreams;
use lod::hcsg::{shared, HcsgStorage};

use crate::built::Built;
use crate::cell::DEVELOPMENT_CELL_SIZE;
use crate::developments::site::DevelopmentSite;
use crate::developments::RichmondDevelopment;
use crate::ground::RichmondGround;
use crate::padded::PaddedTerrain;

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
pub(crate) fn overlaps_xz_strictly(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x < b.max.x && a.max.x > b.min.x && a.min.z < b.max.z && a.max.z > b.min.z
}

/// Index scale of development columns: one development cell per bucket.
const COLUMN_SCALE: DVec3 = DVec3::new(
	DEVELOPMENT_CELL_SIZE as f64,
	2.0 * COLUMN_HALF_HEIGHT as f64,
	DEVELOPMENT_CELL_SIZE as f64,
);

/// Configures Richmond's stores over ground `G`.
///
/// Nodes that read `G`'s cells also join `G::Nodes`, so a ground rebuild
/// drops them with the cells they were generated from.
pub fn register_richmond_nodes<G: RichmondGround>(storage: &HcsgStorage) {
	RichmondNodes::configure::<G>(storage);
}

impl RichmondNodes {
	/// Configures Richmond's stores over ground `G` in the shared storage.
	pub fn configure<G: RichmondGround>(storage: &shared::HcsgStorage) {
		storage
			.configure::<DevelopmentSite>(COLUMN_SCALE)
			.configure::<RichmondDevelopment<G>>(COLUMN_SCALE)
			.configure::<Built<G>>(COLUMN_SCALE)
			.configure::<PaddedTerrain<G>>(COLUMN_SCALE);
	}

	/// Drops every value Richmond derived over ground `G` from the shared storage.
	pub fn clear<G: RichmondGround>(storage: &shared::HcsgStorage) {
		storage.clear::<DevelopmentSite>();
		storage.clear::<RichmondDevelopment<G>>();
		storage.clear::<Built<G>>();
		storage.clear::<PaddedTerrain<G>>();
		PlayableStreams::clear::<PaddedTerrain<G>>(storage);
	}
}
