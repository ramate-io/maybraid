//! Nested [`LodScene`] host registration for firearm recipes.

use bevy::prelude::*;
use lod::{add_lod_refresh_chunk_for, LodRefreshSystems, LodScene};

use crate::member::{build_rig_bone_map, stamp_firearm_members};
use crate::nodes::{PartNode, RigNode};
use crate::socket::{fulfill_socket_ref_roots, invalidate_changed_socket_ref_roots};
use crate::{ComponentsOnly, FirearmComponents};

/// Register chunk fulfill for a structural [`ComponentsOnly<C>`] host.
pub fn add_firearm_components_host<C>(app: &mut App)
where
	C: FirearmComponents + Send + Sync + 'static,
	ComponentsOnly<C>: Component + LodScene,
{
	add_lod_refresh_chunk_for::<ComponentsOnly<C>>(app);
}

/// Realize loop for nested firearm hosts: membership, bone map, socket fulfill.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FirearmHostSystems {
	Membership,
	InvalidateRefs,
	BoneMap,
	Fulfill,
}

/// Nested firearm hosts are spawned as LodScene; membership is stamped after fulfill.
pub struct FirearmComponentsPlugin;

impl Plugin for FirearmComponentsPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				FirearmHostSystems::Membership.after(LodRefreshSystems::Fulfill),
				FirearmHostSystems::InvalidateRefs.after(FirearmHostSystems::Membership),
				FirearmHostSystems::BoneMap.after(FirearmHostSystems::Membership),
				FirearmHostSystems::Fulfill
					.after(FirearmHostSystems::BoneMap)
					.after(FirearmHostSystems::InvalidateRefs),
			),
		);
		app.add_systems(Update, stamp_firearm_members.in_set(FirearmHostSystems::Membership));
		app.add_systems(
			Update,
			invalidate_changed_socket_ref_roots.in_set(FirearmHostSystems::InvalidateRefs),
		);
		app.add_systems(Update, build_rig_bone_map.in_set(FirearmHostSystems::BoneMap));
		app.add_systems(Update, fulfill_socket_ref_roots.in_set(FirearmHostSystems::Fulfill));
		add_lod_refresh_chunk_for::<RigNode>(app);
		add_lod_refresh_chunk_for::<PartNode>(app);
	}
}
