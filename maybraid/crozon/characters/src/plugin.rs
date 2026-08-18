//! Nested [`LodScene`] host registration for character recipes.

use bevy::app::SceneSpawnerSystems;
use bevy::prelude::*;
use lod::{add_lod_refresh_chunk_for, LodRefreshSystems, LodScene};

use crate::components::{CharacterComponents, ComponentsOnly};
use crate::member::stamp_character_members;
use crate::pose::maintain_resolved_pose;
use crate::rig::build_rig_bone_map;
use crate::skin::{
	fulfill_skin_ref_roots, invalidate_changed_skin_ref_roots, prune_duplicate_part_scenes,
	remap_part_skin_to_rig,
};
use crate::socket::{fulfill_socket_ref_roots, invalidate_changed_socket_ref_roots};
use crate::terrain_pitch::prepare_character_terrain_pitch;

/// Register chunk fulfill for a structural [`ComponentsOnly<C>`] host.
///
/// [`crate::RigNode`] / [`crate::PartNode`] stamp socket, skin, pose, and material
/// refs from [`lod::LodScene::host`] / [`lod::LodScene::scene_with_level`]. Each
/// species only adds its recipe type here (typically [`crate::Clothed<T>`]).
pub fn add_character_components_host<C>(app: &mut App)
where
	C: CharacterComponents + Send + Sync + 'static,
	ComponentsOnly<C>: Component + LodScene,
{
	add_lod_refresh_chunk_for::<ComponentsOnly<C>>(app);
}

/// Realize loop for nested character hosts: membership, bone map, refs, pose.
///
/// [`Self::Membership`] walks [`ChildOf`] to [`crate::CharacterRoot`] after LOD
/// fulfill. [`Self::InvalidateRefs`] drops `*Applied` when socket/skin refs
/// change. [`Self::BoneMap`] indexes named bones. [`Self::Fulfill`] parents
/// sockets, remaps skin, and prunes duplicate GLB armatures. [`Self::Pose`]
/// applies [`crate::ActiveRigPose`]. Clip mailbox lives in
/// [`crozon_character_motion::CharacterMotionSystems::Anim`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterHostSystems {
	Membership,
	InvalidateRefs,
	BoneMap,
	Fulfill,
	Pose,
}

/// Nested character hosts are spawned as LodScene; membership is stamped after fulfill.
pub struct CharacterComponentsPlugin;

impl Plugin for CharacterComponentsPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				CharacterHostSystems::Membership.after(LodRefreshSystems::Fulfill),
				CharacterHostSystems::InvalidateRefs.after(CharacterHostSystems::Membership),
				CharacterHostSystems::BoneMap.after(CharacterHostSystems::Membership),
				CharacterHostSystems::Fulfill
					.after(CharacterHostSystems::BoneMap)
					.after(CharacterHostSystems::InvalidateRefs),
				CharacterHostSystems::Pose
					.after(CharacterHostSystems::BoneMap)
					.after(CharacterHostSystems::InvalidateRefs),
			),
		);
		app.configure_sets(
			PostUpdate,
			CharacterHostSystems::Pose.before(TransformSystems::Propagate),
		);
		app.add_systems(Update, stamp_character_members.in_set(CharacterHostSystems::Membership));
		app.add_systems(
			Update,
			(invalidate_changed_socket_ref_roots, invalidate_changed_skin_ref_roots)
				.in_set(CharacterHostSystems::InvalidateRefs),
		);
		app.add_systems(Update, build_rig_bone_map.in_set(CharacterHostSystems::BoneMap));
		app.add_systems(
			Update,
			(
				fulfill_socket_ref_roots,
				fulfill_skin_ref_roots.after(fulfill_socket_ref_roots),
				remap_part_skin_to_rig
					.after(fulfill_skin_ref_roots)
					.after(SceneSpawnerSystems::WorldInstanceSpawn),
				prune_duplicate_part_scenes.after(remap_part_skin_to_rig),
			)
				.in_set(CharacterHostSystems::Fulfill),
		);
		app.add_systems(Update, maintain_resolved_pose.in_set(CharacterHostSystems::Pose));
		app.add_systems(Update, prepare_character_terrain_pitch.after(CharacterHostSystems::Pose));
		app.add_systems(PostUpdate, maintain_resolved_pose.in_set(CharacterHostSystems::Pose));
	}
}
