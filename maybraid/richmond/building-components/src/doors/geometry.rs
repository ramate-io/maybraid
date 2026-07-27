//! Continuous door kit geometry.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Door {
	Frame15(DoorFrame15),
	Leaf(DoorLeaf),
}

impl Door {
	pub fn frame_15() -> Self {
		Self::Frame15(DoorFrame15)
	}

	pub fn leaf() -> Self {
		Self::Leaf(DoorLeaf)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DoorFrame15;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DoorLeaf;
