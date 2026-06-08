//! Palette stubs and item wrappers for grove bucket payloads.

/// Placeholder until named slots are authored in macro.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PaletteMix {
	pub slots: Vec<PaletteSlot>,
}

/// One authored color range slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteSlot {
	pub start: PaletteColor,
	pub end: PaletteColor,
}

/// Named palette token until a shared chico color registry exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaletteColor(pub &'static str);

/// Wraps construction params with variant color identity.
#[derive(Debug, Clone, PartialEq)]
pub struct WithPaletteMix<T> {
	pub item: T,
	pub palette_mix: PaletteMix,
}

impl<T> WithPaletteMix<T> {
	pub fn new(item: T) -> Self {
		Self { item, palette_mix: PaletteMix::default() }
	}
}
