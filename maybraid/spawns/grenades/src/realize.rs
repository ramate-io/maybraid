//! Physics body, fuse, effect, and source. Presentation attaches the mesh.

use bevy::prelude::*;
use grenades::{DropLiveGrenade, SpawnThrownGrenade};

use crate::request::GrenadeSpawned;

pub fn realize_thrown_grenades(
	mut commands: Commands,
	mut throws: MessageReader<SpawnThrownGrenade>,
	mut drops: MessageReader<DropLiveGrenade>,
	mut spawned: MessageWriter<GrenadeSpawned>,
) {
	for throw in throws.read() {
		let entity = throw.spawn().realize_body(&mut commands);
		spawned.write(GrenadeSpawned::new(entity, throw.source));
	}
	for drop in drops.read() {
		let entity = drop.spawn().realize_body(&mut commands);
		spawned.write(GrenadeSpawned::new(entity, drop.source));
	}
}
