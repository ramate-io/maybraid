//! Nested [`LodScene`] host registration for firearm recipes.

use bevy::prelude::*;
use lod::{add_lod_refresh_chunk_for, LodRefreshSystems, LodScene};
use rigs::{RigPlugin, RigSystems};

use crate::nodes::{PartNode, RigNode};
use crate::{ComponentsOnly, FirearmComponents};

/// Register chunk fulfill for a structural [`ComponentsOnly<C>`] host.
pub fn add_firearm_components_host<C>(app: &mut App)
where
	C: FirearmComponents + Send + Sync + 'static,
	ComponentsOnly<C>: Component + LodScene,
{
	add_lod_refresh_chunk_for::<ComponentsOnly<C>>(app);
}

/// Shared armature realize loop ([`RigSystems`]).
pub type FirearmHostSystems = RigSystems;

/// Nested firearm hosts are spawned as LodScene; membership is stamped after fulfill.
pub struct FirearmComponentsPlugin;

impl Plugin for FirearmComponentsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<RigPlugin>() {
			app.add_plugins(RigPlugin);
		}
		app.configure_sets(
			Update,
			FirearmHostSystems::Membership.after(LodRefreshSystems::Fulfill),
		);
		add_lod_refresh_chunk_for::<RigNode>(app);
		add_lod_refresh_chunk_for::<PartNode>(app);
	}
}
