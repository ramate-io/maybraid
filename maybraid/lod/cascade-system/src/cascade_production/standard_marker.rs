//! [`StandardMarker`] + [`StandardBounds`] wiring and [`StandardFlow`] [`CascadeProductionSource`] impl.

use std::marker::PhantomData;

use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::prelude::*;

use super::{CascadeBounds, CascadeProductionSource};

/// Current focal bounds on the producer (`AaBb3d` as [`CascadeBounds`]).
#[derive(Component, Clone, Copy, PartialEq)]
pub struct StandardBounds(pub CascadeBounds);

/// Discriminates independent flows `T` on producer entities and signal payloads.
#[derive(Component)]
pub struct StandardMarker<T: Send + Sync + 'static>(PhantomData<T>);

impl<T: Send + Sync + 'static> Clone for StandardMarker<T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: Send + Sync + 'static> Copy for StandardMarker<T> {}

impl<T: Send + Sync + 'static> Default for StandardMarker<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T: Send + Sync + 'static> StandardMarker<T> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

/// Type tag for [`crate::cascade_production::CascadeProduction`] / [`crate::cascade_production::CascadeProductionPlugin`].
///
/// Query shape: `(Entity, &StandardBounds, &StandardMarker<T>)` with optional [`QueryFilter`] `QF`.
#[derive(Clone, Copy, Default)]
pub struct StandardFlow<T, B, QF = ()>(PhantomData<(T, B, QF)>);

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
	type PositionData = StandardMarker<T>;
	type Builder = B;
	type QueryData = (Entity, &'static StandardBounds, &'static StandardMarker<T>);
	type QueryFilter = QF;

	fn entity(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> Entity {
		item.0
	}

	fn current_position(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> CascadeBounds {
		item.1 .0
	}

	fn position_data(item: &<Self::QueryData as QueryData>::Item<'_, '_>) -> Self::PositionData {
		*item.2
	}
}
