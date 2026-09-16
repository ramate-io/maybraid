//! Slow Discovery clock. Phase drives palette, key pose, and field gains.
//!
//! Default is paused at the golden-afternoon pose so play matches [#844](https://github.com/ramate-io/maybraid/issues/844).
//! `/sky play` or `MAYBRAID_SKY_PLAY=1` starts a long cycle. The field shader
//! still breathes at a fixed pose.

use bevy::prelude::*;

use crate::SkyMoon;

/// Midnight. Dark zenith, high star gain.
pub const SKY_PHASE_NIGHT: f32 = 0.0;
/// First light, rose horizon.
pub const SKY_PHASE_DAWN: f32 = 0.22;
/// Clearer blue, sun climbing.
pub const SKY_PHASE_MORNING: f32 = 0.32;
/// High key, strongest blue.
pub const SKY_PHASE_NOON: f32 = 0.50;
/// Authored Discovery pose (amber key, 45° elevation).
pub const SKY_PHASE_GOLDEN: f32 = 0.62;
/// Low sun, rose wash.
pub const SKY_PHASE_DUSK: f32 = 0.78;
/// Seconds per full cycle when the clock is playing.
pub const DEFAULT_SKY_PERIOD_SECS: f32 = 1_800.0;

const ENV_PHASE: &str = "MAYBRAID_SKY_PHASE";
const ENV_RATE: &str = "MAYBRAID_SKY_RATE";
const ENV_PLAY: &str = "MAYBRAID_SKY_PLAY";

/// Request from the playground command drawer (or tests).
#[derive(Component, Clone, Debug)]
pub enum SkyCommand {
	Status,
	SetPhase(f32),
	SetPaused(bool),
	SetPeriod(f32),
}

/// Sampled sky at one phase. Lights, wash, fog, and the field shader all read this.
#[derive(Clone, Copy, Debug)]
pub struct SkyMood {
	pub phase: f32,
	pub clear: Color,
	pub zenith: Color,
	pub horizon: Color,
	pub nadir: Color,
	pub fog: Color,
	pub sun_color: Color,
	pub sun_illuminance: f32,
	pub fill_color: Color,
	pub fill_illuminance: f32,
	pub ambient: f32,
	pub sun_elevation: f32,
	pub sun_azimuth: f32,
	pub day_weight: f32,
	pub swirl_gain: f32,
	pub star_gain: f32,
}

impl SkyMood {
	pub fn sun_pose(self) -> Transform {
		Transform::from_rotation(Quat::from_euler(
			EulerRot::XYZ,
			-self.sun_elevation,
			self.sun_azimuth,
			0.0,
		))
	}

	pub fn sun_disk_dir(self) -> Vec3 {
		crate::SkySun::disk_direction(&self.sun_pose())
	}

	pub fn moon_dir(self) -> Vec3 {
		SkyMoon::direction_from_sun(self.sun_disk_dir())
	}

	pub fn sun_above_horizon(self) -> bool {
		self.sun_elevation > 0.02
	}

	fn lerp(self, other: Self, t: f32) -> Self {
		let t = t.clamp(0.0, 1.0);
		Self {
			phase: lerp(self.phase, other.phase, t),
			clear: lerp_color(self.clear, other.clear, t),
			zenith: lerp_color(self.zenith, other.zenith, t),
			horizon: lerp_color(self.horizon, other.horizon, t),
			nadir: lerp_color(self.nadir, other.nadir, t),
			fog: lerp_color(self.fog, other.fog, t),
			sun_color: lerp_color(self.sun_color, other.sun_color, t),
			sun_illuminance: lerp(self.sun_illuminance, other.sun_illuminance, t),
			fill_color: lerp_color(self.fill_color, other.fill_color, t),
			fill_illuminance: lerp(self.fill_illuminance, other.fill_illuminance, t),
			ambient: lerp(self.ambient, other.ambient, t),
			sun_elevation: lerp(self.sun_elevation, other.sun_elevation, t),
			sun_azimuth: lerp(self.sun_azimuth, other.sun_azimuth, t),
			day_weight: lerp(self.day_weight, other.day_weight, t),
			swirl_gain: lerp(self.swirl_gain, other.swirl_gain, t),
			star_gain: lerp(self.star_gain, other.star_gain, t),
		}
	}
}

/// Playing or paused phase in `0..1`. Midnight is 0, noon is 0.5, golden is 0.62.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SkyClock {
	pub phase: f32,
	pub period_secs: f32,
	pub paused: bool,
}

impl Default for SkyClock {
	fn default() -> Self {
		Self::from_env()
	}
}

impl SkyClock {
	pub fn golden() -> Self {
		Self { phase: SKY_PHASE_GOLDEN, period_secs: DEFAULT_SKY_PERIOD_SECS, paused: true }
	}

	pub fn from_env() -> Self {
		let mut clock = Self::golden();
		if let Ok(raw) = std::env::var(ENV_PHASE) {
			if let Some(phase) = parse_sky_phase(&raw) {
				clock.phase = phase;
			}
		}
		if let Ok(raw) = std::env::var(ENV_RATE) {
			if let Ok(secs) = raw.trim().parse::<f32>() {
				if secs > 1.0 {
					clock.period_secs = secs;
				}
			}
		}
		if let Ok(raw) = std::env::var(ENV_PLAY) {
			clock.paused = !env_truthy(&raw);
		}
		clock
	}

	pub fn wrap_phase(phase: f32) -> f32 {
		phase.rem_euclid(1.0)
	}

	pub fn parse_phase(raw: &str) -> Option<f32> {
		parse_sky_phase(raw)
	}

	pub fn nearest_label(self) -> &'static str {
		preset_label(self.phase)
	}

	pub fn status_line(self) -> String {
		let play = if self.paused { "paused" } else { "playing" };
		format!(
			"sky {label} phase={phase:.3} {play} rate={rate:.0}s",
			label = self.nearest_label(),
			phase = self.phase,
			rate = self.period_secs,
		)
	}

	pub fn advance(&mut self, dt: f32) {
		if self.paused || self.period_secs <= 0.0 {
			return;
		}
		self.phase = Self::wrap_phase(self.phase + dt / self.period_secs);
	}

	pub fn sample(self) -> SkyMood {
		sample_keyframes(Self::wrap_phase(self.phase))
	}
}

fn parse_sky_phase(raw: &str) -> Option<f32> {
	let raw = raw.trim().to_ascii_lowercase();
	if raw.is_empty() {
		return None;
	}
	match raw.as_str() {
		"night" | "midnight" => Some(SKY_PHASE_NIGHT),
		"dawn" => Some(SKY_PHASE_DAWN),
		"morning" => Some(SKY_PHASE_MORNING),
		"noon" | "midday" => Some(SKY_PHASE_NOON),
		"golden" | "afternoon" | "gold" => Some(SKY_PHASE_GOLDEN),
		"dusk" | "sunset" => Some(SKY_PHASE_DUSK),
		_ => raw
			.parse::<f32>()
			.ok()
			.filter(|phase| phase.is_finite())
			.map(SkyClock::wrap_phase),
	}
}

fn env_truthy(raw: &str) -> bool {
	matches!(raw.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on" | "play")
}

fn preset_label(phase: f32) -> &'static str {
	let marks = [
		(SKY_PHASE_NIGHT, "night"),
		(SKY_PHASE_DAWN, "dawn"),
		(SKY_PHASE_MORNING, "morning"),
		(SKY_PHASE_NOON, "noon"),
		(SKY_PHASE_GOLDEN, "golden"),
		(SKY_PHASE_DUSK, "dusk"),
	];
	let mut best = marks[0];
	let mut best_d = 1.0_f32;
	for mark in marks {
		let d = circular_distance(phase, mark.0);
		if d < best_d {
			best = mark;
			best_d = d;
		}
	}
	best.1
}

fn circular_distance(a: f32, b: f32) -> f32 {
	let d = (a - b).abs();
	d.min(1.0 - d)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
	let a = a.to_linear();
	let b = b.to_linear();
	Color::from(LinearRgba {
		red: lerp(a.red, b.red, t),
		green: lerp(a.green, b.green, t),
		blue: lerp(a.blue, b.blue, t),
		alpha: lerp(a.alpha, b.alpha, t),
	})
}

#[allow(clippy::too_many_arguments)]
fn key(
	phase: f32,
	zenith: Color,
	horizon: Color,
	nadir: Color,
	clear: Color,
	sun_color: Color,
	sun_illuminance: f32,
	fill_illuminance: f32,
	ambient: f32,
	sun_elevation: f32,
	sun_azimuth: f32,
	day_weight: f32,
	swirl_gain: f32,
	star_gain: f32,
) -> SkyMood {
	SkyMood {
		phase,
		clear,
		zenith,
		horizon,
		nadir,
		fog: horizon,
		sun_color,
		sun_illuminance,
		fill_color: crate::FILL_COLOR,
		fill_illuminance,
		ambient,
		sun_elevation,
		sun_azimuth,
		day_weight,
		swirl_gain,
		star_gain,
	}
}

fn keyframes() -> [SkyMood; 7] {
	let yaw = crate::SUN_YAW;
	[
		key(
			0.0,
			Color::hsla(232.0, 0.48, 0.12, 1.0),
			Color::hsla(248.0, 0.36, 0.10, 1.0),
			Color::hsla(240.0, 0.22, 0.04, 1.0),
			Color::hsla(232.0, 0.42, 0.10, 1.0),
			Color::hsla(220.0, 0.18, 0.72, 1.0),
			70.0,
			400.0,
			180.0,
			-0.18,
			0.2,
			0.04,
			1.0,
			1.0,
		),
		key(
			SKY_PHASE_DAWN,
			Color::hsla(222.0, 0.38, 0.30, 1.0),
			Color::hsla(18.0, 0.70, 0.62, 1.0),
			Color::hsla(14.0, 0.42, 0.16, 1.0),
			Color::hsla(20.0, 0.45, 0.42, 1.0),
			Color::hsla(22.0, 0.48, 0.78, 1.0),
			1_800.0,
			1_100.0,
			400.0,
			0.12,
			-0.35,
			0.42,
			0.48,
			0.28,
		),
		key(
			SKY_PHASE_MORNING,
			Color::hsla(208.0, 0.52, 0.58, 1.0),
			Color::hsla(42.0, 0.42, 0.78, 1.0),
			Color::hsla(30.0, 0.22, 0.26, 1.0),
			Color::hsla(208.0, 0.40, 0.70, 1.0),
			Color::hsla(42.0, 0.28, 0.86, 1.0),
			6_400.0,
			1_400.0,
			540.0,
			0.55,
			0.15,
			0.86,
			0.28,
			0.08,
		),
		key(
			SKY_PHASE_NOON,
			Color::hsla(206.0, 0.58, 0.55, 1.0),
			Color::hsla(50.0, 0.34, 0.82, 1.0),
			Color::hsla(38.0, 0.18, 0.30, 1.0),
			Color::hsla(206.0, 0.48, 0.68, 1.0),
			Color::hsla(48.0, 0.22, 0.90, 1.0),
			9_400.0,
			1_700.0,
			700.0,
			1.15,
			yaw * 0.7,
			1.0,
			0.22,
			0.04,
		),
		key(
			SKY_PHASE_GOLDEN,
			crate::SKY_ZENITH,
			crate::SKY_HORIZON,
			crate::SKY_NADIR,
			crate::SKY_CLEAR,
			crate::SUN_COLOR,
			crate::SUN_ILLUMINANCE,
			crate::FILL_ILLUMINANCE,
			crate::AMBIENT_BRIGHTNESS,
			-crate::SUN_PITCH,
			yaw,
			0.82,
			0.40,
			0.14,
		),
		key(
			SKY_PHASE_DUSK,
			Color::hsla(248.0, 0.34, 0.22, 1.0),
			Color::hsla(16.0, 0.72, 0.54, 1.0),
			Color::hsla(10.0, 0.40, 0.12, 1.0),
			Color::hsla(18.0, 0.48, 0.32, 1.0),
			Color::hsla(24.0, 0.50, 0.76, 1.0),
			2_200.0,
			900.0,
			360.0,
			0.14,
			1.05,
			0.38,
			0.58,
			0.40,
		),
		key(
			0.93,
			Color::hsla(234.0, 0.46, 0.11, 1.0),
			Color::hsla(250.0, 0.34, 0.09, 1.0),
			Color::hsla(240.0, 0.20, 0.04, 1.0),
			Color::hsla(234.0, 0.40, 0.09, 1.0),
			Color::hsla(220.0, 0.16, 0.70, 1.0),
			55.0,
			350.0,
			160.0,
			-0.22,
			1.35,
			0.05,
			1.0,
			1.0,
		),
	]
}

fn sample_keyframes(phase: f32) -> SkyMood {
	let keys = keyframes();
	let last = keys.len() - 1;
	if phase <= keys[0].phase {
		return keys[0];
	}
	for window in keys.windows(2) {
		let a = window[0];
		let b = window[1];
		if phase <= b.phase {
			let span = (b.phase - a.phase).max(1e-4);
			return a.lerp(b, (phase - a.phase) / span);
		}
	}
	let a = keys[last];
	let b = keys[0];
	let span = (1.0 - a.phase + b.phase).max(1e-4);
	let t = (phase - a.phase) / span;
	a.lerp(b, t)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::f32::consts::PI;

	#[test]
	fn golden_keeps_the_authored_key_pose() {
		let mood = SkyClock { phase: SKY_PHASE_GOLDEN, ..SkyClock::golden() }.sample();
		assert!((mood.sun_elevation - PI / 4.0).abs() < 1e-3);
		assert!((mood.sun_azimuth - crate::SUN_YAW).abs() < 1e-3);
		assert!((mood.sun_illuminance - crate::SUN_ILLUMINANCE).abs() < 1.0);
		assert!(mood.day_weight > 0.7);
		assert!(mood.sun_above_horizon());
	}

	#[test]
	fn noon_is_higher_and_brighter_than_golden() {
		let noon = SkyClock { phase: SKY_PHASE_NOON, ..SkyClock::golden() }.sample();
		let golden = SkyClock { phase: SKY_PHASE_GOLDEN, ..SkyClock::golden() }.sample();
		assert!(noon.sun_elevation > golden.sun_elevation);
		assert!(noon.sun_illuminance > golden.sun_illuminance);
		assert!(noon.day_weight > golden.day_weight);
	}

	#[test]
	fn night_drops_the_sun_and_raises_stars() {
		let night = SkyClock { phase: SKY_PHASE_NIGHT, ..SkyClock::golden() }.sample();
		assert!(!night.sun_above_horizon());
		assert!(night.day_weight < 0.15);
		assert!(night.star_gain > 0.8);
		assert!(night.sun_illuminance < 200.0);
	}

	#[test]
	fn wrap_samples_across_midnight() {
		let late = SkyClock { phase: 0.99, ..SkyClock::golden() }.sample();
		assert!(late.day_weight < 0.2);
	}

	#[test]
	fn parse_presets_and_numeric_phase() {
		assert_eq!(SkyClock::parse_phase("golden"), Some(SKY_PHASE_GOLDEN));
		assert_eq!(SkyClock::parse_phase("noon"), Some(SKY_PHASE_NOON));
		assert_eq!(SkyClock::parse_phase("0.35"), Some(0.35));
		assert_eq!(SkyClock::parse_phase("1.25"), Some(0.25));
		assert!(SkyClock::parse_phase("nope").is_none());
	}

	#[test]
	fn advance_respects_pause() {
		let mut clock = SkyClock::golden();
		clock.advance(10.0);
		assert!((clock.phase - SKY_PHASE_GOLDEN).abs() < 1e-5);
		clock.paused = false;
		clock.period_secs = 100.0;
		clock.advance(10.0);
		assert!((clock.phase - (SKY_PHASE_GOLDEN + 0.1)).abs() < 1e-4);
	}
}
