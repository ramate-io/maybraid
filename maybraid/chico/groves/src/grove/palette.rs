//! Authored palette slots and material resolution ([RFC-183 §3.4.5.1]).
//!
//! Palette mixes are authored as `const` data on each grove's cell enum; per-placement seeds
//! pick one endpoint color deterministically at spawn time.

#[cfg(feature = "render")]
use bevy::prelude::Color;
#[cfg(feature = "render")]
use bevy_math::Vec4;
#[cfg(feature = "render")]
use procedural_common::{NoiseConfig, NoiseParams};

/// Named color-range slots for one grove bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PaletteMix {
	pub slots: &'static [PaletteSlot],
}

impl PaletteMix {
	pub const fn new(slots: &'static [PaletteSlot]) -> Self {
		Self { slots }
	}

	pub const EMPTY: Self = Self { slots: &[] };

	/// Noisily pick one authored endpoint color from all slots.
	#[cfg(feature = "render")]
	pub fn pick_color(&self, seed: i32) -> Option<Color> {
		let mut colors = Vec::new();
		for slot in self.slots {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteSlot {
	pub start: PaletteColor,
	pub end: PaletteColor,
}

impl PaletteSlot {
	pub const fn new(start: &'static str, end: &'static str) -> Self {
		Self { start: PaletteColor(start), end: PaletteColor(end) }
	}
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
	fn with_palette(base: Self, palette: PaletteMix, seed: i32) -> Self;
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
		"green" => Color::srgb(0.30, 0.55, 0.28),
		"pale_green" => Color::srgb(0.55, 0.68, 0.48),
		"vibrant_yellow_green" => Color::srgb(0.52, 0.70, 0.24),
		"dry_yellow_green" => Color::srgb(0.60, 0.60, 0.32),
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
		"wet_bark" => Color::srgb(0.28, 0.22, 0.16),
		"dark_bark" => Color::srgb(0.18, 0.12, 0.08),
		"green_brown" => Color::srgb(0.32, 0.28, 0.18),
		"young_bark" => Color::srgb(0.42, 0.34, 0.22),
		"gray_brown" => Color::srgb(0.38, 0.32, 0.24),
		"red_twig" => Color::srgb(0.58, 0.22, 0.18),
		"wet_burgundy" => Color::srgb(0.38, 0.16, 0.14),
		"shrub_bark" => Color::srgb(0.34, 0.26, 0.18),
		"scrub_green" => Color::srgb(0.32, 0.48, 0.26),
		"dry_bark" => Color::srgb(0.48, 0.38, 0.26),
		"tan_brown" => Color::srgb(0.55, 0.44, 0.30),
		"straw_brown" => Color::srgb(0.62, 0.52, 0.34),
		"flower_pink" => Color::srgb(0.82, 0.48, 0.58),
		"flower_white" => Color::srgb(0.92, 0.90, 0.84),
		"leaf_green" => Color::srgb(0.28, 0.52, 0.28),
		"red_bark" => Color::srgb(0.52, 0.22, 0.16),
		"burgundy_brown" => Color::srgb(0.42, 0.18, 0.14),
		"berry_red" => Color::srgb(0.58, 0.18, 0.22),
		"berry_blue" => Color::srgb(0.22, 0.32, 0.58),
		"orange_bark" => Color::srgb(0.62, 0.42, 0.22),
		"young_palm_bark" => Color::srgb(0.52, 0.42, 0.28),
		"spring_green" => Color::srgb(0.38, 0.72, 0.38),
		"olive_green" => Color::srgb(0.42, 0.52, 0.28),
		"gold" => Color::srgb(0.78, 0.62, 0.18),
		"warm_yellow" => Color::srgb(0.82, 0.72, 0.32),
		"light_brown" => Color::srgb(0.58, 0.48, 0.32),
		"dark_brown" => Color::srgb(0.32, 0.22, 0.14),
		"red_brown" => Color::srgb(0.52, 0.28, 0.18),
		"deep_rust" => Color::srgb(0.62, 0.32, 0.18),
		"orange_brown" => Color::srgb(0.68, 0.42, 0.22),
		"deep_teal" => Color::srgb(0.12, 0.42, 0.38),
		"pale_teal" => Color::srgb(0.38, 0.68, 0.65),
		"aqua_green" => Color::srgb(0.48, 0.78, 0.72),
		"sky_blue" => Color::srgb(0.58, 0.82, 0.88),
		"cream_yellow" => Color::srgb(0.88, 0.82, 0.58),
		"silver_green" => Color::srgb(0.62, 0.72, 0.58),
		"flower_flecked" => Color::srgb(0.42, 0.62, 0.32),
		"soft_pink" => Color::srgb(0.88, 0.62, 0.72),
		"white_bloom" => Color::srgb(0.92, 0.90, 0.82),
		"violet_flecked" => Color::srgb(0.48, 0.38, 0.62),
		_ => return None,
	})
}

#[cfg(feature = "render")]
impl WithPalette for bevy::prelude::StandardMaterial {
	fn with_palette(mut base: Self, palette: PaletteMix, seed: i32) -> Self {
		if let Some(color) = palette.pick_color(seed) {
			base.base_color = color;
		}
		base.double_sided = true;
		base
	}
}

#[cfg(feature = "render")]
impl WithPalette for chico_vegetation_shaders::ChicoStickMaterial {
	fn with_palette(mut base: Self, palette: PaletteMix, seed: i32) -> Self {
		if let Some(color) = palette.pick_color(seed) {
			let linear = bevy::color::LinearRgba::from(color);
			base.base_color = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
		}
		base
	}
}

/// After lower-order [`RenderItem::spawn_render_items`](render_item::RenderItem), patch spawned
/// entities with a per-placement palette-resolved material asset.
///
/// `spawn_render_items` returns root entities only; this walks each root's descendants (children
/// exist by the time the queued closure runs) and swaps the handle on every entity already
/// bearing a [`MeshMaterial3d<M>`](bevy::prelude::MeshMaterial3d).
#[cfg(feature = "render")]
pub fn patch_spawned_leaf_material<M: bevy::prelude::Material + WithPalette + Default>(
	entities: &[bevy::prelude::Entity],
	palette: PaletteMix,
	seed: i32,
	commands: &mut bevy::prelude::Commands,
) {
	use bevy::prelude::{Assets, Children, MeshMaterial3d, World};

	if entities.is_empty() {
		return;
	}
	let mut stack = entities.to_vec();
	commands.queue(move |world: &mut World| {
		let material = M::with_palette(M::default(), palette, seed);
		let handle = world.resource_mut::<Assets<M>>().add(material);
		while let Some(entity) = stack.pop() {
			let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
				continue;
			};
			if let Some(children) = entity_mut.get::<Children>() {
				stack.extend(children.iter());
			}
			if entity_mut.contains::<MeshMaterial3d<M>>() {
				entity_mut.insert(MeshMaterial3d(handle.clone()));
			}
		}
	});
}

#[cfg(all(test, feature = "render"))]
mod tests {
	use super::*;
	use anyhow::Result;

	const TEST_MIX: PaletteMix = PaletteMix::new(&[PaletteSlot::new("deep_green", "wet_green")]);

	#[test]
	fn pick_color_is_deterministic() -> Result<()> {
		let a = TEST_MIX.pick_color(42);
		let b = TEST_MIX.pick_color(42);
		assert_eq!(a, b);
		assert!(a.is_some());
		Ok(())
	}

	#[test]
	fn standard_material_with_palette_sets_base_color() -> Result<()> {
		use bevy::prelude::StandardMaterial;

		let material = StandardMaterial::with_palette(StandardMaterial::default(), TEST_MIX, 7);
		let allowed = [
			PaletteColor("deep_green").resolve(),
			PaletteColor("wet_green").resolve(),
		];
		assert!(allowed.contains(&Some(material.base_color)));
		assert!(material.double_sided);
		Ok(())
	}
}
