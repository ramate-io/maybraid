//! Generic presentation drive shared by sustained VFX.
//!
//! Attacks own timing, hazards, and damage. This crate only plays looks. Drive
//! an effect with [`VfxDrive`] and later call [`crate::cancel_vfx`]. Sonic rings
//! render [`SonicWaveState`] snapshots written by the caller; they do not
//! simulate travel. No hit volumes live here.
//!
//! Width is **diameter in meters**. Bikanwarda's 7 m God Beam is `size = 7.0`.

use bevy::prelude::*;

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
pub const SONIC_WAVE_RANGE: f32 = 20.0;
pub const SONIC_WAVE_RANGE_MIN: f32 = 4.0;
pub const SONIC_WAVE_RANGE_MAX: f32 = 60.0;
/// Fan cone for multiple ring paths. `0` is a single straight path.
pub const SONIC_WAVE_SPREAD: f32 = 0.0;
pub const SONIC_WAVE_SPREAD_MAX: f32 = std::f32::consts::FRAC_PI_2;
pub const SONIC_WAVE_SPEED: f32 = 22.0;
pub const SONIC_WAVE_SPEED_MIN: f32 = 4.0;
pub const SONIC_WAVE_SPEED_MAX: f32 = 40.0;
pub const SONIC_WAVE_COUNT: u32 = 3;
pub const SONIC_WAVE_COUNT_MAX: u32 = 6;
pub const SONIC_WAVE_INTERVAL: f32 = 0.28;
pub const SONIC_WAVE_RADIUS: f32 = 0.55;
pub const SONIC_WAVE_RADIUS_MIN: f32 = 0.12;
pub const SONIC_WAVE_RADIUS_MAX: f32 = 3.0;
pub const SONIC_WAVE_THICKNESS: f32 = 0.14;
pub const SONIC_WAVE_THICKNESS_MIN: f32 = 0.04;
/// Wide enough for `#1147` `thickness = 2 * half_depth` (authored 0.70).
pub const SONIC_WAVE_THICKNESS_MAX: f32 = 2.0;
pub const SONIC_WAVE_FADE_IN: f32 = 0.08;
pub const SONIC_WAVE_FADE_OUT: f32 = 0.35;

/// Older name for [`SONIC_WAVE_INTERVAL`].
pub const SONIC_WAVE_STAGGER: f32 = SONIC_WAVE_INTERVAL;

pub const FADE_MAX: f32 = 3.0;
pub const FADE_OUT_MIN: f32 = 0.05;

/// Per-instance spatial, motion, and lifetime overrides.
///
/// | Field | God Beam | Sonic wave |
/// | --- | --- | --- |
/// | [`size`](Self::size) | column **diameter** (m) | travel **range** (m) |
/// | [`reach`](Self::reach) | column height (m) | fan cone (rad); `0` = one path |
/// | [`speed`](Self::speed) | unused | travel m/s (driver / presenter) |
/// | [`count`](Self::count) | `1` | sequential rings |
/// | [`radius`](Self::radius) | unused | constant ring radius (m) |
/// | [`thickness`](Self::thickness) | unused | ring depth along travel (m) |
/// | [`pitch`](Self::pitch) | unused | `None` flattens travel to XZ |
/// | [`interval`](Self::interval) | unused | seconds between sequential rings |
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
	pub far_point: Option<Vec3>,
	pub radius: f32,
	pub thickness: f32,
	pub pitch: Option<f32>,
	/// Seconds between sequential sonic rings. Unused by God Beam.
	pub interval: f32,
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
			radius: 0.0,
			thickness: 0.0,
			pitch: None,
			interval: 0.0,
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
			radius: 0.0,
			thickness: 0.0,
			pitch: None,
			interval: 0.0,
		}
	}

	/// Mouth-emitted vortex rings. Presentation only — no travel simulation.
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
			radius: SONIC_WAVE_RADIUS,
			thickness: SONIC_WAVE_THICKNESS,
			pitch: None,
			interval: SONIC_WAVE_INTERVAL,
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

	pub fn with_far_point(mut self, far_point: Vec3) -> Self {
		self.far_point = Some(far_point);
		self
	}

	pub fn with_radius(mut self, radius: f32) -> Self {
		self.radius = radius;
		self
	}

	pub fn with_thickness(mut self, thickness: f32) -> Self {
		self.thickness = thickness;
		self
	}

	pub fn with_pitch(mut self, pitch: f32) -> Self {
		self.pitch = Some(pitch);
		self
	}

	pub fn with_interval(mut self, interval: f32) -> Self {
		self.interval = interval;
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
		self.wave_total_secs(self.interval)
	}

	/// Includes staggered extra rings for a sonic-wave burst.
	pub fn wave_total_secs(&self, interval: f32) -> f32 {
		let extra = self.count.max(1).saturating_sub(1) as f32 * interval.max(0.0);
		self.fade_in + self.active_secs() + self.fade_out + extra
	}

	/// Snapshots a presenter or the playground driver can write onto [`VfxWaveStates`].
	/// The effect does not call this.
	pub fn sonic_waves_at(self, origin: Vec3, forward: Vec3, age: f32) -> Vec<SonicWaveState> {
		let drive = self.resolved_sonic_wave();
		(0..drive.count)
			.filter_map(|index| {
				let direction =
					fan_direction(forward, drive.reach, index, drive.count, drive.pitch);
				SonicWaveState::at_time(
					origin,
					direction,
					drive.speed,
					age,
					index as f32 * drive.interval,
					drive.radius,
					drive.thickness,
					index,
					drive.size,
					drive.pitch,
				)
				.filter(|wave| !wave.expired())
			})
			.collect()
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
			radius: 0.0,
			thickness: 0.0,
			pitch: None,
			interval: 0.0,
		}
	}

	pub fn resolved_sonic_wave(self) -> Self {
		let radius = if self.radius <= 1e-4 { SONIC_WAVE_RADIUS } else { self.radius };
		let thickness = if self.thickness <= 1e-4 { SONIC_WAVE_THICKNESS } else { self.thickness };
		let interval = if self.interval <= 1e-4 { SONIC_WAVE_INTERVAL } else { self.interval };
		Self {
			size: self.size.clamp(SONIC_WAVE_RANGE_MIN, SONIC_WAVE_RANGE_MAX),
			reach: self.reach.clamp(0.0, SONIC_WAVE_SPREAD_MAX),
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
			radius: radius.clamp(SONIC_WAVE_RADIUS_MIN, SONIC_WAVE_RADIUS_MAX),
			thickness: thickness.clamp(SONIC_WAVE_THICKNESS_MIN, SONIC_WAVE_THICKNESS_MAX),
			pitch: self.pitch,
			interval: interval.clamp(0.05, 2.0),
		}
	}
}

fn fan_direction(forward: Vec3, spread: f32, index: u32, count: u32, pitch: Option<f32>) -> Vec3 {
	let base = SonicWaveState::aim(forward, pitch);
	if count <= 1 || spread <= 1e-4 {
		return base;
	}
	let t = index as f32 / (count - 1) as f32;
	Quat::from_axis_angle(Vec3::Y, (t - 0.5) * spread) * base
}

/// Authoritative pose of one sonic ring. The effect only renders this.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SonicWaveState {
	pub origin: Vec3,
	pub direction: Vec3,
	pub position: Vec3,
	pub radius: f32,
	pub thickness: f32,
	pub index: u32,
	pub range: f32,
}

impl SonicWaveState {
	/// Copy [`WavefrontPresentation`](https://github.com/ramate-io/maybraid/pull/1147)
	/// fields. Does not import `attacks` types. Direction is used as supplied.
	pub fn follow(
		origin: Vec3,
		direction: Vec3,
		position: Vec3,
		radius: f32,
		thickness: f32,
		wave_index: u32,
		range: f32,
	) -> Self {
		let direction =
			if direction.length_squared() > 1e-8 { direction.normalize() } else { Vec3::Z };
		Self {
			origin,
			direction,
			position,
			radius: radius.max(1e-4),
			thickness: thickness.max(1e-4),
			index: wave_index,
			range: range.max(0.0),
		}
	}

	pub fn emit(
		origin: Vec3,
		direction: Vec3,
		radius: f32,
		thickness: f32,
		index: u32,
		range: f32,
	) -> Self {
		let direction = Self::aim(direction, None);
		Self {
			origin,
			direction,
			position: origin,
			radius: radius.max(SONIC_WAVE_RADIUS_MIN),
			thickness: thickness.max(SONIC_WAVE_THICKNESS_MIN),
			index,
			range: range.max(0.0),
		}
	}

	/// Flatten pitch onto XZ. `Some(pitch)` keeps that elevation.
	pub fn aim(direction: Vec3, pitch: Option<f32>) -> Vec3 {
		if let Some(pitch) = pitch {
			let yaw = if direction.xz().length_squared() > 1e-8 {
				direction.x.atan2(direction.z)
			} else {
				0.0
			};
			return Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0) * Vec3::Z;
		}
		let flat = Vec3::new(direction.x, 0.0, direction.z);
		if flat.length_squared() < 1e-8 {
			return Vec3::Z;
		}
		flat.normalize()
	}

	/// Snapshot at `age` for a ring emitted after `delay`. `None` if not yet emitted.
	pub fn at_time(
		origin: Vec3,
		direction: Vec3,
		speed: f32,
		age: f32,
		delay: f32,
		radius: f32,
		thickness: f32,
		index: u32,
		range: f32,
		pitch: Option<f32>,
	) -> Option<Self> {
		let travel_age = age - delay;
		if travel_age < 0.0 {
			return None;
		}
		let direction = Self::aim(direction, pitch);
		let travel = (speed.max(0.0) * travel_age).min(range.max(0.0));
		Some(Self {
			origin,
			direction,
			position: origin + direction * travel,
			radius: radius.max(SONIC_WAVE_RADIUS_MIN),
			thickness: thickness.max(SONIC_WAVE_THICKNESS_MIN),
			index,
			range: range.max(0.0),
		})
	}

	pub fn travel(self) -> f32 {
		self.position.distance(self.origin)
	}

	pub fn expired(self) -> bool {
		self.travel() + 1e-4 >= self.range
	}

	/// Ring plane is perpendicular to travel. Mesh faces +Z at unit scale.
	/// Outer radius and depth match [`Self::radius`] / [`Self::thickness`].
	pub fn ring_transform(self) -> Transform {
		let dir = if self.direction.length_squared() > 1e-8 {
			self.direction.normalize()
		} else {
			Vec3::Z
		};
		Transform {
			translation: self.position,
			rotation: Quat::from_rotation_arc(Vec3::Z, dir),
			scale: Vec3::new(
				self.radius.max(1e-4) * 2.0,
				self.radius.max(1e-4) * 2.0,
				self.thickness.max(1e-4),
			),
		}
	}

	/// Child-local pose so the visible ring sits at the supplied world position.
	pub fn local_ring_transform(self, root: Transform) -> Transform {
		let world = self.ring_transform();
		let inv = root.rotation.inverse();
		Transform {
			translation: inv * (world.translation - root.translation),
			rotation: inv * world.rotation,
			scale: world.scale,
		}
	}

	/// Alpha 1 until the last `fade_out` seconds of travel, then down to 0 at range.
	pub fn fade(self, speed: f32, fade_out: f32) -> f32 {
		let fade_dist = fade_out.max(FADE_OUT_MIN) * speed.max(1e-3);
		((self.range - self.travel()) / fade_dist).clamp(0.0, 1.0)
	}
}

/// Live ring snapshots. Written by a presenter or the playground driver.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct VfxWaveStates {
	pub waves: Vec<SonicWaveState>,
}

impl VfxWaveStates {
	pub fn get(&self, index: u32) -> Option<SonicWaveState> {
		self.waves.iter().copied().find(|wave| wave.index == index)
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
		assert!((drive.size - SONIC_WAVE_RANGE).abs() < 1e-4);
		assert!((drive.active_secs() - (SONIC_WAVE_RANGE / SONIC_WAVE_SPEED)).abs() < 1e-4);
		assert!(
			(drive.wave_total_secs(SONIC_WAVE_INTERVAL)
				- (0.08 + SONIC_WAVE_RANGE / SONIC_WAVE_SPEED + 0.35 + 2.0 * 0.28))
				.abs() < 1e-3
		);
	}

	#[test]
	fn sonic_wave_keeps_wavefront_thickness() -> Result<(), String> {
		let drive = VfxDrive::sonic_wave().with_thickness(0.70).resolved_sonic_wave();
		if (drive.thickness - 0.70).abs() >= 1e-4 {
			return Err(format!("clamped authored thickness {}", drive.thickness));
		}
		Ok(())
	}

	#[test]
	fn sonic_wave_clamps_count_and_speed() -> Result<(), String> {
		let drive = VfxDrive::sonic_wave()
			.with_count(99)
			.with_reach(-1.0)
			.with_speed(0.1)
			.with_size(400.0)
			.resolved_sonic_wave();
		if drive.count != SONIC_WAVE_COUNT_MAX {
			return Err(format!("count {}", drive.count));
		}
		if drive.reach > 1e-4 {
			return Err(format!("spread {}", drive.reach));
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
	fn sonic_ring_travels_at_constant_radius() -> Result<(), String> {
		let origin = Vec3::new(0.0, 1.6, 0.0);
		let early = SonicWaveState::at_time(
			origin,
			Vec3::new(0.2, 0.8, 1.0),
			22.0,
			0.25,
			0.0,
			0.55,
			0.14,
			0,
			20.0,
			None,
		)
		.ok_or("expected first ring")?;
		let later = SonicWaveState::at_time(
			origin,
			Vec3::new(0.2, 0.8, 1.0),
			22.0,
			0.80,
			0.0,
			0.55,
			0.14,
			0,
			20.0,
			None,
		)
		.ok_or("expected later ring")?;
		if (early.travel() - 22.0 * 0.25).abs() >= 1e-3 {
			return Err(format!("travel {}", early.travel()));
		}
		if (later.travel() - 22.0 * 0.80).abs() >= 1e-3 {
			return Err(format!("later {}", later.travel()));
		}
		if (early.radius - later.radius).abs() >= 1e-4 {
			return Err("radius grew".into());
		}
		if early.direction.y.abs() > 1e-4 {
			return Err(format!("not horizontal {}", early.direction));
		}
		Ok(())
	}

	#[test]
	fn sonic_ring_plane_is_perpendicular_to_travel() -> Result<(), String> {
		let wave = SonicWaveState::emit(Vec3::ZERO, Vec3::Z, 0.55, 0.14, 0, 20.0);
		let pose = wave.ring_transform();
		let normal = pose.rotation * Vec3::Z;
		if (normal - Vec3::Z).length() > 1e-3 {
			return Err(format!("normal {normal}"));
		}
		Ok(())
	}

	#[test]
	fn sonic_count_uses_interval_spacing() -> Result<(), String> {
		let origin = Vec3::new(0.0, 1.55, 0.0);
		let second =
			SonicWaveState::at_time(origin, Vec3::Z, 22.0, 0.28, 0.28, 0.55, 0.14, 1, 20.0, None);
		let early =
			SonicWaveState::at_time(origin, Vec3::Z, 22.0, 0.20, 0.28, 0.55, 0.14, 1, 20.0, None);
		if second.is_none() {
			return Err("second ring should emit at interval".into());
		}
		if early.is_some() {
			return Err("second ring emitted too early".into());
		}
		if (second.ok_or("ring")?.travel()).abs() >= 1e-3 {
			return Err("second ring should still be at the mouth".into());
		}
		Ok(())
	}

	#[test]
	fn sonic_ring_expires_at_range() -> Result<(), String> {
		let wave =
			SonicWaveState::at_time(Vec3::ZERO, Vec3::Z, 22.0, 2.0, 0.0, 0.55, 0.14, 0, 20.0, None)
				.ok_or("expected wave")?;
		if (wave.travel() - 20.0).abs() >= 1e-3 {
			return Err(format!("travel {}", wave.travel()));
		}
		if !wave.expired() {
			return Err("should expire at range".into());
		}
		if wave.fade(22.0, 0.35) >= 0.05 {
			return Err("should fade at range".into());
		}
		Ok(())
	}

	#[test]
	fn sonic_driver_helper_emits_three_constant_rings() -> Result<(), String> {
		let origin = Vec3::new(0.0, 1.55, 0.0);
		let waves = VfxDrive::sonic_wave().sonic_waves_at(origin, Vec3::Z, 0.70);
		if waves.len() != 3 {
			return Err(format!("count {}", waves.len()));
		}
		if (waves[0].radius - waves[2].radius).abs() >= 1e-4 {
			return Err("radius changed across rings".into());
		}
		if (waves[0].travel() - waves[1].travel() - SONIC_WAVE_SPEED * SONIC_WAVE_INTERVAL).abs()
			>= 1e-2
		{
			return Err(format!("spacing {} {}", waves[0].travel(), waves[1].travel()));
		}
		if waves.iter().any(|wave| wave.direction.y.abs() > 1e-4) {
			return Err("travel left the XZ plane".into());
		}
		Ok(())
	}

	#[test]
	fn follow_matches_wavefront_presentation_fields() -> Result<(), String> {
		let origin = Vec3::new(0.0, 1.6, 0.2);
		let direction = Vec3::new(0.2, 0.5, 0.8).normalize();
		let position = origin + direction * 4.4;
		let wave = SonicWaveState::follow(origin, direction, position, 1.0, 0.70, 2, 10.0);
		if (wave.position - position).length() > 1e-4 {
			return Err(format!("position {}", wave.position));
		}
		if (wave.direction - direction).length() > 1e-3 {
			return Err(format!("re-aimed {}", wave.direction));
		}
		if wave.index != 2 {
			return Err(format!("wave_index {}", wave.index));
		}
		let pose = wave.ring_transform();
		if (pose.translation - position).length() > 1e-4 {
			return Err(format!("sit {}", pose.translation));
		}
		let facing = pose.rotation * Vec3::Z;
		if (facing - direction).length() > 1e-3 {
			return Err(format!("facing {facing}"));
		}
		if (pose.scale.x * 0.5 - 1.0).abs() >= 1e-4 {
			return Err(format!("radius {}", pose.scale.x * 0.5));
		}
		if (pose.scale.z - 0.70).abs() >= 1e-4 {
			return Err(format!("thickness {}", pose.scale.z));
		}
		Ok(())
	}

	#[test]
	fn local_ring_transform_follows_supplied_world_position() -> Result<(), String> {
		let origin = Vec3::new(2.0, 1.6, -1.0);
		let wave =
			SonicWaveState::at_time(origin, Vec3::Z, 22.0, 0.5, 0.0, 0.55, 0.14, 0, 20.0, None)
				.ok_or("wave")?;
		let local = wave.local_ring_transform(Transform::from_translation(origin));
		if (local.translation - Vec3::new(0.0, 0.0, 11.0)).length() > 1e-3 {
			return Err(format!("local {}", local.translation));
		}
		if (local.scale.x - local.scale.y).abs() >= 1e-4 {
			return Err("ring left the circle".into());
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
