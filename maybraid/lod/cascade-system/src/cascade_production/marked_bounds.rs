//! [`MarkedBounds`] carries both flow discrimination (`T`) and the track entity’s axis-aligned bounds.
//! [`StandardFlow`] wires it into [`crate::cascade_production::CascadeProductionSource`].

use std::marker::PhantomData;

use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::prelude::*;

use super::{CascadeProductionSource, TrackBounds};

/// Bounds of the **track** entity that drives cascade focal motion, tagged by independent flow `T`.
///
/// Using one component avoids conflicting updates between a separate bounds component and a flow marker.
#[derive(Component, PartialEq)]
pub struct MarkedBounds<T: Send + Sync + 'static> {
	pub bounds: TrackBounds,
	_marker: PhantomData<T>,
}

impl<T: Send + Sync + 'static> Clone for MarkedBounds<T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: Send + Sync + 'static> Copy for MarkedBounds<T> {}

impl<T: Send + Sync + 'static> MarkedBounds<T> {
	pub fn new(bounds: TrackBounds) -> Self {
		Self { bounds, _marker: PhantomData }
	}

	/// Bounds payload for transient signals when exact track geometry is irrelevant (tests only).
	pub fn signal_placeholder() -> Self {
		Self::new(TrackBounds::from_min_max(Vec3::ZERO, Vec3::splat(1e-6)))
	}
}

/// Type tag for [`crate::cascade_production::CascadeProduction`] / [`crate::cascade_production::CascadeProductionPlugin`].
///
/// Query shape: `(Entity, &MarkedBounds<T>)`. Default [`QueryFilter`] is **[`Changed<MarkedBounds<T>>`]**
/// so producers tick when track bounds are inserted or updated; use **`StandardFlow<T, B, ()>`**
/// for every-frame scheduling.
pub struct StandardFlow<T, B, QF = Changed<MarkedBounds<T>>>(PhantomData<(T, B, QF)>);

impl<T, B, QF> Default for StandardFlow<T, B, QF> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T, B, QF> Clone for StandardFlow<T, B, QF> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T, B, QF> Copy for StandardFlow<T, B, QF> {}

impl<T, B, QF> StandardFlow<T, B, QF> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

impl<T, B, QF> CascadeProductionSource for StandardFlow<T, B, QF>
where
	T: Send + Sync + 'static,
	B: super::RequirementBuilder + Clone + Send + Sync + 'static,
	QF: QueryFilter + Send + Sync + 'static,
{
	type PositionData = MarkedBounds<T>;
	type Builder = B;
	type QueryData = (Entity, &'static MarkedBounds<T>);
	type QueryFilter = QF;

	fn entity(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> Entity {
		item.0
	}

	fn current_position(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> TrackBounds {
		item.1.bounds
	}

	fn position_data(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> Self::PositionData {
		*item.1
	}
}
