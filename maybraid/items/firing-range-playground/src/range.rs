//! Ground pad under the Les Halles stack (courtyard is open to this slab).

use bevy::prelude::*;
use les_halles_arena::spawn_pad;

pub(crate) fn setup_range(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	spawn_pad(&mut commands, &mut meshes, &mut materials);
}
