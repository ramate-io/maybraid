use super::Ball;
use bevy::prelude::*;
use render_item::mesh::{handle::MeshHandle, MeshDispatchPlugin};
use std::marker::PhantomData;

pub struct BallRenderItemPlugin<M: Material> {
	_material: PhantomData<M>,
}

impl<M: Material> Plugin for BallRenderItemPlugin<M> {
	fn build(&self, app: &mut App) {
		// add mesh dispatch plugin if not already added
		if !app.is_plugin_added::<MeshDispatchPlugin<MeshHandle<Ball>, M>>() {
			app.add_plugins(MeshDispatchPlugin::<MeshHandle<Ball>, M>::default());
		}
	}
}
