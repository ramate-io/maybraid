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
		Self {
			fire_hot: LinearRgba::rgb(1.05, 0.32, 0.36),
			fire_mid: LinearRgba::rgb(0.72, 0.12, 0.22),
			fire_cool: LinearRgba::rgb(0.28, 0.06, 0.12),
			smoke_lit: LinearRgba::rgb(0.48, 0.14, 0.18),
			smoke_shadow: LinearRgba::rgb(0.16, 0.05, 0.09),
			spark: LinearRgba::rgb(0.98, 0.28, 0.32),
			flash: LinearRgba::rgb(0.65, 0.16, 0.22),
		}
	}

	/// Magical spawn/despawn burst derived from a single character hue (cyan default).
	pub fn spirit_burst(tint: Color) -> Self {
		let tint = LinearRgba::from(tint);
		let hot = boost(
			LinearRgba::rgb(
				(0.92 + tint.red * 0.55).min(1.55),
				(0.98 + tint.green * 0.45).min(1.55),
				(1.05 + tint.blue * 0.55).min(1.65),
			),
			1.25,
		);
		let mid = boost(tint, 1.05);
		let cool = darken(tint, 0.42);
		let shadow = darken(tint, 0.18);
		let spark = hot;
		let flash = LinearRgba::rgb(
			(1.2 + tint.red * 0.25).min(1.65),
			(1.18 + tint.green * 0.22).min(1.62),
			(1.22 + tint.blue * 0.28).min(1.68),
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
		Color::srgb(0.12, 0.88, 1.05)
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

fn boost(color: LinearRgba, factor: f32) -> LinearRgba {
	LinearRgba {
		red: color.red * factor,
		green: color.green * factor,
		blue: color.blue * factor,
		alpha: color.alpha,
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
}
