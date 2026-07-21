//! Pre-pocket cells — [RFC-127 §3.1.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#311-pre-pocket-cells).

use crate::noise::n01;
use bevy_math::Vec2;
use procedural_common::Bounds2;

/// Default pre-pocket pitch (world units) — top of the ~400m–3km cell range.
pub const DEFAULT_PRE_POCKET_PITCH: f32 = 3000.0;

/// Discrete pocket pitches that must divide [`DEFAULT_PRE_POCKET_PITCH`].
/// Spans full pre tiles down toward ~500m (guillotine floor ≈400m).
pub const DEFAULT_POCKET_PITCHES: [f32; 4] = [3000.0, 1500.0, 750.0, 500.0];

#[derive(Debug, Clone, Copy)]
pub struct PrePocketParams {
	pub pitch: f32,
	pub origin: Vec2,
	/// Allowed pocket pitches (each must divide `pitch`).
	pub pocket_pitches: [f32; 4],
	pub seed: u32,
}

impl Default for PrePocketParams {
	fn default() -> Self {
		Self {
			pitch: DEFAULT_PRE_POCKET_PITCH,
			origin: Vec2::ZERO,
			pocket_pitches: DEFAULT_POCKET_PITCHES,
			seed: 0,
		}
	}
}

/// One world-anchored pre-pocket tile and the pocket pitch chosen inside it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrePocket {
	pub bounds: Bounds2,
	pub anchor: Vec2,
	pub pocket_pitch: f32,
	pub nx: u32,
	pub nz: u32,
}

impl PrePocket {
	/// Pre-pocket containing world point `(x, z)`.
	pub fn containing(x: f32, z: f32, params: &PrePocketParams) -> Self {
		let w = params.pitch.max(1.0);
		let i = ((x - params.origin.x) / w).floor();
		let j = ((z - params.origin.y) / w).floor();
		let anchor = Vec2::new(params.origin.x + i * w, params.origin.y + j * w);
		let pocket_pitch = choose_pocket_pitch(anchor, params);
		let nx = (w / pocket_pitch).round() as u32;
		let nz = nx;
		let bounds = Bounds2::from_xz(anchor.x, anchor.y, anchor.x + w, anchor.y + w);
		Self { bounds, anchor, pocket_pitch, nx: nx.max(1), nz: nz.max(1) }
	}

	/// Axis-aligned pocket tile `(px, pz)` inside this pre-pocket (`0..nx`, `0..nz`).
	pub fn pocket_bounds(&self, px: u32, pz: u32) -> Bounds2 {
		let px = px.min(self.nx.saturating_sub(1));
		let pz = pz.min(self.nz.saturating_sub(1));
		let x0 = self.anchor.x + px as f32 * self.pocket_pitch;
		let z0 = self.anchor.y + pz as f32 * self.pocket_pitch;
		Bounds2::from_xz(x0, z0, x0 + self.pocket_pitch, z0 + self.pocket_pitch)
	}

	/// Pocket indices covering a world point inside this pre-pocket.
	pub fn pocket_indices_at(&self, x: f32, z: f32) -> (u32, u32) {
		let px = ((x - self.anchor.x) / self.pocket_pitch)
			.floor()
			.clamp(0.0, (self.nx.saturating_sub(1)) as f32) as u32;
		let pz = ((z - self.anchor.y) / self.pocket_pitch)
			.floor()
			.clamp(0.0, (self.nz.saturating_sub(1)) as f32) as u32;
		(px, pz)
	}
}

fn choose_pocket_pitch(anchor: Vec2, params: &PrePocketParams) -> f32 {
	let ix = (anchor.x / params.pitch.max(1.0)).floor() as i32;
	let iz = (anchor.y / params.pitch.max(1.0)).floor() as i32;
	let u = n01(params.seed, 0x70C_AE70, ix, iz);
	let idx =
		((u * params.pocket_pitches.len() as f32) as usize).min(params.pocket_pitches.len() - 1);
	let pitch = params.pocket_pitches[idx];
	debug_assert!(
		(params.pitch / pitch).fract().abs() < 1e-3,
		"pocket pitch {pitch} must divide pre pitch {}",
		params.pitch
	);
	pitch
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pocket_pitch_divides_pre() -> anyhow::Result<()> {
		let params = PrePocketParams::default();
		for (i, j) in [(0, 0), (1, -2), (7, 3)] {
			let x = params.origin.x + i as f32 * params.pitch + 10.0;
			let z = params.origin.y + j as f32 * params.pitch + 10.0;
			let pre = PrePocket::containing(x, z, &params);
			let n = params.pitch / pre.pocket_pitch;
			assert!((n - n.round()).abs() < 1e-3);
			assert_eq!(pre.nx as f32, n.round());
		}
		Ok(())
	}

	#[test]
	fn pocket_tiles_cover_pre() -> anyhow::Result<()> {
		let params = PrePocketParams::default();
		let pre = PrePocket::containing(100.0, 100.0, &params);
		let mut area = 0.0;
		for px in 0..pre.nx {
			for pz in 0..pre.nz {
				let b = pre.pocket_bounds(px, pz);
				area += (b.max.x - b.min.x) * (b.max.y - b.min.y);
			}
		}
		let pre_area =
			(pre.bounds.max.x - pre.bounds.min.x) * (pre.bounds.max.y - pre.bounds.min.y);
		assert!((area - pre_area).abs() < 1.0);
		Ok(())
	}

	#[test]
	fn pitches_span_400m_to_3km() -> anyhow::Result<()> {
		let params = PrePocketParams::default();
		assert!((params.pitch - 3000.0).abs() < 1e-3);
		let min_p = params.pocket_pitches.iter().copied().fold(f32::INFINITY, f32::min);
		let max_p = params.pocket_pitches.iter().copied().fold(0.0_f32, f32::max);
		assert!(min_p <= 500.0);
		assert!(max_p >= 3000.0);
		Ok(())
	}
}
