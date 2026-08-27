//! [`PresentationScene`] — expand a stored value into lower-order constituents.
//!
//! Distinct from [`crate::scene::LodScene`]: no bands, no level roots. The
//! presenter decides existence; this trait describes how the value chunks into
//! the next grain (forest cell → grove tiles). Plant / mesh construction stays
//! on the child [`crate::scene::LodScene`].

use crate::gen::SpatialIndex;
use crate::lod_ref::LodRef;

use super::chunk::PresentationChunk;
use super::RegionPresenter;

/// A stored type that presents by expanding into constituents (no LOD banding).
pub trait PresentationScene<S>: Sized
where
	S: SpatialIndex<Self>,
{
	/// One step down (e.g. a grove-tile recipe). Not a band of `Self`.
	type Constituent: Send + Sync + 'static;

	/// Scheduling tree of constituents. Must be cheap — no plant construction.
	fn presentation_chunks(
		&self,
		spatial_index: &S,
		lod_ref: &LodRef,
	) -> PresentationChunk<Self::Constituent>;
}

/// Presenter that consumes [`PresentationScene`] constituents.
pub trait PresentationPresenter<T, S>: RegionPresenter<T, S>
where
	T: PresentationScene<S>,
	S: SpatialIndex<T>,
{
	/// Called once when fulfill begins an id (despawn previous hosts).
	fn begin_presentation(&mut self, _id: crate::gen::Id, _version: crate::gen::Version) {}

	/// Spawn or attach one constituent. Must not expand further chunks.
	fn present_constituent(
		&mut self,
		id: crate::gen::Id,
		version: crate::gen::Version,
		constituent: T::Constituent,
		lod_ref: &LodRef,
	);

	/// All constituents for this version have been presented.
	fn finish_presentation(&mut self, id: crate::gen::Id, version: crate::gen::Version);
}
