//! Shared Avian collision memberships for Maybraid LOD / terrain / motion.
//!
//! Contact pairs require both sides' **memberships** to pass the other's **filters**
//! ([`CollisionLayers::interacts_with`]). [`LayerMask::NONE`] is empty (no layers),
//! not “all” — use [`LayerMask::ALL`] for the inclusive-everything mask.
//!
//! | Layer | Role | Typical filters |
//! | --- | --- | --- |
//! | [`PhysicsInteractionLayer::Host`] | LOD query volumes | none (query-only) |
//! | [`PhysicsInteractionLayer::Fixed`] | Terrain / buildings | [`Animated`](PhysicsInteractionLayer::Animated) |
//! | [`PhysicsInteractionLayer::Animated`] | Characters / movers | [`Fixed`](PhysicsInteractionLayer::Fixed) |

use avian3d::prelude::{Collider, CollisionLayers, LayerMask, PhysicsLayer};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::AvianLodSceneBoundsMarshaller;
use lod::LodSceneBoundsMarshaller;

/// Maybraid physics layers (Avian bit 0 is reserved as the engine default layer).
#[derive(PhysicsLayer, Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhysicsInteractionLayer {
	/// Avian default layer (bit 0). Prefer tagging new colliders explicitly.
	#[default]
	Default,
	/// LOD scene-host volumes (`AvianLodSceneBoundsMarshaller`). Query-only contacts.
	Host,
	/// Static world geometry (terrain trimeshes, buildings).
	Fixed,
	/// Dynamic movers (characters, props that should rest on Fixed).
	Animated,
}

impl PhysicsInteractionLayer {
	/// LOD host: member of [`Host`](Self::Host), contacts nobody.
	pub fn host_layers() -> CollisionLayers {
		CollisionLayers::new(Self::Host, LayerMask::NONE)
	}

	/// Fixed geometry: member of [`Fixed`](Self::Fixed), contacts [`Animated`](Self::Animated) only.
	pub fn fixed_layers() -> CollisionLayers {
		CollisionLayers::new(Self::Fixed, Self::Animated)
	}

	/// Animated movers: member of [`Animated`](Self::Animated), contacts [`Fixed`](Self::Fixed) only.
	pub fn animated_layers() -> CollisionLayers {
		CollisionLayers::new(Self::Animated, Self::Fixed)
	}
}

/// Volume stamped on LOD hosts: cuboid (or compound) + [`PhysicsInteractionLayer::host_layers`].
#[derive(Bundle)]
pub struct AvianLodHostVolume {
	pub collider: Collider,
	pub layers: CollisionLayers,
}

impl LodSceneBoundsMarshaller for AvianLodSceneBoundsMarshaller {
	type Volume = AvianLodHostVolume;

	fn volume_from_bounds(bounds: Aabb3d) -> Self::Volume {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let center = (min + max) * 0.5;
		let size = (max - min).max(Vec3::splat(1e-3));
		let cuboid = Collider::cuboid(size.x, size.y, size.z);
		let collider = if center.length_squared() <= 1e-8 {
			cuboid
		} else {
			Collider::compound(vec![(center, Quat::IDENTITY, cuboid)])
		};
		AvianLodHostVolume { collider, layers: PhysicsInteractionLayer::host_layers() }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn host_contacts_nobody() {
		let host = PhysicsInteractionLayer::host_layers();
		let fixed = PhysicsInteractionLayer::fixed_layers();
		let animated = PhysicsInteractionLayer::animated_layers();
		assert!(!host.interacts_with(host));
		assert!(!host.interacts_with(fixed));
		assert!(!host.interacts_with(animated));
		assert!(!fixed.interacts_with(host));
	}

	#[test]
	fn fixed_does_not_contact_fixed() {
		let fixed = PhysicsInteractionLayer::fixed_layers();
		assert!(!fixed.interacts_with(fixed));
	}

	#[test]
	fn fixed_contacts_animated_both_ways() {
		let fixed = PhysicsInteractionLayer::fixed_layers();
		let animated = PhysicsInteractionLayer::animated_layers();
		assert!(fixed.interacts_with(animated));
		assert!(animated.interacts_with(fixed));
	}
}
