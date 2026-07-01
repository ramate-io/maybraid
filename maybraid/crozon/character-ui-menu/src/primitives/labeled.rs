#[derive(Clone, Debug, PartialEq)]
pub struct Labeled<T> {
	pub label: &'static str,
	pub value: T,
}

impl<T> Labeled<T> {
	pub const fn new(label: &'static str, value: T) -> Self {
		Self { label, value }
	}
}

/// Section-style label above a block of controls (asset grids, multi-select rows).
#[derive(Clone, Debug, PartialEq)]
pub struct BlockLabeled<T> {
	pub label: &'static str,
	pub value: T,
}

impl<T> BlockLabeled<T> {
	pub const fn new(label: &'static str, value: T) -> Self {
		Self { label, value }
	}
}
