//! Maybraid firearm recipes: kit assembly over [`firearms_components`].
//!
//! [`FirearmKit`] is the character-recipe analogue: a required body plus optional
//! barrel / trigger-box / grip / stock, socketed onto the matching bones.
//! [`FirearmConcept`] is a named preset of that kit.

pub mod concepts;
pub mod kit;
pub mod parts;
pub mod plugin;

pub use concepts::FirearmConcept;
pub use firearms_components::{
	add_firearm_components_host, firearm_bounds, spawn_firearm_components, AssetPath, BoneMap,
	ComponentsOnly, FirearmComponents, FirearmComponentsPlugin, FirearmHostSystems, FirearmMembers,
	FirearmPartSlot, FirearmRoot, Layer, Layers, MemberOf, PartNode, RigNode, SocketRef,
	SocketRefApplied, SocketRefRoot, RECEIVER_LANDMARKS,
};
pub use kit::FirearmKit;
pub use parts::{BarrelMesh, BodyMesh, GripMesh, StockMesh, TriggerBoxMesh};
pub use plugin::FirearmHostsPlugin;

/// Config → inner [`FirearmComponents`] recipe.
///
/// Mirrors character recipes without clothing. Concepts expand to a [`FirearmKit`].
pub trait FirearmRecipe {
	type Components: FirearmComponents + Clone + Default + Send + Sync + 'static;

	fn components(&self) -> Self::Components;
}

impl FirearmRecipe for FirearmKit {
	type Components = Self;

	fn components(&self) -> Self::Components {
		*self
	}
}

impl FirearmRecipe for FirearmConcept {
	type Components = FirearmKit;

	fn components(&self) -> Self::Components {
		self.kit()
	}
}
