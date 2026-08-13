//! Nested [`LodScene`] host registration for character recipes.

use bevy::prelude::*;
use lod::{add_lod_refresh_chunk_for, LodRefreshSystems, LodScene};

use crate::components::{CharacterComponents, ComponentsOnly};
use crate::member::stamp_character_members;
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

/// Membership stamp and ref-invalidate for nested character hosts.
///
/// [`Self::Membership`] walks [`ChildOf`] to [`crate::CharacterRoot`] after LOD
/// fulfill and before socket/skin fulfill. [`Self::InvalidateRefs`] drops
/// `*Applied` when [`crate::SocketRefRoot`] / [`crate::SkinRefRoot`] change.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterHostSystems {
	Membership,
	InvalidateRefs,
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
			),
		);
		app.add_systems(Update, stamp_character_members.in_set(CharacterHostSystems::Membership));
		app.add_systems(
			Update,
			(invalidate_changed_socket_ref_roots, invalidate_changed_skin_ref_roots)
				.in_set(CharacterHostSystems::InvalidateRefs),
		);
	}
}
