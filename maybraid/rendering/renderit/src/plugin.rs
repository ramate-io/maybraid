//! Optional [`Plugin`] wiring for [`crate::dispatch::process_render_dispatches`].

use std::marker::PhantomData;

use bevy::prelude::*;

use crate::dispatch::{process_render_dispatches, RenderDispatchSource};

/// Registers [`process_render_dispatches::<S>`](crate::dispatch::process_render_dispatches) on [`Update`].
pub struct RenderDispatchPlugin<S: RenderDispatchSource> {
	_marker: PhantomData<S>,
}

impl<S: RenderDispatchSource> Default for RenderDispatchPlugin<S> {
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<S: RenderDispatchSource> Clone for RenderDispatchPlugin<S> {
	fn clone(&self) -> Self {
		Self { _marker: PhantomData }
	}
}

impl<S: RenderDispatchSource> Plugin for RenderDispatchPlugin<S> {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, process_render_dispatches::<S>);
	}
}
