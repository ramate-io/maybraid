//! Authored palette slots and material resolution ([RFC-183 §3.4.5.1]).

#[cfg(feature = "render")]
use bevy::prelude::Color;
#[cfg(feature = "render")]
use procedural_common::{NoiseConfig, NoiseParams};

/// Named color-range slots for one grove bucket.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PaletteMix {
	pub slots: Vec<PaletteSlot>,
}

impl PaletteMix {
	pub fn from_slots(slots: Vec<PaletteSlot>) -> Self {
		Self { slots }
	}

	/// Noisily pick one authored endpoint color from all slots.
	#[cfg(feature = "render")]
	pub fn pick_color(&self, seed: i32) -> Option<Color> {
		let mut colors = Vec::new();
		for slot in &self.slots {
			if let Some(color) = slot.start.resolve() {
				colors.push(color);
			}
			if let Some(color) = slot.end.resolve() {
				colors.push(color);
			}
		}
		if colors.is_empty() {
			return None;
		}
		let noise = NoiseConfig::new(NoiseParams::from_scalar(seed as f32, 1.0, 1.0, 1));
		let unit = noise.sample_unit_1d(17.0);
		let index = (unit * colors.len() as f32).floor() as usize;
		colors.get(index.min(colors.len().saturating_sub(1))).copied()
	}
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

impl PaletteColor {
	#[cfg(feature = "render")]
	pub fn resolve(self) -> Option<Color> {
		resolve_palette_color(self.0)
	}
}

/// Apply an authored [`PaletteMix`] to a material template.
#[cfg(feature = "render")]
pub trait WithPalette: Sized {
	fn with_palette(base: Self, palette: &PaletteMix, seed: i32) -> Self;
}

/// Resolve RFC-style palette tokens to playground sRGB colors.
#[cfg(feature = "render")]
pub fn resolve_palette_color(name: &str) -> Option<Color> {
	Some(match name {
		"deep_green" => Color::srgb(0.12, 0.35, 0.18),
		"wet_green" => Color::srgb(0.18, 0.48, 0.28),
		"dark_green" => Color::srgb(0.10, 0.28, 0.16),
		"emerald_green" => Color::srgb(0.15, 0.55, 0.35),
		"blue_green" => Color::srgb(0.20, 0.52, 0.42),
		"fresh_green" => Color::srgb(0.25, 0.62, 0.32),
		"yellow_green" => Color::srgb(0.45, 0.62, 0.28),
		"pale_straw" => Color::srgb(0.72, 0.68, 0.42),
		"dry_green" => Color::srgb(0.38, 0.48, 0.28),
		"light_green" => Color::srgb(0.42, 0.58, 0.35),
		"tan_green" => Color::srgb(0.48, 0.52, 0.32),
		"lush_green" => Color::srgb(0.22, 0.65, 0.30),
		"bright_green" => Color::srgb(0.30, 0.72, 0.38),
		"lime_green" => Color::srgb(0.55, 0.75, 0.28),
		"red_green" => Color::srgb(0.35, 0.48, 0.22),
		"copper_red" => Color::srgb(0.62, 0.38, 0.22),
		"dark_red" => Color::srgb(0.42, 0.18, 0.12),
		"young_green" => Color::srgb(0.38, 0.72, 0.32),
		"palm_bark" => Color::srgb(0.45, 0.32, 0.18),
		"tan_bark" => Color::srgb(0.58, 0.48, 0.32),
		"green_stem" => Color::srgb(0.28, 0.52, 0.22),
		"wet_brown" => Color::srgb(0.32, 0.24, 0.14),
		"young_palm_bark" => Color::srgb(0.52, 0.42, 0.28),
		_ => return None,
	})
}

#[cfg(feature = "render")]
impl WithPalette for bevy::prelude::StandardMaterial {
	fn with_palette(mut base: Self, palette: &PaletteMix, seed: i32) -> Self {
		if let Some(color) = palette.pick_color(seed) {
			base.base_color = color;
		}
		base.double_sided = true;
		base
	}
}

/// After lower-order [`RenderItem::spawn_render_items`], patch spawned entities with a
/// per-placement palette-resolved material asset.
#[cfg(feature = "render")]
pub fn patch_spawned_leaf_material<M: bevy::prelude::Material + WithPalette + Default>(
	entities: &[bevy::prelude::Entity],
	palette: &PaletteMix,
	seed: i32,
	commands: &mut bevy::prelude::Commands,
) {
	if entities.is_empty() {
		return;
	}
	let palette = palette.clone();
	let entities = entities.to_vec();
	commands.queue(move |world: &mut bevy::prelude::World| {
		let material = M::with_palette(M::default(), &palette, seed);
		let handle = world.resource_mut::<bevy::prelude::Assets<M>>().add(material);
		for entity in entities {
			world
				.entity_mut(entity)
				.insert(bevy::prelude::MeshMaterial3d(handle.clone()));
		}
	});
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[cfg(feature = "render")]
	#[test]
	fn pick_color_is_deterministic() -> Result<()> {
		let palette = PaletteMix::from_slots(vec![PaletteSlot {
			start: PaletteColor("deep_green"),
			end: PaletteColor("wet_green"),
		}]);
		let a = palette.pick_color(42);
		let b = palette.pick_color(42);
		assert_eq!(a, b);
		assert!(a.is_some());
		Ok(())
	}

	#[cfg(feature = "render")]
	#[test]
	fn standard_material_with_palette_sets_base_color() -> Result<()> {
		use bevy::prelude::StandardMaterial;

		let palette = PaletteMix::from_slots(vec![PaletteSlot {
			start: PaletteColor("deep_green"),
			end: PaletteColor("wet_green"),
		}]);
		let material = StandardMaterial::with_palette(StandardMaterial::default(), &palette, 7);
		assert_eq!(material.base_color, PaletteColor("deep_green").resolve().unwrap());
		Ok(())
	}
}
