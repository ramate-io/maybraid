//! [`MobModel`]: what [`crate::Mobs`] and mob presentation read.

use bevy::app::App;
use bevy::ecs::message::Message;
use bevy::ecs::system::SystemParam;
use bevy::prelude::Entity;
use lod::gen::Id;
use terrain_layer_model::TerrainModel;

/// Named mobs over a ground model.
///
/// Only what [`crate::Mobs<Self>`]'s [`TerrainModel`] impl, mob presentation, and
/// schemes read. No method exists for a higher layer.
pub trait MobModel: Send + Sync + 'static {
	type Ground: TerrainModel;
	/// Spatial-index cell of placed scenes.
	type Cell: Send + Sync + 'static;
	/// Scheme writes. Insert stores the cell and announces [`lod::gen::LodGenerated`].
	type Writes: SystemParam + 'static;

	fn require_generation(app: &App);
}

/// Hosts a presenter spawned for one mob cell.
///
/// Training tags its squad from this message. The layer does not tag them.
#[derive(Message, Clone, Debug)]
pub struct MobCellPresented {
	pub id: Id,
	pub hosts: Vec<Entity>,
}
