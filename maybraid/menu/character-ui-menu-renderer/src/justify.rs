use bevy::prelude::*;

/// How the sink packs rows inside the host parent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MenuJustify {
	#[default]
	Left,
	Right,
}

impl MenuJustify {
	pub fn content(self) -> JustifyContent {
		match self {
			Self::Left => JustifyContent::FlexStart,
			Self::Right => JustifyContent::FlexEnd,
		}
	}

	pub fn items(self) -> AlignItems {
		match self {
			Self::Left => AlignItems::FlexStart,
			Self::Right => AlignItems::FlexEnd,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::MenuJustify;

	#[test]
	fn default_is_left() {
		assert_eq!(MenuJustify::default(), MenuJustify::Left);
	}
}
