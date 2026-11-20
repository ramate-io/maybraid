//! Shared explosion colors. Tint is an optional overall multiplier.

use bevy::prelude::*;

/// Fire, smoke, spark, and flash colors for one instance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExplosionPalette {
	pub fire_hot: LinearRgba,
	pub fire_mid: LinearRgba,
	pub fire_cool: LinearRgba,
	pub smoke_lit: LinearRgba,
	pub smoke_shadow: LinearRgba,
	pub spark: LinearRgba,
	pub flash: LinearRgba,
}

impl Default for ExplosionPalette {
	fn default() -> Self {
		Self::maybraid()
	}
}

impl ExplosionPalette {
	/// Dark viscous combat fluid — stylized, not photoreal gore.
	pub fn maybraid_ooze() -> Self {
		Self::combat_ooze(Color::srgb(0.78, 0.07, 0.12))
	}

	/// Crimson (or per-character) impact fluid. Floors luminance so tint never
	/// collapses to a black flash after multiply.
	pub fn combat_ooze(tint: Color) -> Self {
		let tint = LinearRgba::from(tint);
		let red = tint.red.max(0.62).min(0.94);
		let green = tint.green.clamp(0.04, 0.22);
		let blue = tint.blue.clamp(0.05, 0.28);
		Self {
			fire_hot: LinearRgba::rgb(red.min(0.94), (green * 1.15).min(0.18), (blue * 1.05).min(0.20)),
			fire_mid: LinearRgba::rgb(red * 0.88, green, blue),
			fire_cool: LinearRgba::rgb((red * 0.58).max(0.34), green * 0.4, blue * 0.4),
			smoke_lit: LinearRgba::rgb(red * 0.7, green * 0.85, blue * 0.85),
			smoke_shadow: LinearRgba::rgb((red * 0.38).max(0.22), green * 0.35, blue * 0.35),
			spark: LinearRgba::rgb(red.min(0.9), (green * 1.5).min(0.24), (blue * 1.3).min(0.26)),
			flash: LinearRgba::rgb(red * 0.75, green * 0.9, blue * 0.9),
		}
	}

	/// Magical spawn/despawn burst derived from a single character hue (cyan default).
	///
	/// Channels stay near 1.0 so additive particles keep the tint instead of
	/// stacking into a white ball. Hue variation still tracks `tint`.
	pub fn spirit_burst(tint: Color) -> Self {
		let tint = LinearRgba::from(tint);
		let hot = LinearRgba::rgb(
			(tint.red * 0.72 + 0.16).min(1.05),
			(tint.green * 0.72 + 0.16).min(1.05),
			(tint.blue * 0.72 + 0.16).min(1.05),
		);
		let mid = LinearRgba::rgb(
			(tint.red * 0.88).clamp(0.08, 0.88),
			(tint.green * 0.88).clamp(0.08, 0.88),
			(tint.blue * 0.88).clamp(0.08, 0.88),
		);
		let cool = darken(tint, 0.42);
		let shadow = darken(tint, 0.2);
		let spark = LinearRgba::rgb(
			(tint.red * 0.5 + 0.38).min(1.08),
			(tint.green * 0.5 + 0.38).min(1.08),
			(tint.blue * 0.5 + 0.38).min(1.08),
		);
		let flash = LinearRgba::rgb(
			(tint.red * 0.6 + 0.22).min(1.02),
			(tint.green * 0.6 + 0.22).min(1.02),
			(tint.blue * 0.6 + 0.22).min(1.02),
		);
		Self {
			fire_hot: hot,
			fire_mid: mid,
			fire_cool: cool,
			smoke_lit: mid,
			smoke_shadow: shadow,
			spark,
			flash,
		}
	}

	pub fn default_spirit_tint() -> Color {
		Color::srgb(0.16, 0.82, 0.96)
	}

	/// Rising wisps on defeat — soft celestial tones used elsewhere in Maybraid.
	pub fn celestial() -> Self {
		Self {
			fire_hot: LinearRgba::rgb(1.05, 0.92, 1.25),
			fire_mid: LinearRgba::rgb(0.55, 0.72, 1.15),
			fire_cool: LinearRgba::rgb(0.22, 0.38, 0.85),
			smoke_lit: LinearRgba::rgb(0.75, 0.82, 1.05),
			smoke_shadow: LinearRgba::rgb(0.18, 0.28, 0.55),
			spark: LinearRgba::rgb(1.2, 1.0, 1.35),
			flash: LinearRgba::rgb(0.85, 0.90, 1.1),
		}
	}

	pub fn maybraid() -> Self {
		Self {
			fire_hot: LinearRgba::rgb(1.12, 0.88, 0.38),
			fire_mid: LinearRgba::rgb(0.98, 0.34, 0.06),
			fire_cool: LinearRgba::rgb(0.32, 0.05, 0.02),
			smoke_lit: LinearRgba::rgb(0.46, 0.36, 0.28),
			smoke_shadow: LinearRgba::rgb(0.18, 0.22, 0.28),
			spark: LinearRgba::rgb(1.6, 1.05, 0.32),
			flash: LinearRgba::rgb(1.0, 0.82, 0.45),
		}
	}

	pub fn with_tint(self, tint: Option<Color>) -> Self {
		let Some(tint) = tint else {
			return self;
		};
		let tint = LinearRgba::from(tint);
		Self {
			fire_hot: mul(self.fire_hot, tint),
			fire_mid: mul(self.fire_mid, tint),
			fire_cool: mul(self.fire_cool, tint),
			smoke_lit: mul(self.smoke_lit, tint),
			smoke_shadow: mul(self.smoke_shadow, tint),
			spark: mul(self.spark, tint),
			flash: mul(self.flash, tint),
		}
	}

	pub fn vec4(color: LinearRgba) -> Vec4 {
		Vec4::new(color.red, color.green, color.blue, color.alpha)
	}

	pub fn color(color: LinearRgba) -> Color {
		Color::from(color)
	}
}

fn darken(color: LinearRgba, factor: f32) -> LinearRgba {
	LinearRgba {
		red: color.red * factor,
		green: color.green * factor,
		blue: color.blue * factor,
		alpha: color.alpha,
	}
}

fn mul(a: LinearRgba, b: LinearRgba) -> LinearRgba {
	LinearRgba {
		red: a.red * b.red,
		green: a.green * b.green,
		blue: a.blue * b.blue,
		alpha: a.alpha * b.alpha,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tint_multiplies_instead_of_replacing() {
		let tinted = ExplosionPalette::maybraid().with_tint(Some(Color::srgb(0.0, 1.0, 1.0)));
		assert!(tinted.fire_hot.red < 0.05);
		assert!(tinted.fire_hot.green > 0.5);
		assert!(tinted.flash.red < 0.05);
	}

	#[test]
	fn spirit_burst_keeps_hue_without_hdr_blowout() {
		let cyan = ExplosionPalette::spirit_burst(Color::srgb(0.16, 0.82, 0.96));
		let rose = ExplosionPalette::spirit_burst(Color::srgb(1.0, 0.22, 0.55));
		assert!(rose.fire_hot.red > cyan.fire_hot.red + 0.15);
		assert!(cyan.fire_hot.blue > rose.fire_hot.blue + 0.1);
		for channel in [cyan.flash.red, cyan.flash.green, cyan.flash.blue, cyan.fire_hot.red] {
			assert!(channel <= 1.1, "hdr {channel}");
		}
	}

	#[test]
	fn combat_ooze_never_goes_black() {
		let dark = ExplosionPalette::combat_ooze(Color::srgb(0.2, 0.0, 0.0));
		assert!(dark.fire_hot.red > 0.5);
		assert!(dark.fire_cool.red > 0.25);
		let crimson = ExplosionPalette::maybraid_ooze();
		assert!(crimson.fire_mid.red > 0.4);
		assert!(crimson.spark.green < 0.3);
	}
}
