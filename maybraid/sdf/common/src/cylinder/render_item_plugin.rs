//! Registers [`MeshDispatchPlugin`](render_item::mesh::MeshDispatchPlugin) for [`MeshHandle`](render_item::mesh::handle::MeshHandle)`<`[`TaperedCylinder`](super::TaperedCylinder)`>`.

use super::TaperedCylinder;
use bevy::prelude::*;
use render_item::mesh::{handle::MeshHandle, MeshDispatchPlugin};
use std::marker::PhantomData;

pub struct TaperedCylinderRenderItemPlugin<M: Material> {
	_material: PhantomData<M>,
}

impl<M: Material> Default for TaperedCylinderRenderItemPlugin<M> {
	fn default() -> Self {
		Self { _material: PhantomData }
	}
}

impl<M: Material> Plugin for TaperedCylinderRenderItemPlugin<M> {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MeshDispatchPlugin<MeshHandle<TaperedCylinder>, M>>() {
			app.add_plugins(MeshDispatchPlugin::<MeshHandle<TaperedCylinder>, M>::default());
		}
	}
}
