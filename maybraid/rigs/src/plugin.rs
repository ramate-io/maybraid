//! Membership → bone map → socket fulfill → bind pose.

use bevy::prelude::*;

use crate::bone_map::build_bone_maps;
use crate::member::stamp_assembly_members;
use crate::pose::maintain_bind_pose;
use crate::socket::{fulfill_socket_ref_roots, invalidate_changed_socket_ref_roots};

/// Realize loop for nested rig assemblies.
///
/// Domain plugins that spawn via LOD should order [`Self::Membership`] after
/// their fulfill drain (e.g. `LodRefreshSystems::Fulfill`).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RigSystems {
	Membership,
	InvalidateRefs,
	BoneMap,
	Fulfill,
	Pose,
}

/// Stamp membership, index bones, parent sockets, apply bind-relative pose.
pub struct RigPlugin;

impl Plugin for RigPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				RigSystems::InvalidateRefs.after(RigSystems::Membership),
				RigSystems::BoneMap.after(RigSystems::Membership),
				RigSystems::Fulfill.after(RigSystems::BoneMap).after(RigSystems::InvalidateRefs),
				RigSystems::Pose.after(RigSystems::BoneMap).after(RigSystems::InvalidateRefs),
			),
		);
		app.configure_sets(PostUpdate, RigSystems::Pose.before(TransformSystems::Propagate));
		app.add_systems(Update, stamp_assembly_members.in_set(RigSystems::Membership));
		app.add_systems(
			Update,
			invalidate_changed_socket_ref_roots.in_set(RigSystems::InvalidateRefs),
		);
		app.add_systems(Update, build_bone_maps.in_set(RigSystems::BoneMap));
		app.add_systems(Update, fulfill_socket_ref_roots.in_set(RigSystems::Fulfill));
		app.add_systems(Update, maintain_bind_pose.in_set(RigSystems::Pose));
		app.add_systems(PostUpdate, maintain_bind_pose.in_set(RigSystems::Pose));
	}
}
