//! Registers [`MeshDispatchPlugin`](render_item::mesh::MeshDispatchPlugin) for [`MeshHandle`](render_item::mesh::handle::MeshHandle)`<`[`CrookCylinder`](super::CrookCylinder)`>`.

use super::CrookCylinder;
use bevy::prelude::*;
use render_item::mesh::{handle::MeshHandle, MeshDispatchPlugin};
use std::marker::PhantomData;

pub struct CrookCylinderRenderItemPlugin<M: Material> {
	_material: PhantomData<M>,
}

impl<M: Material> Default for CrookCylinderRenderItemPlugin<M> {
	fn default() -> Self {
		Self { _material: PhantomData }
	}
}

impl<M: Material> Plugin for CrookCylinderRenderItemPlugin<M> {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MeshDispatchPlugin<MeshHandle<CrookCylinder>, M>>() {
			app.add_plugins(MeshDispatchPlugin::<MeshHandle<CrookCylinder>, M>::default());
		}
	}
}
