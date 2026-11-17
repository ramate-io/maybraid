//! Runtime intensity knobs so a discrete GPU can retune without a rebuild.
//!
//! `MAYBRAID_SPIRIT_INTENSITY` and `MAYBRAID_OOZE_INTENSITY` (default `1.0`,
//! clamped to `0.25..=2.0`).

pub fn env_intensity(var: &str, default: f32) -> f32 {
	std::env::var(var)
		.ok()
		.and_then(|value| value.parse::<f32>().ok())
		.unwrap_or(default)
		.clamp(0.25, 2.0)
}

pub fn spirit_intensity() -> f32 {
	env_intensity("MAYBRAID_SPIRIT_INTENSITY", 1.0)
}

pub fn ooze_intensity() -> f32 {
	env_intensity("MAYBRAID_OOZE_INTENSITY", 1.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn defaults_are_unity() {
		assert!((spirit_intensity() - 1.0).abs() < 1e-5 || (0.25..=2.0).contains(&spirit_intensity()));
		assert!((0.25..=2.0).contains(&ooze_intensity()));
	}
}
