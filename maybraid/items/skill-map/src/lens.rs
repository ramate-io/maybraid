//! Circular overlay lens and the 2D braid that rides its rim.
//!
//! Radius is the spawn-clearing (no-debraid) pocket as it appears in the map
//! camera — not a magic UV number. The braid is the apa-kan tile-rim weave:
//! closed-circle centerline, one thread per slot, opposite sines, and true
//! over/under stacking from `phase.cos()` (each strand alternates in front).
//! The interior is a hole onto the 3D world; the stained-glass map stays
//! outside the ring.

use bevy::prelude::*;

use crate::SkillMapEvent;
use crate::map::MapExtents;
use crate::overlay::{
	OverlayParams, SkillMapOverlay, SkillMapOverlayMaterial, SkillMapOverlayPane,
};
use crate::tiles::SPAWN_CLEARING_RADIUS;
use crate::user::{SkillMapMember, SkillMapUser};
use crate::viewport::map_view_extent;

/// apa-kan `TILE_BRAID_TURN_UNITS` (0.21), interpreted in overlay UV so a
/// closed rim meets itself after an integer number of twists.
pub const BRAID_TURN_PERIOD_UV: f32 = 0.21;
/// Floor from apa-kan `tile_braid_turns` (small tiles). The overlay ring is
/// larger, so the ceiling is raised to keep the weave readable.
pub const BRAID_TURNS_MIN: f32 = 4.0;
pub const BRAID_TURNS_MAX: f32 = 12.0;
/// apa-kan `TILE_BRAID_AMP / PREV_TILE_RADIUS_PX` (1.6 / 22).
pub const BRAID_AMP_FRAC: f32 = 1.6 / 22.0;
/// Slightly thinner than apa-kan's rim stroke so three threads keep air.
pub const BRAID_STROKE_FRAC: f32 = 0.038;
/// Spherical rim thickness as a fraction of the lens radius.
pub const LENS_RING_THICKNESS_FRAC: f32 = 0.055;
/// Smoothstep width into the clear interior.
pub const LENS_EDGE_FRAC: f32 = 0.012;
/// Skill-activation braid flash envelope.
pub const LENS_FLASH_SECS: f32 = 0.10;
/// Sweeping film sheen on the lens glass.
pub const LENS_FILM_SECS: f32 = 0.08;
/// Fail / Debraid: braid coverage 1 → 0.
pub const LENS_UNCURL_SECS: f32 = 0.40;
/// Fail / Debraid: braid coverage 0 → 1.
pub const LENS_RECURL_SECS: f32 = 0.46;
const BRAID_STRANDS: u32 = 3;
const BRAID_COLORS: [Vec3; 3] =
	[Vec3::new(0.82, 0.22, 0.18), Vec3::new(0.86, 0.68, 0.18), Vec3::new(0.22, 0.52, 0.86)];
const RING_BASE: Vec3 = Vec3::new(0.14, 0.11, 0.16);
const RING_LIT: Vec3 = Vec3::new(0.78, 0.86, 0.95);
const FILM_TINT: Vec3 = Vec3::new(1.0, 0.95, 0.78);
/// 12 o'clock in overlay pixel space (Y grows down). Unravel starts here.
const BRAID_BREAK: f32 = -std::f32::consts::FRAC_PI_2;
const FLARE_IDLE: f32 = 1.0;
const FLARE_UNCURL: f32 = 1.0;
const FLARE_RECURL: f32 = 0.42;

/// How the rim braid is travelling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverlayBraidMotion {
	#[default]
	Idle,
	Uncurling,
	Recurling,
}

/// Per-user overlay lens: clear circular interior, braid on the circumference.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct SkillMapOverlayLens {
	curl: f32,
	flash: f32,
	film: f32,
	motion: OverlayBraidMotion,
}

impl Default for SkillMapOverlayLens {
	fn default() -> Self {
		Self { curl: 1.0, flash: 0.0, film: 0.0, motion: OverlayBraidMotion::Idle }
	}
}

impl MapExtents {
	/// World half-extent of the land pocket that never Debraids.
	///
	/// Chebyshev radius [`SPAWN_CLEARING_RADIUS`] plus the outer half-tile so
	/// the circle meets the pocket's land / water boundary.
	pub fn spawn_clearing_half_extent(self) -> f32 {
		(SPAWN_CLEARING_RADIUS as f32 + 0.5) * self.tile_size().x
	}
}

impl SkillMapOverlayLens {
	pub fn curl(self) -> f32 {
		self.curl
	}

	pub fn flash(self) -> f32 {
		self.flash
	}

	pub fn film(self) -> f32 {
		self.film
	}

	pub fn motion(self) -> OverlayBraidMotion {
		self.motion
	}

	pub fn is_idle(self) -> bool {
		self.motion == OverlayBraidMotion::Idle && self.curl >= 0.999 && self.flash <= 0.001
	}

	pub fn is_flashing(self) -> bool {
		self.flash > 0.05 || self.film > 0.05
	}

	pub fn is_uncurling(self) -> bool {
		self.motion == OverlayBraidMotion::Uncurling
	}

	pub fn is_recurling(self) -> bool {
		self.motion == OverlayBraidMotion::Recurling
	}

	/// Extra tip splay: more while unravelling, less while re-plaiting.
	pub fn flare(self) -> f32 {
		match self.motion {
			OverlayBraidMotion::Idle => FLARE_IDLE,
			OverlayBraidMotion::Uncurling => FLARE_UNCURL,
			OverlayBraidMotion::Recurling => FLARE_RECURL,
		}
	}

	/// Overlay UV radius (`length(uv - 0.5)` on a square) matching the
	/// spawn-clearing pocket in the map camera.
	pub fn uv_radius() -> f32 {
		let world = MapExtents::default().spawn_clearing_half_extent();
		world / map_view_extent().x.max(1e-5)
	}

	/// Lens radius as a fraction of the overlay's short axis (height on 16:9).
	pub fn min_axis_radius_frac() -> f32 {
		Self::uv_radius()
	}

	/// Integer twists so a closed weave meets itself (apa-kan `tile_braid_turns`).
	pub fn braid_turns() -> f32 {
		let circ = std::f32::consts::TAU * Self::uv_radius();
		(circ / BRAID_TURN_PERIOD_UV).clamp(BRAID_TURNS_MIN, BRAID_TURNS_MAX).round()
	}

	pub fn reset_idle(&mut self) {
		*self = Self::default();
	}

	/// Skill activation: braid flash plus a film sheen over the lens.
	pub fn flash_activation(&mut self) {
		self.flash = 1.0;
		self.film = 1.0;
	}

	/// Water / Debraid: uncurl then recurl around the circumference.
	pub fn uncurl_from_fail(&mut self) {
		self.motion = OverlayBraidMotion::Uncurling;
	}

	pub fn tick(&mut self, dt: f32) {
		if dt > f32::EPSILON {
			self.flash = (self.flash - dt / LENS_FLASH_SECS).max(0.0);
			self.film = (self.film - dt / LENS_FILM_SECS).max(0.0);
		}
		match self.motion {
			OverlayBraidMotion::Idle => {}
			OverlayBraidMotion::Uncurling => {
				self.curl = (self.curl - dt / LENS_UNCURL_SECS).max(0.0);
				if self.curl <= 0.0 {
					self.curl = 0.0;
					self.motion = OverlayBraidMotion::Recurling;
				}
			}
			OverlayBraidMotion::Recurling => {
				self.curl = (self.curl + dt / LENS_RECURL_SECS).min(1.0);
				if self.curl >= 1.0 {
					self.curl = 1.0;
					self.motion = OverlayBraidMotion::Idle;
				}
			}
		}
	}

	/// Mid-unravel snapshot used by captures and tests.
	pub fn mid_uncurl() -> Self {
		Self { curl: 0.32, flash: 0.0, film: 0.0, motion: OverlayBraidMotion::Uncurling }
	}

	/// Mid re-plait snapshot used by captures and tests.
	pub fn mid_replait() -> Self {
		Self { curl: 0.78, flash: 0.0, film: 0.0, motion: OverlayBraidMotion::Recurling }
	}

	pub fn peak_flash() -> Self {
		Self { curl: 1.0, flash: 1.0, film: 1.0, motion: OverlayBraidMotion::Idle }
	}

	/// Flash pose with the sheen band across the lens (film mid-sweep).
	pub fn sweep_flash() -> Self {
		Self { curl: 1.0, flash: 1.0, film: 0.50, motion: OverlayBraidMotion::Idle }
	}
}

/// CPU sample of the rim braid (apa-kan opposite-sine threads).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OverlayBraidSample {
	pub alpha: f32,
	pub color: Vec3,
	pub front: f32,
}

impl OverlayParams {
	pub fn from_overlay_lens(overlay: SkillMapOverlay, lens: SkillMapOverlayLens) -> Self {
		Self {
			amount: overlay.amount(),
			warp: crate::overlay::OVERLAY_WARP,
			glass: crate::overlay::OVERLAY_GLASS,
			lens_radius: SkillMapOverlayLens::uv_radius(),
			curl: lens.curl(),
			flash: lens.flash(),
			film: lens.film(),
			braid_turns: SkillMapOverlayLens::braid_turns(),
			flare: lens.flare(),
			_pad0: 0.0,
			_pad1: 0.0,
			_pad2: 0.0,
		}
	}

	/// Pixel offset from the pane centre. A circle in this space is circular
	/// on screen even when the overlay is widescreen.
	pub fn pixel_from_center(uv: Vec2, size: Vec2) -> Vec2 {
		(uv - Vec2::splat(0.5)) * size.max(Vec2::ONE)
	}

	pub fn lens_radius_px(self, size: Vec2) -> f32 {
		self.lens_radius * size.max(Vec2::ONE).min_element()
	}

	pub fn polar(self, uv: Vec2, size: Vec2) -> (f32, f32) {
		let p = Self::pixel_from_center(uv, size);
		(p.length(), p.y.atan2(p.x))
	}

	fn wrap_pi(a: f32) -> f32 {
		a.sin().atan2(a.cos())
	}

	/// UV on the rim at `theta` (atan2, Y down), slightly outside the ring so
	/// the sample sits on the braid centreline.
	pub fn rim_uv(self, theta: f32, size: Vec2) -> Vec2 {
		let radius = self.lens_radius_px(size);
		let ring_thick = (radius * LENS_RING_THICKNESS_FRAC).max(1.0);
		self.uv_at_polar(theta, radius + ring_thick * 0.65, size)
	}

	pub fn uv_at_polar(self, theta: f32, radius_px: f32, size: Vec2) -> Vec2 {
		let p = Vec2::new(theta.cos(), theta.sin()) * radius_px;
		p / size.max(Vec2::ONE) + Vec2::splat(0.5)
	}

	fn strand_radial(
		self,
		theta: f32,
		i: u32,
		radius: f32,
		amp: f32,
		braid_radius: f32,
		size: Vec2,
	) -> f32 {
		let offset = std::f32::consts::TAU * i as f32 / BRAID_STRANDS as f32;
		let phase = self.braid_turns * theta + offset;
		let woven = amp * phase.sin();
		let loosen = 1.0 - self.curl.clamp(0.0, 1.0);
		let from_break = Self::wrap_pi(theta - BRAID_BREAK).abs();
		let peel_width = loosen * std::f32::consts::PI + 1e-4;
		let peel = (1.0 - (from_break / peel_width).clamp(0.0, 1.0)).powf(1.35) * loosen;
		let recurl = self.flare < 0.85;
		let unplait = peel * if recurl { 0.22 } else { 1.0 };
		let idle_r = braid_radius + woven;
		if unplait <= 0.01 {
			return idle_r;
		}
		let ring_thick = (radius * LENS_RING_THICKNESS_FRAC).max(1.0);
		let stroke = (radius * BRAID_STROKE_FRAC).max(1.0);
		let inner = (radius + ring_thick * 0.20).max(braid_radius);
		let half = size.min_element() * 0.5;
		let outer = (half - stroke - 2.0).max(inner);
		let t = (0.16 + i as f32 * 0.34).clamp(0.0, 1.0);
		let parked = inner * (1.0 - t) + outer * t;
		let flare = peel * (outer - inner) * 0.12 * self.flare;
		let loose_r = (parked + flare).clamp(inner, outer);
		let park_amt = (unplait / 0.35).clamp(0.0, 1.0);
		let radial = idle_r * (1.0 - park_amt) + loose_r * park_amt;
		radial.clamp(inner, outer)
	}

	/// Strand in front at `theta` by apa-kan `phase.cos()` (true over/under).
	pub fn braid_front_strand(self, theta: f32) -> usize {
		let mut best = 0usize;
		let mut best_front = f32::NEG_INFINITY;
		for i in 0..BRAID_STRANDS {
			let offset = std::f32::consts::TAU * i as f32 / BRAID_STRANDS as f32;
			let front = (self.braid_turns * theta + offset).cos();
			if front > best_front {
				best_front = front;
				best = i as usize;
			}
		}
		best
	}

	/// 1 inside the clear lens, 0 outside. Keep in sync with `overlay_material.wesl`.
	pub fn lens_mask(self, uv: Vec2, size: Vec2) -> f32 {
		if self.amount <= 0.0 {
			return 0.0;
		}
		let (r, _) = self.polar(uv, size);
		let radius = self.lens_radius_px(size);
		let edge = (radius * LENS_EDGE_FRAC).max(1e-4);
		let t = ((r - (radius - edge)) / edge).clamp(0.0, 1.0);
		1.0 - t * t * (3.0 - 2.0 * t)
	}

	pub fn interior_tinted(self, uv: Vec2, size: Vec2) -> bool {
		self.lens_mask(uv, size) < 0.5
	}

	pub fn ring_alpha(self, uv: Vec2, size: Vec2) -> f32 {
		if self.amount <= 0.0 {
			return 0.0;
		}
		let (r, _) = self.polar(uv, size);
		let radius = self.lens_radius_px(size);
		let thick = (radius * LENS_RING_THICKNESS_FRAC).max(1.0);
		let d = (r - radius).abs();
		let span = (thick * 0.65).max(1e-4);
		let t = ((d - thick * 0.35) / span).clamp(0.0, 1.0);
		(1.0 - t * t * (3.0 - 2.0 * t)) * self.amount
	}

	pub fn braid_sample(self, uv: Vec2, size: Vec2) -> OverlayBraidSample {
		if self.amount <= 0.0 {
			return OverlayBraidSample::default();
		}
		let (r, theta) = self.polar(uv, size);
		let radius = self.lens_radius_px(size);
		let ring_thick = (radius * LENS_RING_THICKNESS_FRAC).max(1.0);
		let amp = radius * BRAID_AMP_FRAC;
		let stroke = (radius * BRAID_STROKE_FRAC).max(1.0);
		let braid_radius = radius + ring_thick * 0.65;
		let mut hits = [(0.0_f32, Vec3::ZERO, -2.0_f32); 3];
		for i in 0..BRAID_STRANDS {
			let offset = std::f32::consts::TAU * i as f32 / BRAID_STRANDS as f32;
			let phase = self.braid_turns * theta + offset;
			let radial = self.strand_radial(theta, i, radius, amp, braid_radius, size);
			let dist = (r - radial).abs();
			let body = 1.0 - Self::smoothstep(stroke * 0.42, stroke, dist);
			let tube = 0.62 + 0.38 * (1.0 - (dist / stroke.max(1e-4)).clamp(0.0, 1.0));
			let alpha = body * self.amount;
			if alpha > 0.01 {
				let core = BRAID_COLORS[i as usize];
				let body_col = core * tube;
				let flash_add = (core * 1.20 + FILM_TINT) * self.flash * 0.90;
				hits[i as usize] = (alpha, body_col + flash_add, phase.cos());
			}
		}
		hits.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
		let mut rgb = Vec3::ZERO;
		let mut out_a = 0.0;
		let mut front = -2.0;
		for (alpha, color, depth) in hits {
			if alpha > 0.01 {
				rgb = rgb.lerp(color, alpha.clamp(0.0, 1.0));
				out_a = alpha + out_a * (1.0 - alpha);
				front = depth;
			}
		}
		OverlayBraidSample { alpha: out_a, color: rgb, front }
	}

	pub fn film_alpha(self, uv: Vec2, size: Vec2) -> f32 {
		self.film_sweep(uv, size)
	}

	/// Additive sheen weight. A bright band sweeps left→right as `film` decays.
	pub fn film_sweep(self, uv: Vec2, size: Vec2) -> f32 {
		if self.film <= 0.0 || self.amount <= 0.0 {
			return 0.0;
		}
		let lens = self.lens_mask(uv, size);
		let p = Self::pixel_from_center(uv, size);
		let radius = self.lens_radius_px(size).max(1.0);
		let sweep = -0.82 + 1.64 * (1.0 - self.film);
		let nx = p.x / radius;
		let band = (-((nx - sweep) * 3.4).powi(2)).exp();
		(0.22 + 0.95 * band) * self.film * self.amount * lens
	}

	pub fn film_rgb(self, uv: Vec2, size: Vec2) -> Vec3 {
		FILM_TINT * self.film_sweep(uv, size)
	}

	fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
		let t = ((x - edge0) / (edge1 - edge0).max(1e-5)).clamp(0.0, 1.0);
		t * t * (3.0 - 2.0 * t)
	}

	/// CPU preview of the expanded overlay look (not a GPU readback).
	pub fn preview_rgba(self, width: u32, height: u32) -> Vec<u8> {
		let size = Vec2::new(width as f32, height as f32);
		let mut pixels = vec![0_u8; width as usize * height as usize * 4];
		for y in 0..height {
			let v = (y as f32 + 0.5) / height as f32;
			for x in 0..width {
				let u = (x as f32 + 0.5) / width as f32;
				let uv = Vec2::new(u, v);
				let rgb = self.preview_pixel(uv, size);
				let i = ((y * width + x) * 4) as usize;
				pixels[i] = (rgb.x * 255.0).round().clamp(0.0, 255.0) as u8;
				pixels[i + 1] = (rgb.y * 255.0).round().clamp(0.0, 255.0) as u8;
				pixels[i + 2] = (rgb.z * 255.0).round().clamp(0.0, 255.0) as u8;
				pixels[i + 3] = 255;
			}
		}
		pixels
	}

	fn preview_pixel(self, uv: Vec2, size: Vec2) -> Vec3 {
		let world = Self::preview_world(uv);
		let map = Self::preview_skill_map(uv);
		let lens = self.lens_mask(uv, size);
		let mut rgb = world.lerp(map, (1.0 - lens) * self.amount);
		let ring = self.ring_alpha(uv, size);
		if ring > 0.01 {
			let p = Self::pixel_from_center(uv, size);
			let n = p.normalize_or_zero();
			let light = (n.dot(Vec2::new(-0.35, 0.72)) * 0.5 + 0.5).clamp(0.0, 1.0);
			let spec = light.powf(18.0);
			let rim = RING_BASE.lerp(RING_LIT, light * 0.7) + Vec3::splat(spec * 0.45);
			rgb = rgb.lerp(rim, ring);
		}
		let braid = self.braid_sample(uv, size);
		if braid.alpha > 0.01 {
			rgb = rgb.lerp(braid.color, braid.alpha.clamp(0.0, 1.0));
		}
		let film = self.film_rgb(uv, size);
		(rgb + film).min(Vec3::ONE)
	}

	fn preview_world(uv: Vec2) -> Vec3 {
		let sky = Vec3::new(0.42, 0.62, 0.88);
		let ground = Vec3::new(0.28, 0.46, 0.22);
		let horizon = Self::smoothstep(0.42, 0.58, uv.y);
		let mut rgb = sky.lerp(ground, horizon);
		let cube = uv - Vec2::new(0.50, 0.54);
		if cube.x.abs() < 0.055 && cube.y.abs() < 0.08 {
			rgb = Vec3::new(0.78, 0.22, 0.18);
		}
		rgb
	}

	fn preview_skill_map(uv: Vec2) -> Vec3 {
		let cell = Vec2::new((uv.x * 14.0).floor(), (uv.y * 8.0).floor());
		let n = ((cell.x * 13.0 + cell.y * 7.0).sin() * 0.5 + 0.5).abs();
		let pane = if n > 0.82 {
			Vec3::new(0.82, 0.18, 0.78)
		} else if n < 0.16 {
			Vec3::new(0.16, 0.42, 0.78)
		} else {
			Vec3::new(0.58 + 0.12 * n, 0.22 + 0.08 * (1.0 - n), 0.14)
		};
		let came = {
			let f = Vec2::new((uv.x * 14.0).fract(), (uv.y * 8.0).fract());
			let edge = (1.0 - (f.x.min(1.0 - f.x) * 18.0).clamp(0.0, 1.0))
				.max(1.0 - (f.y.min(1.0 - f.y) * 18.0).clamp(0.0, 1.0));
			edge
		};
		pane * (1.0 - came * 0.55)
	}

	pub fn write_preview_png(
		self,
		path: impl AsRef<std::path::Path>,
		width: u32,
		height: u32,
	) -> Result<(), String> {
		let rgba = self.preview_rgba(width, height);
		let bytes = screen_capture::encode_rgba_png(width, height, &rgba)?;
		screen_capture::write_all_atomic(path.as_ref(), &bytes)
	}
}

pub(crate) fn apply_overlay_lens_events(
	mut events: MessageReader<SkillMapEvent>,
	mut lenses: Query<&mut SkillMapOverlayLens>,
) {
	for event in events.read() {
		match *event {
			SkillMapEvent::Claim { user, .. } => {
				if let Ok(mut lens) = lenses.get_mut(user) {
					lens.flash_activation();
				}
			}
			SkillMapEvent::Fail { user, .. } => {
				if let Ok(mut lens) = lenses.get_mut(user) {
					lens.uncurl_from_fail();
				}
			}
		}
	}
}

pub(crate) fn tick_overlay_lens(time: Res<Time>, mut lenses: Query<&mut SkillMapOverlayLens>) {
	let dt = time.delta_secs();
	for mut lens in &mut lenses {
		lens.tick(dt);
	}
}

pub(crate) fn apply_overlay_lens_material(
	users: Query<(&SkillMapUser, &SkillMapOverlay, &SkillMapOverlayLens)>,
	panes: Query<(&SkillMapMember, &SkillMapOverlayPane)>,
	mut overlay_materials: Option<ResMut<Assets<SkillMapOverlayMaterial>>>,
) {
	let Some(materials) = overlay_materials.as_deref_mut() else {
		return;
	};
	for (user, overlay, lens) in &users {
		for (member, pane) in &panes {
			if member.session != user.maps {
				continue;
			}
			if let Some(mut material) = materials.get_mut(&pane.material) {
				material.set_overlay_lens(*overlay, *lens);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::overlay::SkillMapOverlay;

	fn square() -> Vec2 {
		Vec2::splat(512.0)
	}

	fn idle_params() -> OverlayParams {
		OverlayParams::from_overlay_lens(SkillMapOverlay::Expanded, SkillMapOverlayLens::default())
	}

	#[test]
	fn lens_radius_is_the_spawn_clearing_pocket() -> anyhow::Result<()> {
		let extents = MapExtents::default();
		let world = extents.spawn_clearing_half_extent();
		assert!(
			(world - (SPAWN_CLEARING_RADIUS as f32 + 0.5) * extents.tile_size().x).abs() < 1e-5
		);
		let uv = SkillMapOverlayLens::uv_radius();
		assert!((uv - world / map_view_extent().x).abs() < 1e-6);
		assert!((SkillMapOverlayLens::min_axis_radius_frac() - uv).abs() < 1e-6);
		assert!(uv > 0.30, "pocket should fill a large central disk, got {uv}");
		assert!(uv < 0.50, "ring must sit inside the mid-edge, got {uv}");
		// 16:9 overlay: short axis is height. 1080p radius is this fraction of 1080.
		let radius_1080 = SkillMapOverlayLens::min_axis_radius_frac() * 1080.0;
		assert!(
			(radius_1080 - 473.684_2).abs() < 0.05,
			"1080p pocket radius should be ~473.7px, got {radius_1080}"
		);
		Ok(())
	}

	#[test]
	fn braid_turns_are_integer_and_closed() -> anyhow::Result<()> {
		let turns = SkillMapOverlayLens::braid_turns();
		assert!((turns - turns.round()).abs() < 1e-5);
		assert!(turns >= BRAID_TURNS_MIN);
		assert!(turns <= BRAID_TURNS_MAX);
		Ok(())
	}

	#[test]
	fn expanded_interior_is_not_tinted_and_the_rim_holds_a_braid() -> anyhow::Result<()> {
		let params = idle_params();
		let size = square();
		assert!(
			!params.interior_tinted(Vec2::splat(0.5), size),
			"centre of the lens must stay clear"
		);
		assert!(
			params.interior_tinted(Vec2::ZERO, size),
			"corners stay in the stained-glass field"
		);
		assert!(params.lens_mask(Vec2::splat(0.5), size) > 0.95);
		let radius = params.lens_radius;
		let rim = Vec2::new(0.5 + radius, 0.5);
		let braid_uv = Vec2::new(0.5 + radius * 1.08, 0.5);
		assert!(params.ring_alpha(rim, size) > 0.4, "spherical ring sits on the pocket radius");
		assert!(params.braid_sample(braid_uv, size).alpha > 0.15, "idle braid surrounds the ring");
		Ok(())
	}

	#[test]
	fn two_slot_sines_are_opposites_like_apa_kan() -> anyhow::Result<()> {
		let theta = 0.35;
		let turns = SkillMapOverlayLens::braid_turns();
		let a = turns * theta;
		let b = a + std::f32::consts::PI;
		assert!((a.sin() + b.sin()).abs() < 1e-4, "two-slot braid is the apa-kan +sin / −sin pair");
		let third = std::f32::consts::TAU / 3.0;
		assert!((third * 3.0 - std::f32::consts::TAU).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn activation_flashes_then_decays() -> anyhow::Result<()> {
		let mut lens = SkillMapOverlayLens::default();
		lens.flash_activation();
		assert!(lens.is_flashing());
		assert!((lens.flash() - 1.0).abs() < 1e-5);
		assert!((lens.film() - 1.0).abs() < 1e-5);
		lens.tick(LENS_FILM_SECS * 0.5);
		assert!(lens.film() > 0.4 && lens.film() < 0.6);
		lens.tick(LENS_FLASH_SECS);
		assert!(lens.flash() < 0.05);
		assert!(!lens.is_flashing() || lens.flash() < 0.05);
		Ok(())
	}

	#[test]
	fn fail_uncurls_then_recurls() -> anyhow::Result<()> {
		let mut lens = SkillMapOverlayLens::default();
		lens.uncurl_from_fail();
		assert!(lens.is_uncurling());
		lens.tick(LENS_UNCURL_SECS * 0.5);
		assert!(lens.is_uncurling());
		assert!((lens.curl() - 0.5).abs() < 0.08, "mid-uncurl curl, got {}", lens.curl());
		lens.tick(LENS_UNCURL_SECS);
		assert!(lens.is_recurling());
		assert!(lens.curl() < 0.15);
		lens.tick(LENS_RECURL_SECS);
		assert!(lens.is_idle());
		assert!((lens.curl() - 1.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn idle_braid_closes_at_three_oclock() -> anyhow::Result<()> {
		let params = idle_params();
		let size = square();
		let east = params.braid_sample(params.rim_uv(0.0, size), size);
		let west = params.braid_sample(params.rim_uv(std::f32::consts::PI, size), size);
		assert!(east.alpha > 0.15, "3 o'clock must stay on the closed loop, got {}", east.alpha);
		assert!(west.alpha > 0.15, "9 o'clock stays on the loop, got {}", west.alpha);
		assert!(
			(east.alpha - west.alpha).abs() < 0.35,
			"the old 3 o'clock seam must not drop out, east={} west={}",
			east.alpha,
			west.alpha
		);
		Ok(())
	}

	#[test]
	fn over_under_puts_the_front_strand_on_top() -> anyhow::Result<()> {
		let params = idle_params();
		let size = square();
		// Gold (1) crosses blue (2) at the same radius; gold's cos is in front.
		let theta = -std::f32::consts::FRAC_PI_2 / params.braid_turns;
		assert_eq!(params.braid_front_strand(theta), 1);
		let radius = params.lens_radius_px(size);
		let amp = radius * BRAID_AMP_FRAC;
		let braid_radius = radius + (radius * LENS_RING_THICKNESS_FRAC).max(1.0) * 0.65;
		let r = params.strand_radial(theta, 1, radius, amp, braid_radius, size);
		let sample = params.braid_sample(params.uv_at_polar(theta, r, size), size);
		assert!(sample.alpha > 0.2, "crossing at {theta} should hit the braid");
		assert!(
			sample.color.y > 0.40,
			"gold (strand 1) must sit over red when it is in front, color={}",
			sample.color
		);
		Ok(())
	}

	#[test]
	fn fail_unravels_from_the_break_without_clipping_the_far_arc() -> anyhow::Result<()> {
		let params = OverlayParams::from_overlay_lens(
			SkillMapOverlay::Expanded,
			SkillMapOverlayLens::mid_uncurl(),
		);
		let size = square();
		let bottom =
			params.braid_sample(params.rim_uv(BRAID_BREAK + std::f32::consts::PI, size), size);
		assert!(bottom.alpha > 0.15, "the far arc stays plaited, got {}", bottom.alpha);
		let radius = params.lens_radius_px(size);
		let amp = radius * BRAID_AMP_FRAC;
		let braid_radius = radius + (radius * LENS_RING_THICKNESS_FRAC).max(1.0) * 0.65;
		let mut tip_hits = 0;
		for i in 0..BRAID_STRANDS {
			let r = params.strand_radial(BRAID_BREAK, i, radius, amp, braid_radius, size);
			assert!(
				r >= radius,
				"unravelled strand {i} must stay outside the lens, r={r} radius={radius}"
			);
			assert!(
				r < size.min_element() * 0.5 - 1.0,
				"unravelled strand {i} must stay on-screen, r={r} half={}",
				size.min_element() * 0.5
			);
			let sample = params.braid_sample(params.uv_at_polar(BRAID_BREAK, r, size), size);
			if sample.alpha > 0.05 {
				tip_hits += 1;
			}
		}
		assert!(tip_hits >= 2, "unravelled tips stay visible at the break, hits={tip_hits}");
		let stroke = (radius * BRAID_STROKE_FRAC).max(1.0);
		let spread = (0..BRAID_STRANDS)
			.map(|i| params.strand_radial(BRAID_BREAK, i, radius, amp, braid_radius, size))
			.fold(0.0_f32, |acc, r| acc.max(r))
			- (0..BRAID_STRANDS)
				.map(|i| params.strand_radial(BRAID_BREAK, i, radius, amp, braid_radius, size))
				.fold(f32::MAX, |acc, r| acc.min(r));
		assert!(
			spread > stroke * 0.7,
			"unravelled strands occupy distinct radii in the annulus, spread={spread} stroke={stroke}"
		);
		Ok(())
	}

	#[test]
	fn recurl_replaits_tighter_than_the_unravel() -> anyhow::Result<()> {
		let open = OverlayParams::from_overlay_lens(
			SkillMapOverlay::Expanded,
			SkillMapOverlayLens::mid_uncurl(),
		);
		let shut = OverlayParams::from_overlay_lens(
			SkillMapOverlay::Expanded,
			SkillMapOverlayLens::mid_replait(),
		);
		assert!(open.flare > shut.flare);
		assert!(
			open.curl + 0.30 < shut.curl,
			"re-plait must be a later pose than unravel, open={} shut={}",
			open.curl,
			shut.curl
		);
		let size = square();
		let radius = open.lens_radius_px(size);
		let ring_thick = (radius * LENS_RING_THICKNESS_FRAC).max(1.0);
		let stroke = (radius * BRAID_STROKE_FRAC).max(1.0);
		let braid_radius = radius + ring_thick * 0.65;
		let inner = (radius + ring_thick * 0.20).max(braid_radius);
		let outer = (size.min_element() * 0.5 - stroke - 2.0).max(inner);
		let mid_annulus = inner * 0.35 + outer * 0.65;
		let open_loose = open.braid_sample(open.uv_at_polar(BRAID_BREAK, mid_annulus, size), size);
		let shut_loose = shut.braid_sample(shut.uv_at_polar(BRAID_BREAK, mid_annulus, size), size);
		assert!(
			open_loose.alpha > 0.12,
			"unravel must occupy the annulus outside the rim, got {}",
			open_loose.alpha
		);
		assert!(
			open_loose.alpha > shut_loose.alpha + 0.08,
			"re-plait must draw strands back in from the annulus, open={} shut={}",
			open_loose.alpha,
			shut_loose.alpha
		);
		Ok(())
	}

	#[test]
	fn fire_film_sweeps_a_bright_sheen() -> anyhow::Result<()> {
		let params = OverlayParams::from_overlay_lens(
			SkillMapOverlay::Expanded,
			SkillMapOverlayLens::sweep_flash(),
		);
		let size = square();
		let centre = params.film_alpha(Vec2::splat(0.5), size);
		let west = params.film_alpha(Vec2::new(0.5 - params.lens_radius * 0.45, 0.5), size);
		let east = params.film_alpha(Vec2::new(0.5 + params.lens_radius * 0.45, 0.5), size);
		assert!(centre > west + 0.15, "sweep band sits on the glass, centre={centre} west={west}");
		assert!(centre > east + 0.15, "not a full-view fade, centre={centre} east={east}");
		assert!(params.film_alpha(Vec2::new(0.04, 0.04), size) < 0.02, "film stays on the glass");
		let idle = idle_params();
		assert!(idle.film_alpha(Vec2::splat(0.5), size) < 1e-5);
		let flash = OverlayParams::from_overlay_lens(
			SkillMapOverlay::Expanded,
			SkillMapOverlayLens::peak_flash(),
		);
		let rim = flash.rim_uv(0.0, size);
		let idle_c = idle.braid_sample(rim, size).color;
		let flash_c = flash.braid_sample(rim, size).color;
		assert!(
			flash_c.max_element() > idle_c.max_element() + 0.25,
			"braid must flash brighter, idle={idle_c} flash={flash_c}"
		);
		Ok(())
	}

	#[test]
	fn collapsed_overlay_draws_no_lens() -> anyhow::Result<()> {
		let params = OverlayParams::from_overlay_lens(
			SkillMapOverlay::Collapsed,
			SkillMapOverlayLens::default(),
		);
		let size = square();
		assert_eq!(params.amount, 0.0);
		assert_eq!(params.lens_mask(Vec2::splat(0.5), size), 0.0);
		assert_eq!(params.ring_alpha(Vec2::new(0.5 + params.lens_radius, 0.5), size), 0.0);
		Ok(())
	}
}
