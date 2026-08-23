//! Fold [`LodSceneRefreshLevel`] impulses and write host [`LodSceneLevel`].

use std::collections::HashMap;

use bevy::prelude::*;

use crate::scene::host::LodSceneHost;
use crate::scene::level::LodSceneLevel;

use super::super::ensure_refresh_core;
use super::super::levels::LodSceneRefreshLevel;

/// Keep the max [`LodSceneLevel`] per entity, then write once onto any host.
///
/// The bus is already untyped (`entity` + `level`). A per-`T` fold would re-read
/// every message, allocate, and `get_mut`-miss for every other host type.
pub fn refresh_lod_host_levels(
	mut reader: MessageReader<LodSceneRefreshLevel>,
	mut hosts: Query<&mut LodSceneLevel, With<LodSceneHost>>,
) {
	if reader.is_empty() {
		return;
	}
	let mut best: HashMap<Entity, LodSceneLevel> = HashMap::new();
	for msg in reader.read() {
		best.entry(msg.entity)
			.and_modify(|level| {
				if msg.level > *level {
					*level = msg.level;
				}
			})
			.or_insert(msg.level);
	}

	for (entity, level) in best {
		let Ok(mut current) = hosts.get_mut(entity) else {
			continue;
		};
		if *current != level {
			*current = level;
		}
	}
}

/// Fold [`LodSceneRefreshLevel`] → [`LodSceneLevel`] on [`LodSceneHost`]s.
///
/// The system lives on [`crate::scene::refresh::LodRefreshCorePlugin`]; this plugin
/// only ensures that core is present (tests / standalone fold).
pub struct LodSceneRefreshEntitiesPlugin;

impl Plugin for LodSceneRefreshEntitiesPlugin {
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
	}
}
