//! Deterministic title-sequence clock: arcing star, pulsing tail, celestial field.

use std::f32::consts::{PI, TAU};

use bevy::math::Vec2;
use menu_components::theme::LOADING_SPIN_SECS;

/// Authored length of the promotional title sequence.
pub const DURATION_SECS: f32 = 5.0;

/// Default capture rate.
pub const DEFAULT_FPS: u32 = 60;

/// Default output size.
pub const DEFAULT_WIDTH: u32 = 1920;
pub const DEFAULT_HEIGHT: u32 = 1080;

/// Icon size in logical pixels. Matches the loading mark optically, a bit smaller
/// so the fallen-star path reads against 1080p.
pub const STAR_SIZE_PX: f32 = 160.0;

/// Trail copies behind the star. Sampled from the path, not a time buffer, so
/// a frame at time `t` always has the same tail.
pub const TRAIL_BEADS: usize = 24;

/// Energy motes around the tail. Deterministic from `t`, not a GPU particle sim,
/// so capture stays frame-exact with a frozen clock.
pub const SPARK_COUNT: usize = 32;

/// Twinkling field stars composited over the shader blend.
pub const FIELD_STAR_COUNT: usize = 56;

/// Soft nebula washes that sit on top of the furniture shaders.
pub const NEBULA_COUNT: usize = 4;

const FLIGHT_START: f32 = 0.12;
const FLIGHT_END: f32 = 1.85;
const BOUNCE_END: f32 = 2.55;
const LABEL_FADE_START: f32 = 2.70;
const LABEL_FADE_END: f32 = 3.45;
const PULSE_PERIOD: f32 = 1.2;
const TRAIL_SPACING: f32 = 0.022;
const BOUNCE_HEIGHT: f32 = 0.20;
const ENERGY_HZ: f32 = 4.2;

/// High meteor arc: enter from the left, peak near the top, land bottom-right.
const STAR_P0: Vec2 = Vec2::new(-0.16, 0.36);
const STAR_P1: Vec2 = Vec2::new(0.42, -0.22);
const STAR_P2: Vec2 = Vec2::new(0.86, 0.78);

/// Named stills captured from the PNG sequence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keyframe {
	pub name: &'static str,
	pub time: f32,
}

impl Keyframe {
	pub const BACKGROUND_BLEND: Self = Self { name: "01_background_blend", time: 0.08 };
	pub const MID_ARC_TAIL: Self = Self { name: "02_mid_arc_tail", time: 0.95 };
	pub const LANDING_BOUNCE: Self = Self { name: "03_landing_bounce", time: 1.96 };
	pub const LABEL_VISIBLE: Self = Self { name: "04_label_visible", time: 3.90 };

	pub const ALL: [Self; 4] =
		[Self::BACKGROUND_BLEND, Self::MID_ARC_TAIL, Self::LANDING_BOUNCE, Self::LABEL_VISIBLE];
}

/// One title sequence. Label is the only per-run payload; motion is authored.
#[derive(Clone, Debug, PartialEq)]
pub struct TitleSequence {
	pub label: String,
	pub duration: f32,
}

impl TitleSequence {
	pub fn new(label: impl Into<String>) -> Self {
		Self { label: label.into(), duration: DURATION_SECS }
	}

	pub fn frame_count(&self, fps: u32) -> u32 {
		((self.duration * fps as f32).round() as u32).max(1)
	}

	pub fn time_of_frame(&self, frame: u32, fps: u32) -> f32 {
		let fps = fps.max(1) as f32;
		(frame as f32 / fps).min(self.duration)
	}

	pub fn frame_at(&self, time: f32, fps: u32) -> u32 {
		let fps = fps.max(1) as f32;
		let frame = (time * fps).round() as u32;
		frame.min(self.frame_count(fps as u32).saturating_sub(1))
	}

	pub fn sample(&self, time: f32) -> TitleFrame {
		let t = time.clamp(0.0, self.duration);
		TitleFrame {
			time: t,
			blend: background_blend(t),
			star: StarPose::at(t),
			label: LabelPose::at(t),
			celestial: Celestial::at(t),
		}
	}

	pub fn spin_radians_per_sec() -> f32 {
		TAU / LOADING_SPIN_SECS
	}
}

/// Sampled state for one instant of the sequence.
#[derive(Clone, Debug, PartialEq)]
pub struct TitleFrame {
	pub time: f32,
	/// 0 = only rockadder, 1 = only cosmos. Mid values mix both shaders.
	pub blend: f32,
	pub star: StarPose,
	pub label: LabelPose,
	pub celestial: Celestial,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StarPose {
	pub x: f32,
	pub y: f32,
	pub rotation: f32,
	pub scale: f32,
	pub visible: bool,
	pub spinning: bool,
	pub glow_scale: f32,
	pub glow_alpha: f32,
	pub trail: Vec<TrailBead>,
	pub sparks: Vec<SparkMote>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrailBead {
	pub x: f32,
	pub y: f32,
	pub alpha: f32,
	pub scale: f32,
	pub energy: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SparkMote {
	pub x: f32,
	pub y: f32,
	pub alpha: f32,
	pub scale: f32,
	pub warmth: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelPose {
	pub alpha: f32,
	pub pulse: f32,
	pub scale: f32,
}

/// Sky dressing over the furniture-shader blend.
#[derive(Clone, Debug, PartialEq)]
pub struct Celestial {
	pub stars: Vec<FieldStar>,
	pub nebula: Vec<NebulaWash>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldStar {
	pub x: f32,
	pub y: f32,
	pub alpha: f32,
	pub scale: f32,
	pub warmth: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NebulaWash {
	pub x: f32,
	pub y: f32,
	pub scale: f32,
	pub alpha: f32,
	pub warmth: f32,
}

impl StarPose {
	fn at(t: f32) -> Self {
		if t < FLIGHT_START {
			return Self {
				x: STAR_P0.x,
				y: STAR_P0.y,
				rotation: heading_at(0.0),
				scale: 1.0,
				visible: false,
				spinning: false,
				glow_scale: 0.0,
				glow_alpha: 0.0,
				trail: Vec::new(),
				sparks: Vec::new(),
			};
		}

		let (pos, rotation, spinning, bouncing) = if t <= FLIGHT_END {
			let u = inverse_lerp(FLIGHT_START, FLIGHT_END, t);
			let p = quadratic(STAR_P0, STAR_P1, STAR_P2, u);
			(p, heading_at(u), false, false)
		} else if t < BOUNCE_END {
			let u = inverse_lerp(FLIGHT_END, BOUNCE_END, t);
			let p = STAR_P2 + Vec2::new(0.0, -bounce_height(u));
			(p, heading_at(1.0), false, true)
		} else {
			let spin_t = t - BOUNCE_END;
			let rotation = heading_at(1.0) + TitleSequence::spin_radians_per_sec() * spin_t;
			(STAR_P2, rotation, true, false)
		};

		let live = t < BOUNCE_END + 0.45;
		let head_energy = if live { energy_wave(t, 0.0) } else { 0.12 + 0.08 * (t * 2.1).sin() };
		let trail = if live { trail_at(t) } else { Vec::new() };
		let sparks = if live { sparks_at(t) } else { Vec::new() };

		Self {
			x: pos.x,
			y: pos.y,
			rotation,
			scale: if bouncing { 1.0 + bounce_squash(t) } else { 1.0 },
			visible: true,
			spinning,
			glow_scale: 1.28 + 0.42 * head_energy,
			glow_alpha: if live { 0.10 + 0.14 * head_energy } else { 0.06 + 0.04 * head_energy },
			trail,
			sparks,
		}
	}
}

impl LabelPose {
	fn at(t: f32) -> Self {
		let fade = smoothstep(LABEL_FADE_START, LABEL_FADE_END, t);
		let pulse = if fade <= 0.0 {
			0.0
		} else {
			0.72 + 0.28 * (TAU * (t - LABEL_FADE_START) / PULSE_PERIOD).sin()
		};
		let alpha = fade * pulse.max(0.0);
		Self { alpha, pulse, scale: 1.0 + 0.045 * pulse }
	}
}

impl Celestial {
	fn at(t: f32) -> Self {
		Self { stars: field_stars_at(t), nebula: nebula_at(t) }
	}
}

fn background_blend(t: f32) -> f32 {
	// Cosmos-forward mix; rockadder stays in the frame as larger mosaic tiles.
	(0.62 + 0.22 * (t * 0.48).sin()).clamp(0.42, 0.88)
}

fn trail_at(t: f32) -> Vec<TrailBead> {
	let mut beads = Vec::with_capacity(TRAIL_BEADS);
	for i in 1..=TRAIL_BEADS {
		let behind = t - i as f32 * TRAIL_SPACING;
		if behind < FLIGHT_START {
			continue;
		}
		let pose = star_path_only(behind);
		if !pose.0 {
			continue;
		}
		let falloff = 1.0 - i as f32 / (TRAIL_BEADS as f32 + 1.0);
		let energy = energy_wave(t, i as f32);
		beads.push(TrailBead {
			x: pose.1.x,
			y: pose.1.y,
			alpha: (0.38 + 0.58 * energy) * falloff.powf(1.12),
			scale: (0.40 + 0.52 * falloff) * (0.85 + 0.35 * energy),
			energy,
		});
	}
	beads
}

fn sparks_at(t: f32) -> Vec<SparkMote> {
	let mut motes = Vec::with_capacity(SPARK_COUNT);
	for i in 0..SPARK_COUNT {
		let lag = 0.03 + hash01(i as u32 + 19) * 0.52;
		let behind = t - lag;
		if behind < FLIGHT_START {
			continue;
		}
		let pose = star_path_only(behind);
		if !pose.0 {
			continue;
		}
		let u = inverse_lerp(FLIGHT_START, FLIGHT_END, behind).clamp(0.0, 1.0);
		let tangent = quadratic_deriv(STAR_P0, STAR_P1, STAR_P2, u).normalize_or_zero();
		let perp = Vec2::new(-tangent.y, tangent.x);
		let side = hash01(i as u32 + 3) * 2.0 - 1.0;
		let breathe = (TAU * (t * 6.5 + i as f32 * 0.37)).sin();
		let offset = (0.018 + 0.055 * hash01(i as u32 + 11)) * side * (1.0 + 0.45 * breathe);
		let energy = energy_wave(t, lag / TRAIL_SPACING);
		let pop = 0.35 + 0.65 * ((breathe + 1.0) * 0.5).powf(1.8);
		let falloff = 1.0 - lag / 0.58;
		motes.push(SparkMote {
			x: pose.1.x + perp.x * offset,
			y: pose.1.y + perp.y * offset,
			alpha: (0.22 + 0.75 * energy * pop) * falloff.max(0.0),
			scale: 0.08 + 0.16 * pop * energy,
			warmth: 0.25 + 0.75 * hash01(i as u32 + 29),
		});
	}
	motes
}

fn field_stars_at(t: f32) -> Vec<FieldStar> {
	(0..FIELD_STAR_COUNT)
		.map(|i| {
			let x = 0.04 + hash01(i as u32 * 2 + 1) * 0.92;
			let y = 0.05 + hash01(i as u32 * 2 + 7) * 0.90;
			let phase = hash01(i as u32 + 41) * TAU;
			let freq = 0.7 + hash01(i as u32 + 53) * 2.4;
			let twinkle = 0.35 + 0.65 * (0.5 + 0.5 * (t * freq + phase).sin());
			let size = 0.004 + hash01(i as u32 + 17) * 0.011;
			FieldStar {
				x,
				y,
				alpha: 0.18 + 0.72 * twinkle * twinkle,
				scale: size * (0.75 + 0.55 * twinkle),
				warmth: hash01(i as u32 + 23),
			}
		})
		.collect()
}

fn nebula_at(t: f32) -> Vec<NebulaWash> {
	[
		(0.22, 0.28, 0.72, 0.14, 0.15),
		(0.68, 0.18, 0.85, 0.12, 0.55),
		(0.48, 0.62, 0.95, 0.10, 0.35),
		(0.82, 0.72, 0.58, 0.11, 0.80),
	]
	.into_iter()
	.enumerate()
	.map(|(i, (x, y, scale, alpha, warmth))| {
		let drift = 0.02 * (t * (0.18 + i as f32 * 0.07) + i as f32).sin();
		let breath = 0.82 + 0.18 * (t * 0.35 + i as f32 * 1.1).sin();
		NebulaWash {
			x: x + drift,
			y: y - drift * 0.4,
			scale: scale * breath,
			alpha: alpha * breath,
			warmth,
		}
	})
	.collect()
}

/// Path without trail recursion: `(visible, position)`.
fn star_path_only(t: f32) -> (bool, Vec2) {
	if t < FLIGHT_START {
		return (false, STAR_P0);
	}
	if t <= FLIGHT_END {
		let u = inverse_lerp(FLIGHT_START, FLIGHT_END, t);
		return (true, quadratic(STAR_P0, STAR_P1, STAR_P2, u));
	}
	if t < BOUNCE_END {
		let u = inverse_lerp(FLIGHT_END, BOUNCE_END, t);
		return (true, STAR_P2 + Vec2::new(0.0, -bounce_height(u)));
	}
	(true, STAR_P2)
}

fn heading_at(u: f32) -> f32 {
	let d = quadratic_deriv(STAR_P0, STAR_P1, STAR_P2, u.clamp(0.0, 1.0));
	d.y.atan2(d.x) + PI / 4.0
}

fn bounce_height(u: f32) -> f32 {
	let u = u.clamp(0.0, 1.0);
	let hops = (1.0 - u).powf(1.35) * (2.0 * PI * u).sin().abs();
	hops * BOUNCE_HEIGHT
}

fn bounce_squash(t: f32) -> f32 {
	let u = inverse_lerp(FLIGHT_END, BOUNCE_END, t);
	0.08 * bounce_height(u) / BOUNCE_HEIGHT
}

fn energy_wave(t: f32, along: f32) -> f32 {
	let wave = (TAU * (t * ENERGY_HZ - along * 0.18)).sin();
	0.22 + 0.78 * ((wave + 1.0) * 0.5).powf(1.65)
}

fn quadratic(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
	let u = 1.0 - t;
	p0 * (u * u) + p1 * (2.0 * u * t) + p2 * (t * t)
}

fn quadratic_deriv(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
	(p1 - p0) * (2.0 * (1.0 - t)) + (p2 - p1) * (2.0 * t)
}

fn inverse_lerp(a: f32, b: f32, v: f32) -> f32 {
	if (b - a).abs() < f32::EPSILON {
		return 0.0;
	}
	((v - a) / (b - a)).clamp(0.0, 1.0)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
	let t = inverse_lerp(edge0, edge1, x);
	t * t * (3.0 - 2.0 * t)
}

fn hash01(n: u32) -> f32 {
	let mut x = n.wrapping_mul(0x9E37_79B9);
	x ^= x >> 16;
	x = x.wrapping_mul(0x85EB_CA6B);
	x ^= x >> 13;
	x = x.wrapping_mul(0xC2B2_AE35);
	x ^= x >> 16;
	x as f32 / u32::MAX as f32
}

#[cfg(test)]
mod tests {
	use super::*;

	fn distance_to_chord(point: Vec2, a: Vec2, b: Vec2) -> f32 {
		let chord = b - a;
		let len2 = chord.length_squared();
		if len2 < f32::EPSILON {
			return point.distance(a);
		}
		let u = ((point - a).dot(chord) / len2).clamp(0.0, 1.0);
		point.distance(a + chord * u)
	}

	#[test]
	fn five_seconds_at_sixty_fps_is_three_hundred_frames() -> anyhow::Result<()> {
		let sequence = TitleSequence::new("Maybraid");
		assert_eq!(sequence.frame_count(60), 300);
		assert!((sequence.time_of_frame(0, 60) - 0.0).abs() < 1e-6);
		assert!((sequence.time_of_frame(60, 60) - 1.0).abs() < 1e-6);
		assert_eq!(sequence.frame_at(1.0, 60), 60);
		assert_eq!(sequence.duration, DURATION_SECS);
		Ok(())
	}

	#[test]
	fn opening_frame_is_background_without_star_or_label() -> anyhow::Result<()> {
		let frame = TitleSequence::new("Coming Soon").sample(Keyframe::BACKGROUND_BLEND.time);
		assert!(!frame.star.visible);
		assert!(frame.star.trail.is_empty());
		assert!(frame.star.sparks.is_empty());
		assert!(frame.label.alpha < 0.01);
		assert!(frame.blend > 0.4 && frame.blend < 0.9);
		assert_eq!(frame.celestial.stars.len(), FIELD_STAR_COUNT);
		assert_eq!(frame.celestial.nebula.len(), NEBULA_COUNT);
		Ok(())
	}

	#[test]
	fn mid_arc_tail_curves_off_the_chord_with_particles() -> anyhow::Result<()> {
		let frame = TitleSequence::new("Maybraid").sample(Keyframe::MID_ARC_TAIL.time);
		assert!(frame.star.visible);
		assert!(!frame.star.spinning);
		assert!(frame.star.trail.len() >= 10, "trail beads = {}", frame.star.trail.len());
		assert!(frame.star.sparks.len() >= 8, "sparks = {}", frame.star.sparks.len());
		assert!(frame.star.glow_alpha > 0.08);
		let head = Vec2::new(frame.star.x, frame.star.y);
		let off_chord = distance_to_chord(head, STAR_P0, STAR_P2);
		assert!(off_chord > 0.18, "arc should leave the chord; distance {off_chord} from P0–P2");
		let first = frame
			.star
			.trail
			.first()
			.copied()
			.ok_or_else(|| anyhow::anyhow!("mid-arc trail should include at least one bead"))?;
		let closest = Vec2::new(first.x, first.y).distance(head);
		let farthest = frame
			.star
			.trail
			.last()
			.map(|bead| Vec2::new(bead.x, bead.y).distance(head))
			.ok_or_else(|| anyhow::anyhow!("mid-arc trail should include a last bead"))?;
		assert!(closest < farthest);
		assert!(first.alpha > frame.star.trail[frame.star.trail.len() - 1].alpha);
		Ok(())
	}

	#[test]
	fn tail_energy_pulses_along_the_arc() -> anyhow::Result<()> {
		let sequence = TitleSequence::new("Maybraid");
		let a = sequence.sample(Keyframe::MID_ARC_TAIL.time);
		let b = sequence.sample(Keyframe::MID_ARC_TAIL.time + 0.12);
		let energy_a = a
			.star
			.trail
			.first()
			.map(|bead| bead.energy)
			.ok_or_else(|| anyhow::anyhow!("mid-arc trail should carry energy"))?;
		let energy_b = b
			.star
			.trail
			.first()
			.map(|bead| bead.energy)
			.ok_or_else(|| anyhow::anyhow!("pulsed trail should still carry energy"))?;
		assert!((energy_a - energy_b).abs() > 0.08, "energy {energy_a} vs {energy_b}");
		let spark_a = a.star.sparks.iter().map(|mote| mote.alpha).fold(0.0_f32, f32::max);
		let spark_b = b.star.sparks.iter().map(|mote| mote.alpha).fold(0.0_f32, f32::max);
		assert!(spark_a > 0.2 || spark_b > 0.2);
		Ok(())
	}

	#[test]
	fn landing_bounce_lifts_the_star_above_rest() -> anyhow::Result<()> {
		let sequence = TitleSequence::new("Maybraid");
		let bounce = sequence.sample(Keyframe::LANDING_BOUNCE.time);
		let rest = sequence.sample(BOUNCE_END + 0.2);
		assert!(bounce.star.visible);
		assert!(!bounce.star.spinning);
		assert!(rest.star.spinning);
		assert!(
			bounce.star.y + 0.04 < rest.star.y,
			"bounce y {} should sit clearly above rest y {}",
			bounce.star.y,
			rest.star.y
		);
		assert!((rest.star.x - STAR_P2.x).abs() < 1e-5);
		assert!((rest.star.y - STAR_P2.y).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn spinning_after_landing_uses_the_loading_icon_rate() -> anyhow::Result<()> {
		let sequence = TitleSequence::new("Maybraid");
		let a = sequence.sample(BOUNCE_END + 0.5);
		let b = sequence.sample(BOUNCE_END + 1.0);
		assert!(a.star.spinning && b.star.spinning);
		let delta = b.star.rotation - a.star.rotation;
		let expected = TitleSequence::spin_radians_per_sec() * 0.5;
		assert!((delta - expected).abs() < 1e-4, "spin delta {delta} != loading rate {expected}");
		assert!((TitleSequence::spin_radians_per_sec() - TAU / LOADING_SPIN_SECS).abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn centered_label_fades_in_then_pulses_with_the_configured_text() -> anyhow::Result<()> {
		let sequence = TitleSequence::new("Reliquary");
		assert_eq!(sequence.label, "Reliquary");
		let before = sequence.sample(LABEL_FADE_START - 0.1);
		let mid = sequence.sample((LABEL_FADE_START + LABEL_FADE_END) * 0.5);
		let shown = sequence.sample(Keyframe::LABEL_VISIBLE.time);
		assert!(before.label.alpha < 0.01);
		assert!(mid.label.alpha > 0.2 && mid.label.alpha < shown.label.alpha + 0.05);
		assert!(shown.label.alpha > 0.5);
		assert!(shown.label.scale > 1.0);
		let pulse_a = sequence.sample(4.0).label.alpha;
		let pulse_b = sequence.sample(4.0 + PULSE_PERIOD * 0.5).label.alpha;
		assert!((pulse_a - pulse_b).abs() > 0.05);
		Ok(())
	}

	#[test]
	fn field_stars_twinkle_in_place() -> anyhow::Result<()> {
		let sequence = TitleSequence::new("Maybraid");
		let a = sequence.sample(0.2);
		let b = sequence.sample(1.4);
		assert_eq!(a.celestial.stars.len(), FIELD_STAR_COUNT);
		let first_a = a
			.celestial
			.stars
			.first()
			.copied()
			.ok_or_else(|| anyhow::anyhow!("celestial field should include stars"))?;
		let first_b = b
			.celestial
			.stars
			.first()
			.copied()
			.ok_or_else(|| anyhow::anyhow!("celestial field should remain populated"))?;
		assert!((first_a.x - first_b.x).abs() < 1e-5);
		assert!((first_a.y - first_b.y).abs() < 1e-5);
		assert!((first_a.alpha - first_b.alpha).abs() > 0.02);
		Ok(())
	}

	#[test]
	fn keyframes_are_ordered_inside_the_duration() -> anyhow::Result<()> {
		let mut previous = -1.0;
		for key in Keyframe::ALL {
			assert!(key.time >= 0.0 && key.time < DURATION_SECS, "{}", key.name);
			assert!(key.time > previous, "{}", key.name);
			previous = key.time;
		}
		assert_eq!(Keyframe::MID_ARC_TAIL.name, "02_mid_arc_tail");
		Ok(())
	}

	#[test]
	fn sampling_is_frame_stable() -> anyhow::Result<()> {
		let sequence = TitleSequence::new("Maybraid");
		let a = sequence.sample(1.25);
		let b = sequence.sample(1.25);
		assert_eq!(a, b);
		Ok(())
	}
}
