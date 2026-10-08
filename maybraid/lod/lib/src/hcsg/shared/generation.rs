//! [`generation<B, T>`]: keeps `T` warm in storage within `B`'s bounds.

use std::marker::PhantomData;

use bevy::ecs::system::StaticSystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{App, IntoScheduleConfigs, Local, Plugin, Res, Update};

use super::bounds::HcsgBounds;
use super::context::GenerationScheme;
use super::demand::{HcsgDemand, SubscriptionId};
use super::runtime::{ensure_runtime, HcsgSystems};

/// Per-system request state for [`generation`].
pub struct Generated<T> {
	subscription: Option<SubscriptionId>,
	requested: Option<Aabb3d>,
	_t: PhantomData<fn() -> T>,
}

impl<T> Default for Generated<T> {
	fn default() -> Self {
		Self { subscription: None, requested: None, _t: PhantomData }
	}
}

/// Fire and forget: replaces its subscription when `B::inner` changes or the
/// subscription is dropped (an epoch change). Never reads results.
pub fn generation<B: HcsgBounds, T: GenerationScheme>(
	bounds: StaticSystemParam<B::Param>,
	demand: Res<HcsgDemand>,
	mut state: Local<Generated<T>>,
) {
	let inner = B::inner(&bounds);
	let live = match state.subscription {
		Some(subscription) => match demand.try_is_live(subscription) {
			Ok(live) => live,
			Err(_) => return,
		},
		None => false,
	};
	if inner == state.requested && live == inner.is_some() {
		return;
	}
	state.requested = inner;
	state.subscription = match inner {
		Some(region) => Some(demand.subscribe::<T>(state.subscription, region, B::focus(&bounds))),
		None => {
			if let Some(subscription) = state.subscription.take() {
				demand.unsubscribe(subscription);
			}
			None
		}
	};
}

/// Keeps `T` generated within `B` without presenting it. A presented type
/// needs none: presentation requests its own generation.
pub struct GenerationPlugin<B, T>(PhantomData<fn() -> (B, T)>);

impl<B, T> Default for GenerationPlugin<B, T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<B: HcsgBounds, T: GenerationScheme> Plugin for GenerationPlugin<B, T> {
	fn build(&self, app: &mut App) {
		ensure_runtime(app);
		app.add_systems(Update, generation::<B, T>.in_set(HcsgSystems));
	}
}
