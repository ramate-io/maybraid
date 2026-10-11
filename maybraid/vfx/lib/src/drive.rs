//! Generic presentation drive shared by sustained VFX.
//!
//! Attacks own timing, hazards, and damage. This crate only plays looks. B03
//! ([#1118](https://github.com/ramate-io/maybraid/issues/1118),
//! [PR #1147](https://github.com/ramate-io/maybraid/pull/1147)) emits
//! `AttackPresentationEvent`. Core effects stay on [`VfxDrive`]; optional
//! [`crate::attack_drive`] glue maps those payloads without depending on
//! `attacks`. Call [`crate::cancel_vfx`] when driving an effect independently.
//! No hit volumes live here.
//!
//! Width is **diameter in meters**. Bikanwarda's 7 m God Beam is `size = 7.0`.

use crate::composition::EffectKind;

/// Diameter of Bikanwarda's authored God Beam, in meters. Shared with B04.
pub const GOD_BEAM_WIDTH: f32 = 7.0;
pub const GOD_BEAM_WIDTH_MIN: f32 = 0.5;
pub const GOD_BEAM_WIDTH_MAX: f32 = 24.0;
pub const GOD_BEAM_HEIGHT: f32 = 96.0;
pub const GOD_BEAM_HEIGHT_MIN: f32 = 8.0;
pub const GOD_BEAM_HEIGHT_MAX: f32 = 160.0;
pub const GOD_BEAM_ACTIVE: f32 = 10.0;
pub const GOD_BEAM_ACTIVE_MIN: f32 = 0.25;
pub const GOD_BEAM_ACTIVE_MAX: f32 = 30.0;
pub const GOD_BEAM_FADE_IN: f32 = 0.4;
pub const GOD_BEAM_FADE_OUT: f32 = 0.55;

/// Presentation defaults until B04 authors wavefront numbers.
pub const SONIC_WAVE_RANGE: f32 = 28.0;
pub const SONIC_WAVE_RANGE_MIN: f32 = 4.0;
pub const SONIC_WAVE_RANGE_MAX: f32 = 60.0;
pub const SONIC_WAVE_SPREAD: f32 = 2.2;
pub const SONIC_WAVE_SPREAD_MIN: f32 = 0.35;
pub const SONIC_WAVE_SPEED: f32 = 22.0;
pub const SONIC_WAVE_SPEED_MIN: f32 = 4.0;
pub const SONIC_WAVE_SPEED_MAX: f32 = 40.0;
pub const SONIC_WAVE_COUNT: u32 = 3;
pub const SONIC_WAVE_COUNT_MAX: u32 = 6;
pub const SONIC_WAVE_STAGGER: f32 = 0.28;
pub const SONIC_WAVE_FADE_IN: f32 = 0.08;
pub const SONIC_WAVE_FADE_OUT: f32 = 0.35;

pub const FADE_MAX: f32 = 3.0;
pub const FADE_OUT_MIN: f32 = 0.05;

/// Per-instance spatial, motion, and lifetime overrides.
///
/// | Field | God Beam | Sonic wave |
/// | --- | --- | --- |
/// | [`size`](Self::size) | column **diameter** (m) | travel range (m) |
/// | [`reach`](Self::reach) | column height (m) | yaw spread (rad) |
/// | [`speed`](Self::speed) | unused | wavefront m/s |
/// | [`count`](Self::count) | `1` | number of rings |
/// | [`lifetime`](Self::lifetime) | active seconds (default 10) | travel seconds; `None` = range / speed |
/// | [`far_point`](Self::far_point) | optional sky origin | unused |
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VfxDrive {
	pub size: f32,
	pub reach: f32,
	pub speed: f32,
	pub count: u32,
	pub lifetime: Option<f32>,
	pub fade_in: f32,
	pub fade_out: f32,
	pub far_point: Option<bevy::prelude::Vec3>,
}

impl Default for VfxDrive {
	fn default() -> Self {
		Self {
			size: 1.0,
			reach: 1.0,
			speed: 0.0,
			count: 1,
			lifetime: None,
			fade_in: 0.0,
			fade_out: 0.0,
			far_point: None,
		}
	}
}

impl VfxDrive {
	/// 7 m diameter, 96 m sky column, 10 s active. B04 God Beam preset.
	pub fn god_beam() -> Self {
		Self {
			size: GOD_BEAM_WIDTH,
			reach: GOD_BEAM_HEIGHT,
			speed: 0.0,
			count: 1,
			lifetime: Some(GOD_BEAM_ACTIVE),
			fade_in: GOD_BEAM_FADE_IN,
			fade_out: GOD_BEAM_FADE_OUT,
			far_point: None,
		}
	}

	/// Forward cone that can match a 20 m Bray wavefront. Presentation only.
	pub fn sonic_wave() -> Self {
		Self {
			size: SONIC_WAVE_RANGE,
			reach: SONIC_WAVE_SPREAD,
			speed: SONIC_WAVE_SPEED,
			count: SONIC_WAVE_COUNT,
			lifetime: None,
			fade_in: SONIC_WAVE_FADE_IN,
			fade_out: SONIC_WAVE_FADE_OUT,
			far_point: None,
		}
	}

	pub fn with_size(mut self, size: f32) -> Self {
		self.size = size;
		self
	}

	pub fn with_reach(mut self, reach: f32) -> Self {
		self.reach = reach;
		self
	}

	pub fn with_speed(mut self, speed: f32) -> Self {
		self.speed = speed;
		self
	}

	pub fn with_count(mut self, count: u32) -> Self {
		self.count = count;
		self
	}

	pub fn with_lifetime(mut self, lifetime: f32) -> Self {
		self.lifetime = Some(lifetime);
		self
	}

	pub fn with_fade_in(mut self, fade_in: f32) -> Self {
		self.fade_in = fade_in;
		self
	}

	pub fn with_fade_out(mut self, fade_out: f32) -> Self {
		self.fade_out = fade_out;
		self
	}

	pub fn with_far_point(mut self, far_point: bevy::prelude::Vec3) -> Self {
		self.far_point = Some(far_point);
		self
	}

	/// Active phase after fade-in. Sonic waves derive travel as range / speed.
	pub fn active_secs(&self) -> f32 {
		if let Some(lifetime) = self.lifetime {
			return lifetime.max(0.0);
		}
		if self.speed > 1e-3 {
			return self.size / self.speed;
		}
		0.0
	}

	pub fn total_secs(&self) -> f32 {
		self.wave_total_secs(0.0)
	}

	/// Includes staggered extra rings for a sonic-wave burst.
	pub fn wave_total_secs(&self, stagger: f32) -> f32 {
		let extra = self.count.max(1).saturating_sub(1) as f32 * stagger.max(0.0);
		self.fade_in + self.active_secs() + self.fade_out + extra
	}

	pub fn resolved_for(self, kind: EffectKind) -> Self {
		match kind {
			EffectKind::GodBeam => self.resolved_god_beam(),
			EffectKind::SonicWave => self.resolved_sonic_wave(),
			EffectKind::Burst => self,
		}
	}

	pub fn resolved_god_beam(self) -> Self {
		Self {
			size: self.size.clamp(GOD_BEAM_WIDTH_MIN, GOD_BEAM_WIDTH_MAX),
			reach: self.reach.clamp(GOD_BEAM_HEIGHT_MIN, GOD_BEAM_HEIGHT_MAX),
			speed: 0.0,
			count: 1,
			lifetime: Some(self.active_secs().clamp(GOD_BEAM_ACTIVE_MIN, GOD_BEAM_ACTIVE_MAX)),
			fade_in: self.fade_in.clamp(0.0, FADE_MAX),
			fade_out: self.fade_out.clamp(FADE_OUT_MIN, FADE_MAX),
			far_point: self.far_point,
		}
	}

	pub fn resolved_sonic_wave(self) -> Self {
		Self {
			size: self.size.clamp(SONIC_WAVE_RANGE_MIN, SONIC_WAVE_RANGE_MAX),
			reach: self.reach.clamp(SONIC_WAVE_SPREAD_MIN, std::f32::consts::TAU),
			speed: self.speed.clamp(SONIC_WAVE_SPEED_MIN, SONIC_WAVE_SPEED_MAX),
			count: self.count.clamp(1, SONIC_WAVE_COUNT_MAX),
			lifetime: self.lifetime.map(|life| {
				life.clamp(
					SONIC_WAVE_RANGE_MIN / SONIC_WAVE_SPEED_MAX,
					SONIC_WAVE_RANGE_MAX / SONIC_WAVE_SPEED_MIN,
				)
			}),
			fade_in: self.fade_in.clamp(0.0, FADE_MAX),
			fade_out: self.fade_out.clamp(FADE_OUT_MIN, FADE_MAX),
			far_point: self.far_point,
		}
	}
}

/// CPU envelope shared by field materials and sustain lights.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldEnvelope {
	pub alpha: f32,
	/// 0 = hidden, 1 = full column / full travel.
	pub reveal: f32,
	/// Ground-contact bloom, 0..1.
	pub contact: f32,
}

impl FieldEnvelope {
	pub fn sample(age: f32, drive: &VfxDrive) -> Self {
		Self::sample_canceled(age, drive, None)
	}

	/// `cancel_at` is instance age when fade-out was requested.
	pub fn sample_canceled(age: f32, drive: &VfxDrive, cancel_at: Option<f32>) -> Self {
		let fade_in = drive.fade_in.max(0.0);
		let fade_out = drive.fade_out.max(FADE_OUT_MIN);
		let active = drive.active_secs();
		if let Some(cancel_at) = cancel_at {
			return Self::canceled(age, fade_in, cancel_at, fade_out);
		}
		if age <= fade_in {
			let t = if fade_in > 1e-4 { (age / fade_in).clamp(0.0, 1.0) } else { 1.0 };
			return Self { alpha: t, reveal: t, contact: (t - 0.72).max(0.0) / 0.28 };
		}
		let after_in = age - fade_in;
		if after_in <= active {
			return Self { alpha: 1.0, reveal: 1.0, contact: 1.0 };
		}
		let fade_t = ((after_in - active) / fade_out).clamp(0.0, 1.0);
		Self { alpha: 1.0 - fade_t, reveal: 1.0 - fade_t * 0.85, contact: 1.0 - fade_t }
	}

	fn canceled(age: f32, fade_in: f32, cancel_at: f32, fade_out: f32) -> Self {
		let at_cancel = Self::sample_canceled(cancel_at, &unfaded_drive(fade_in), None);
		let t = ((age - cancel_at) / fade_out).clamp(0.0, 1.0);
		Self {
			alpha: at_cancel.alpha * (1.0 - t),
			reveal: at_cancel.reveal * (1.0 - t * 0.85),
			contact: at_cancel.contact * (1.0 - t),
		}
	}
}

fn unfaded_drive(fade_in: f32) -> VfxDrive {
	VfxDrive { lifetime: Some(1.0e6), fade_in, fade_out: FADE_OUT_MIN, ..VfxDrive::god_beam() }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn god_beam_width_is_seven_meter_diameter() {
		let drive = VfxDrive::god_beam();
		assert!((drive.size - 7.0).abs() < 1e-4);
		assert!((drive.active_secs() - 10.0).abs() < 1e-4);
		assert!((drive.total_secs() - (0.4 + 10.0 + 0.55)).abs() < 1e-4);
	}

	#[test]
	fn god_beam_clamps_size_lifetime_and_fades() -> Result<(), String> {
		let drive = VfxDrive::god_beam()
			.with_size(0.01)
			.with_reach(1.0)
			.with_lifetime(80.0)
			.with_fade_in(-1.0)
			.with_fade_out(0.0)
			.resolved_god_beam();
		if (drive.size - GOD_BEAM_WIDTH_MIN).abs() >= 1e-4 {
			return Err(format!("size {}", drive.size));
		}
		if (drive.reach - GOD_BEAM_HEIGHT_MIN).abs() >= 1e-4 {
			return Err(format!("reach {}", drive.reach));
		}
		if (drive.active_secs() - GOD_BEAM_ACTIVE_MAX).abs() >= 1e-4 {
			return Err(format!("active {}", drive.active_secs()));
		}
		if drive.fade_in > 1e-4 {
			return Err(format!("fade_in {}", drive.fade_in));
		}
		if (drive.fade_out - FADE_OUT_MIN).abs() >= 1e-4 {
			return Err(format!("fade_out {}", drive.fade_out));
		}
		Ok(())
	}

	#[test]
	fn sonic_wave_travel_is_range_over_speed() {
		let drive = VfxDrive::sonic_wave();
		assert!((drive.active_secs() - (28.0 / 22.0)).abs() < 1e-4);
		assert!(
			(drive.wave_total_secs(SONIC_WAVE_STAGGER) - (0.08 + 28.0 / 22.0 + 0.35 + 2.0 * 0.28))
				.abs() < 1e-3
		);
	}

	#[test]
	fn sonic_wave_clamps_count_spread_and_speed() -> Result<(), String> {
		let drive = VfxDrive::sonic_wave()
			.with_count(99)
			.with_reach(0.01)
			.with_speed(0.1)
			.with_size(400.0)
			.resolved_sonic_wave();
		if drive.count != SONIC_WAVE_COUNT_MAX {
			return Err(format!("count {}", drive.count));
		}
		if (drive.reach - SONIC_WAVE_SPREAD_MIN).abs() >= 1e-4 {
			return Err(format!("reach {}", drive.reach));
		}
		if (drive.speed - SONIC_WAVE_SPEED_MIN).abs() >= 1e-4 {
			return Err(format!("speed {}", drive.speed));
		}
		if (drive.size - SONIC_WAVE_RANGE_MAX).abs() >= 1e-4 {
			return Err(format!("size {}", drive.size));
		}
		Ok(())
	}

	#[test]
	fn envelope_arrives_holds_and_departs() {
		let drive = VfxDrive::god_beam();
		let start = FieldEnvelope::sample(0.0, &drive);
		let arriving = FieldEnvelope::sample(0.2, &drive);
		let held = FieldEnvelope::sample(2.0, &drive);
		let gone = FieldEnvelope::sample(drive.total_secs(), &drive);
		assert!(start.alpha < 0.05);
		assert!(arriving.reveal > 0.4 && arriving.reveal < 0.7);
		assert!((held.alpha - 1.0).abs() < 1e-4);
		assert!(gone.alpha < 0.05);
	}

	#[test]
	fn cancel_fades_from_current_envelope() {
		let drive = VfxDrive::god_beam();
		let mid = FieldEnvelope::sample_canceled(3.0, &drive, Some(3.0));
		let later = FieldEnvelope::sample_canceled(3.0 + drive.fade_out, &drive, Some(3.0));
		assert!(mid.alpha > 0.9);
		assert!(later.alpha < 0.05);
	}
}
