//! Circular overlay lens and the 2D braid that rides its rim.
//!
//! Radius is the spawn-clearing (no-debraid) pocket as it appears in the map
//! camera — not a magic UV number. The braid is the apa-kan tile-rim weave:
//! closed-circle centerline, one thread per slot, opposite sines, over/under
//! from `phase.cos()`.

use bevy::prelude::*;

use crate::map::MapExtents;
use crate::overlay::{
	OverlayParams, SkillMapOverlay, SkillMapOverlayMaterial, SkillMapOverlayPane,
};
use crate::tiles::SPAWN_CLEARING_RADIUS;
use crate::user::{SkillMapMember, SkillMapUser};
use crate::viewport::map_view_extent;
use crate::SkillMapEvent;

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
pub const LENS_FLASH_SECS: f32 = 0.22;
/// Full-view film sheen on the lens.
pub const LENS_FILM_SECS: f32 = 0.16;
/// Fail / Debraid: braid coverage 1 → 0.
pub const LENS_UNCURL_SECS: f32 = 0.40;
/// Fail / Debraid: braid coverage 0 → 1.
pub const LENS_RECURL_SECS: f32 = 0.46;
const BRAID_STRANDS: u32 = 3;
const BRAID_COLORS: [Vec3; 3] =
	[Vec3::new(0.82, 0.22, 0.18), Vec3::new(0.86, 0.68, 0.18), Vec3::new(0.22, 0.52, 0.86)];
const RING_BASE: Vec3 = Vec3::new(0.14, 0.11, 0.16);
const RING_LIT: Vec3 = Vec3::new(0.78, 0.86, 0.95);
const FILM_TINT: Vec3 = Vec3::new(1.0, 0.82, 0.45);

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

	/// Overlay UV radius (`length(uv - 0.5)` on a square) matching the
	/// spawn-clearing pocket in the map camera.
	pub fn uv_radius() -> f32 {
		let world = MapExtents::default().spawn_clearing_half_extent();
		world / map_view_extent().x.max(1e-5)
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

	/// Mid-uncurl snapshot used by captures and tests.
	pub fn mid_uncurl() -> Self {
		Self { curl: 0.5, flash: 0.0, film: 0.0, motion: OverlayBraidMotion::Uncurling }
	}

	pub fn peak_flash() -> Self {
		Self { curl: 1.0, flash: 1.0, film: 1.0, motion: OverlayBraidMotion::Idle }
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
		let r = p.length();
		let mut theta = p.y.atan2(p.x);
		if theta < 0.0 {
			theta += std::f32::consts::TAU;
		}
		(r, theta)
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
		if self.amount <= 0.0 || self.curl <= 0.001 {
			return OverlayBraidSample::default();
		}
		let (r, theta) = self.polar(uv, size);
		let radius = self.lens_radius_px(size);
		let ring_thick = (radius * LENS_RING_THICKNESS_FRAC).max(1.0);
		let amp = radius * BRAID_AMP_FRAC;
		let stroke = (radius * BRAID_STROKE_FRAC).max(1.0);
		let braid_radius = radius + ring_thick * 0.65;
		let coverage = self.curl.clamp(0.0, 1.0) * std::f32::consts::TAU;
		if theta > coverage + 0.12 {
			return OverlayBraidSample::default();
		}
		let tip = (1.0 - ((coverage - theta) / 0.45).clamp(0.0, 1.0)).clamp(0.0, 1.0);
		let peel = tip * (1.0 - self.curl) * amp * 1.6;
		let along = 1.0 - Self::smoothstep(coverage - 0.10, coverage, theta);
		let mut best = OverlayBraidSample::default();
		for i in 0..BRAID_STRANDS {
			let offset = std::f32::consts::TAU * i as f32 / BRAID_STRANDS as f32;
			let phase = self.braid_turns * theta + offset;
			let radial = braid_radius + amp * phase.sin() + peel;
			let dist = (r - radial).abs();
			let body = 1.0 - Self::smoothstep(stroke * 0.45, stroke, dist);
			let alpha = body * along * self.amount;
			if alpha <= 0.01 {
				continue;
			}
			let front = phase.cos();
			let gain = 1.0 + self.flash * 1.55;
			if alpha > best.alpha && (front >= best.front || best.alpha < 0.05) {
				best = OverlayBraidSample { alpha, color: BRAID_COLORS[i as usize] * gain, front };
			}
		}
		best
	}

	pub fn film_alpha(self, uv: Vec2, size: Vec2) -> f32 {
		if self.film <= 0.0 || self.amount <= 0.0 {
			return 0.0;
		}
		let p = Self::pixel_from_center(uv, size);
		let dir = Vec2::new(0.62, 0.38).normalize_or_zero();
		let n = (p + Vec2::splat(0.001)).normalize_or_zero();
		let sheen = ((n.dot(dir) * 0.5 + 0.5).clamp(0.0, 1.0)).powf(6.0);
		self.film * self.amount * (0.16 + 0.58 * sheen)
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
		let world = Vec3::new(0.18 + uv.x * 0.08, 0.22 + uv.y * 0.06, 0.28);
		let mosaic = Vec3::new(
			0.42 + 0.22 * (uv.x * 7.0).fract(),
			0.18 + 0.12 * (uv.y * 5.5).fract(),
			0.16 + 0.28 * ((uv.x + uv.y) * 3.1).fract(),
		);
		let lens = self.lens_mask(uv, size);
		let mut rgb = world.lerp(world * mosaic, (1.0 - lens) * self.amount * 0.72);
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
		let film = self.film_alpha(uv, size);
		if film > 0.0 {
			rgb = rgb.lerp(FILM_TINT, film);
		}
		rgb
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
		assert!(uv > 0.30, "pocket should fill a large central disk, got {uv}");
		assert!(uv < 0.50, "ring must sit inside the mid-edge, got {uv}");
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
	fn mid_uncurl_leaves_a_gap_on_the_rim() -> anyhow::Result<()> {
		let params = OverlayParams::from_overlay_lens(
			SkillMapOverlay::Expanded,
			SkillMapOverlayLens::mid_uncurl(),
		);
		let size = square();
		let radius = params.lens_radius;
		let covered = Vec2::new(0.5 + radius * 1.08, 0.5);
		let gap = Vec2::new(0.5 - radius * 1.08, 0.5);
		assert!(
			params.braid_sample(covered, size).alpha > 0.05,
			"the remaining braid still occupies the covered arc"
		);
		assert!(
			params.braid_sample(gap, size).alpha < 0.05,
			"the peeled arc must be empty at curl=0.5"
		);
		Ok(())
	}

	#[test]
	fn fire_film_tints_the_full_view() -> anyhow::Result<()> {
		let params = OverlayParams::from_overlay_lens(
			SkillMapOverlay::Expanded,
			SkillMapOverlayLens::peak_flash(),
		);
		let size = square();
		assert!(params.film_alpha(Vec2::splat(0.5), size) > 0.1);
		assert!(params.film_alpha(Vec2::new(0.12, 0.12), size) > 0.05);
		let idle = idle_params();
		assert!(idle.film_alpha(Vec2::splat(0.5), size) < 1e-5);
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
