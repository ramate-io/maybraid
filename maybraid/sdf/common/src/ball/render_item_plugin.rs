use super::Ball;
use bevy::prelude::*;
use render_item::mesh::{fetch_meshes, handle::MeshHandle};
use std::marker::PhantomData;

pub struct BallRenderItemPlugin<M: Material> {
	_material: PhantomData<M>,
}

impl<M: Material> Plugin for BallRenderItemPlugin<M> {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, fetch_meshes::<MeshHandle<Ball>, M>);
	}
}
