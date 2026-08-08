//! Fold [`LodSceneRefreshLevel`] impulses and write host [`LodSceneLevel`].

use std::collections::HashMap;
use std::marker::PhantomData;

use bevy::prelude::*;

use crate::scene::host::LodSceneHost;
use crate::scene::level::LodSceneLevel;
use crate::scene::LodScene;

use super::super::levels::LodSceneRefreshLevel;
use super::super::{ensure_refresh_core, LodRefreshSystems};

/// Keep the max [`LodSceneLevel`] per entity, then write once onto hosts `T`.
pub fn refresh_lod_host_levels<T>(
	mut reader: MessageReader<LodSceneRefreshLevel>,
	mut hosts: Query<&mut LodSceneLevel, (With<LodSceneHost>, With<T>)>,
) where
	T: Component + LodScene + 'static,
{
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

	let mut changed = 0u32;
	for (entity, level) in best {
		let Ok(mut current) = hosts.get_mut(entity) else {
			continue;
		};
		if *current != level {
			*current = level;
			changed += 1;
		}
	}
	if changed > 0 {
		info!("[lod.refresh] refresh_lod_host_levels: changed={changed}");
	}
}

/// Fold [`LodSceneRefreshLevel`] → [`LodSceneLevel`] on hosts `T`.
pub struct LodSceneRefreshEntitiesPlugin<T>
where
	T: Component + LodScene + 'static,
{
	_marker: PhantomData<fn() -> T>,
}

impl<T> Default for LodSceneRefreshEntitiesPlugin<T>
where
	T: Component + LodScene + 'static,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<T> Plugin for LodSceneRefreshEntitiesPlugin<T>
where
	T: Component + LodScene + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.add_message::<LodSceneRefreshLevel>().add_systems(
			Update,
			refresh_lod_host_levels::<T>.in_set(LodRefreshSystems::UpdateLevels),
		);
	}
}
