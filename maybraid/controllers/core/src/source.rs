//! Stable virtual-controller identities, with a transitional primary-pad alias.

use maybraid_virtual_pad_controller::VirtualPad;

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

	/// The process-wide [`VirtualPad`] is the only hardware source today.
	pub fn matches_local_pad(self, _pad: &VirtualPad) -> bool {
		matches!(self, Self::LocalPad)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn local_pad_is_the_only_source() {
		let pad = VirtualPad::default();
		assert!(InputSource::local_pad().matches_local_pad(&pad));
		assert_eq!(InputSource::default(), InputSource::LocalPad);
	}
}
