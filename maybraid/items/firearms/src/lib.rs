//! Maybraid firearm recipes: kit assembly over [`firearms_components`].
//!
//! [`FirearmConcept`] is the character-recipe analogue: a named gun that emits
//! [`FirearmComponents`] nodes (body / barrel / grip), socketed onto a receiver
//! rig when one exists.

pub mod concepts;
pub mod plugin;

pub use concepts::FirearmConcept;
pub use firearms_components::{
	add_firearm_components_host, spawn_firearm_components, AssetPath, BoneMap, ComponentsOnly,
	FirearmComponents, FirearmComponentsPlugin, FirearmHostSystems, FirearmMembers,
	FirearmPartSlot, FirearmRoot, Layer, Layers, MemberOf, PartNode, RigNode, SocketRef,
	SocketRefApplied, SocketRefRoot,
};
pub use plugin::FirearmHostsPlugin;

/// Config → inner [`FirearmComponents`] recipe.
///
/// Mirrors character recipes without clothing. Concepts implement this as
/// identity so a later config/slider layer can wrap them.
pub trait FirearmRecipe {
	type Components: FirearmComponents + Clone + Default + Send + Sync + 'static;

	fn components(&self) -> Self::Components;
}

impl FirearmRecipe for FirearmConcept {
	type Components = Self;

	fn components(&self) -> Self::Components {
		*self
	}
}
