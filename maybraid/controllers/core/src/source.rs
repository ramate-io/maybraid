//! Stable virtual-controller identities, with a transitional primary-pad alias.

/// Who produced a sample. Entity generations prevent source identity reuse.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InputSource {
	#[default]
	LocalPad,
	/// Stable identity of an entity-owned virtual controller.
	VirtualPad(bevy::prelude::Entity),
}

impl InputSource {
	pub const fn local_pad() -> Self {
		Self::LocalPad
	}

	pub const fn virtual_pad(entity: bevy::prelude::Entity) -> Self {
		Self::VirtualPad(entity)
	}

	pub const fn entity(self) -> Option<bevy::prelude::Entity> {
		match self {
			Self::VirtualPad(entity) => Some(entity),
			Self::LocalPad => None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn canonical_sources_keep_their_controller_identity() {
		let entity = bevy::prelude::Entity::from_bits(7);
		assert_eq!(InputSource::virtual_pad(entity).entity(), Some(entity));
		assert_eq!(InputSource::local_pad().entity(), None);
		assert_eq!(InputSource::default(), InputSource::LocalPad);
	}
}
