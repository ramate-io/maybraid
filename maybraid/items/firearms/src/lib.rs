//! Maybraid firearm recipes: kit assembly over [`firearms_components`].
//!
//! [`FirearmKit`] is the character-recipe analogue: a required body plus optional
//! barrel / trigger-box / grip / stock / sight, socketed onto the matching bones.
//! [`FirearmConcept`] is a named preset of that kit.

pub mod cadence;
pub mod concepts;
pub mod energy;
pub mod impact;
pub mod kit;
pub mod muzzle_flame;
pub mod parts;
pub mod plugin;
pub mod pose;
pub mod projectiles;
pub mod sound;

pub use ::projectiles::{BoltSpec, BulletSpec, Flight, PenetrationCost, ProjectileSource};
pub use cadence::{
	CATCH_UP_SHOTS, Cadence, FireControl, WeaponFired, WeaponRecoil, advance_shot_clock,
	catch_up_limit,
};
pub use concepts::FirearmConcept;
pub use energy::{
	EnergyKind, EnergyMaterial, EnergyMaterialLib, EnergyMaterialPlugin, EnergyMaterialRefCache,
	KIND_HEX, KIND_PULSE, KIND_TAIL, LASER_HEX_RECIPE, LASER_PULSE_RECIPE, LASER_TAIL_RECIPE,
	init_energy_material_caches, is_energy_recipe, laser_hex_ref, laser_pulse_ref, laser_tail_ref,
};
pub use firearms_components::{
	ActiveRigPose, AssetPath, BindPose, BoneMap, BoneScale, ComponentsOnly, FirearmComponents,
	FirearmComponentsPlugin, FirearmHostSystems, FirearmMembers, FirearmPartSlot, FirearmRoot,
	Layer, Layers, MemberOf, PartNode, RECEIVER_LANDMARKS, ResolvedRigPose, RigNode, RigPoseLayer,
	RigRoot, SocketRef, SocketRefApplied, SocketRefRoot, add_firearm_components_host,
	assembled_firearm_bounds, firearm_bounds, firearm_preview_camera, spawn_firearm_components,
};
pub use kit::FirearmKit;
pub use muzzle_flame::{
	MUZZLE_FLAME_RECIPE, MuzzleFlameMaterial, MuzzleFlameMaterialLib, MuzzleFlameMaterialPlugin,
	MuzzleFlameMaterialRefCache, init_muzzle_flame_caches, muzzle_flame_ref,
};
pub use parts::{
	BarrelMesh, BodyMesh, GripMesh, IRON_SIGHT_FOV, KitBone, SightMesh, StockMesh, TriggerBoxMesh,
	fov_at_zoom,
};
pub use plugin::FirearmHostsPlugin;
pub use pose::{BoneFit, FirearmPose, aim_plus_x};
pub use projectiles::{
	BARREL_REST_LENGTH, FireOnTrigger, FirearmWeaponSystems, FirearmWeaponsPlugin, LaserBeam,
	LaserSpec, ProjectileLoad, Weapon, WeaponTrigger, WeaponsArmed, muzzle_world,
};
pub use sound::{
	FIRE_LISTENER_GAP, FIRE_SPATIAL_RADIUS, FIRE_SPATIAL_SCALE, FIRE_VOLUME, FIZZ_SPATIAL_RADIUS,
	FIZZ_SPATIAL_SCALE, FIZZ_VOLUME, FirearmFireSounds, FlightFizz, HAMMER_SPATIAL_RADIUS,
	HAMMER_SPATIAL_SCALE, HAMMER_VOLUME, IMPACT_SPATIAL_RADIUS, IMPACT_SPATIAL_SCALE,
	IMPACT_VOLUME, LASER_MUZZLE_LOCAL, WEAPON_FIRE, WEAPON_FIZZ, WEAPON_HAMMER, WEAPON_IMPACT,
};

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
