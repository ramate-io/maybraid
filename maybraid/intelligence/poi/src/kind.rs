use std::any::type_name;

/// Stable semantic category used to match POIs with user interests.
///
/// Prefer a namespaced, explicitly versioned name when this value crosses save
/// or network boundaries. [`PoiKind::of`] is convenient for process-local use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PoiKind(&'static str);

impl PoiKind {
	pub const fn new(name: &'static str) -> Self {
		Self(name)
	}

	pub fn of<T: 'static>() -> Self {
		Self(type_name::<T>())
	}

	pub const fn name(self) -> &'static str {
		self.0
	}
}

/// Stable identity for one logical point of interest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PoiId(pub u64);
