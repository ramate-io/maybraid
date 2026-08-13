//! Nested [`LodScene`] host registration for character recipes.

use bevy::prelude::*;
use lod::{add_lod_refresh_chunk_for, LodRefreshSystems, LodScene};

use crate::anim::{prepare_anim_mailbox, tick_anim_mailbox};
use crate::components::{CharacterComponents, ComponentsOnly};
use crate::member::stamp_character_members;
use crate::pose::maintain_resolved_pose;
use crate::skin::invalidate_changed_skin_ref_roots;
use crate::socket::invalidate_changed_socket_ref_roots;

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

/// Membership, pose, and animation mailbox for nested character hosts.
///
/// [`Self::Membership`] walks [`ChildOf`] to [`crate::CharacterRoot`] after LOD
/// fulfill. [`Self::InvalidateRefs`] drops `*Applied` when socket/skin refs
/// change. [`Self::Pose`] applies [`crate::ActiveRigPose`]. [`Self::Anim`]
/// prepares and ticks the clip mailbox.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterHostSystems {
	Membership,
	InvalidateRefs,
	Pose,
	Anim,
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
				CharacterHostSystems::Pose.after(CharacterHostSystems::InvalidateRefs),
				CharacterHostSystems::Anim.after(CharacterHostSystems::Pose),
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
		app.add_systems(Update, maintain_resolved_pose.in_set(CharacterHostSystems::Pose));
		app.add_systems(
			Update,
			(prepare_anim_mailbox, tick_anim_mailbox.after(prepare_anim_mailbox))
				.in_set(CharacterHostSystems::Anim),
		);
		app.add_systems(PostUpdate, maintain_resolved_pose.in_set(CharacterHostSystems::Pose));
	}
}
