//! Union-first hydro composition: primitives + broadphase + sample-time blend.
//!
//! Authored plans emit [`HydroPrimitive`] nodes into a
//! [`crate::complex::WatershedDepressionComplex`]. The complex modulates terrain
//! directly: each sample does continuous `min` / `smoothmin` over a
//! broadphase-culled candidate set. Rim/apron come from \(\phi_{\mathrm{union}}\)
//! with one shared [`ComplexApronParams`].

use bevy_math::{FloatExt, Vec2};
use jersey_terrain_stamps::RegionNoise;
use procedural_common::Bounds2;
use std::collections::HashMap;

/// Soft-min length scale for free-surface blending (world units).
pub const SURFACE_SMOOTHMIN_K: f32 = 1.5;
/// Default hard cap on add-only rim height noise.
pub const DEFAULT_RIM_UPLIFT_CAP: f32 = 1.5;

/// One hydraulic node: footprint + local elevation field.
#[derive(Debug, Clone)]
pub struct HydroPrimitive {
	pub footprint: HydroFootprint,
	pub elevation: HydroElevation,
	/// Extra AABB pad for broadphase / apron support (world units).
	pub influence_pad: f32,
}

/// Marazion-owned support geometry.
#[derive(Debug, Clone)]
pub enum HydroFootprint {
	/// Capsule / stadium for one reach segment.
	ReachSegment {
		a: Vec2,
		b: Vec2,
		half_width: f32,
	},
	/// Rotated elliptical disc (lake body).
	Ellipse {
		center: Vec2,
		radii: Vec2,
		rotation: f32,
	},
}

/// Depth / surface field over the footprint (local coordinates).
#[derive(Debug, Clone)]
pub enum HydroElevation {
	/// Local \(Z\) along travel, local \(X\) across channel.
	ReachProfile {
		surface_a: f32,
		surface_b: f32,
		/// Centerline depth below \(W\); transverse bowl \(D_0 P(|X|)\).
		center_depth: f32,
	},
	/// Flat \(W\); bowl in ellipse-normalized \(u\).
	RadialBowl {
		surface: f32,
		center_depth: f32,
	},
}

/// One complex-wide rim / apron policy (not per-primitive).
#[derive(Debug, Clone)]
pub struct ComplexApronParams {
	pub rim_lift: f32,
	pub rim_width: f32,
	pub apron_width: f32,
	pub rim_height: RegionNoise,
	pub rim_uplift_cap: f32,
	/// Softmask fade for fill past \(\phi=0\).
	pub shore_fade: f32,
	pub fill_undercut: f32,
}

impl Default for ComplexApronParams {
	fn default() -> Self {
		Self {
			rim_lift: 1.1,
			rim_width: 4.0,
			apron_width: 8.0,
			rim_height: RegionNoise::from_seed(0, 0.02, 0.0),
			rim_uplift_cap: DEFAULT_RIM_UPLIFT_CAP,
			shore_fade: 2.5,
			fill_undercut: 2.0,
		}
	}
}

impl ComplexApronParams {
	pub fn with_rim_noise(mut self, noise: RegionNoise, cap: f32) -> Self {
		self.rim_height = noise;
		self.rim_uplift_cap = cap.max(0.0);
		self
	}
}

/// Uniform-grid broadphase over conservative AABBs (not exact intersection bake).
#[derive(Debug, Clone)]
pub struct FootprintIndex {
	origin: Vec2,
	cell: f32,
	buckets: HashMap<(i32, i32), Vec<u16>>,
}

impl FootprintIndex {
	pub fn build(bounds: Bounds2, primitives: &[HydroPrimitive], cell: f32, support_pad: f32) -> Self {
		let cell = cell.max(1.0);
		let origin = bounds.min;
		let support_pad = support_pad.max(0.0);
		let mut buckets: HashMap<(i32, i32), Vec<u16>> = HashMap::new();
		for (i, prim) in primitives.iter().enumerate() {
			let id = i as u16;
			let (mn, mx) = prim.aabb();
			let pad = prim.influence_pad.max(0.0) + support_pad;
			let i0 = ((mn.x - pad - origin.x) / cell).floor() as i32;
			let i1 = ((mx.x + pad - origin.x) / cell).floor() as i32;
			let j0 = ((mn.y - pad - origin.y) / cell).floor() as i32;
			let j1 = ((mx.y + pad - origin.y) / cell).floor() as i32;
			for ix in i0..=i1 {
				for iz in j0..=j1 {
					buckets.entry((ix, iz)).or_default().push(id);
				}
			}
		}
		Self {
			origin,
			cell,
			buckets,
		}
	}

	pub fn candidates(&self, p: Vec2) -> &[u16] {
		let ix = ((p.x - self.origin.x) / self.cell).floor() as i32;
		let iz = ((p.y - self.origin.y) / self.cell).floor() as i32;
		self.buckets
			.get(&(ix, iz))
			.map(|v| v.as_slice())
			.unwrap_or(&[])
	}

	/// All primitive ids (for brute-force tests).
	pub fn all_ids(&self, n: usize) -> Vec<u16> {
		(0..n as u16).collect()
	}
}

impl HydroFootprint {
	pub fn sdf(&self, p: Vec2) -> f32 {
		match self {
			Self::ReachSegment { a, b, half_width } => {
				segment_distance(p, *a, *b) - half_width.max(1e-3)
			}
			Self::Ellipse {
				center,
				radii,
				rotation,
			} => ellipse_sdf(p, *center, *radii, *rotation),
		}
	}

	pub fn aabb(&self) -> (Vec2, Vec2) {
		match self {
			Self::ReachSegment { a, b, half_width } => {
				let hw = half_width.max(1e-3);
				let mn = Vec2::new(a.x.min(b.x), a.y.min(b.y)) - Vec2::splat(hw);
				let mx = Vec2::new(a.x.max(b.x), a.y.max(b.y)) + Vec2::splat(hw);
				(mn, mx)
			}
			Self::Ellipse {
				center,
				radii,
				rotation,
			} => {
				// Conservative AABB of rotated ellipse.
				let (s, c) = rotation.sin_cos();
				let rx = radii.x.max(1e-3);
				let rz = radii.y.max(1e-3);
				let ex = (c * rx).abs() + (s * rz).abs();
				let ez = (s * rx).abs() + (c * rz).abs();
				(*center - Vec2::new(ex, ez), *center + Vec2::new(ex, ez))
			}
		}
	}
}

impl HydroPrimitive {
	pub fn aabb(&self) -> (Vec2, Vec2) {
		self.footprint.aabb()
	}

	pub fn phi(&self, p: Vec2) -> f32 {
		self.footprint.sdf(p)
	}

	/// Local-frame surface and bed at `p` (valid even slightly outside; caller masks).
	pub fn surface_and_bed(&self, p: Vec2) -> (f32, f32) {
		match (&self.footprint, &self.elevation) {
			(
				HydroFootprint::ReachSegment { a, b, half_width },
				HydroElevation::ReachProfile {
					surface_a,
					surface_b,
					center_depth,
				},
			) => {
				let (z, x_signed) = reach_frame(p, *a, *b);
				let w = surface_a + (surface_b - surface_a) * z;
				let xn = (x_signed.abs() / half_width.max(1e-3)).clamp(0.0, 1.0);
				let depth = center_depth.max(0.0) * transverse_bowl(xn);
				(w, w - depth)
			}
			(
				HydroFootprint::Ellipse {
					center,
					radii,
					rotation,
				},
				HydroElevation::RadialBowl {
					surface,
					center_depth,
				},
			) => {
				let u = ellipse_radial_norm(p, *center, *radii, *rotation).clamp(0.0, 1.0);
				let depth = center_depth.max(0.0) * transverse_bowl(u);
				(*surface, *surface - depth)
			}
			// Mismatched footprint/elevation: fall back to flat mid values.
			(_, HydroElevation::ReachProfile { surface_a, surface_b, center_depth }) => {
				let w = 0.5 * (surface_a + surface_b);
				(w, w - center_depth.max(0.0))
			}
			(_, HydroElevation::RadialBowl { surface, center_depth }) => {
				(*surface, *surface - center_depth.max(0.0))
			}
		}
	}
}

/// Prepared hydro complex: sample-time modulation + shared fill surface.
#[derive(Debug, Clone)]
pub struct PreparedHydroComplex {
	pub bounds: Bounds2,
	pub seed: u32,
	pub primitives: Vec<HydroPrimitive>,
	pub index: FootprintIndex,
	pub apron: ComplexApronParams,
}

impl PreparedHydroComplex {
	pub fn prepare(
		bounds: Bounds2,
		seed: u32,
		primitives: Vec<HydroPrimitive>,
		apron: ComplexApronParams,
	) -> Self {
		let short = bounds.extent().min_element().max(1.0);
		let cell = (short * 0.08).clamp(8.0, 64.0);
		let support = apron.rim_width + apron.apron_width;
		let index = FootprintIndex::build(bounds, &primitives, cell, support);
		Self {
			bounds,
			seed,
			primitives,
			index,
			apron,
		}
	}

	pub fn is_empty(&self) -> bool {
		self.primitives.is_empty()
	}

	/// Candidates from broadphase (deduped).
	pub fn candidate_ids(&self, p: Vec2) -> Vec<u16> {
		let raw = self.index.candidates(p);
		if raw.is_empty() {
			return Vec::new();
		}
		let mut ids = raw.to_vec();
		ids.sort_unstable();
		ids.dedup();
		ids
	}

	/// Fold \(\phi\), bed, \(W\) over candidates (or all if `use_index` is false).
	pub fn fold_fields(&self, p: Vec2, use_index: bool) -> Option<(f32, f32, f32)> {
		let ids: Vec<u16> = if use_index {
			self.candidate_ids(p)
		} else {
			self.index.all_ids(self.primitives.len())
		};
		if ids.is_empty() {
			return None;
		}
		let pad = self.apron.rim_width + self.apron.apron_width;
		let mut phi = f32::INFINITY;
		let mut bed = f32::INFINITY;
		let mut surfaces: Vec<f32> = Vec::new();
		for &id in &ids {
			let Some(prim) = self.primitives.get(id as usize) else {
				continue;
			};
			let d = prim.phi(p);
			if d > pad + prim.influence_pad {
				continue;
			}
			phi = phi.min(d);
			let (w, b) = prim.surface_and_bed(p);
			// Bed only unions inside wet footprints; surface also feeds apron banks.
			if d <= 0.0 {
				bed = bed.min(b);
				surfaces.push(w);
			} else if d < pad.max(1.0) {
				surfaces.push(w);
			}
		}
		if !phi.is_finite() {
			return None;
		}
		let w = if surfaces.is_empty() {
			bed
		} else {
			smoothmin_fold(&surfaces, SURFACE_SMOOTHMIN_K)
		};
		let bed = if bed.is_finite() { bed } else { w };
		Some((phi, bed, w))
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		// Hard identity outside leaf AABB (+ small ease handled by caller bind if any).
		if p.x < self.bounds.min.x
			|| p.x > self.bounds.max.x
			|| p.y < self.bounds.min.y
			|| p.y > self.bounds.max.y
		{
			return elevation;
		}
		let Some((phi, bed, w)) = self.fold_fields(p, true) else {
			return elevation;
		};
		let rim_w = self.apron.rim_width.max(0.25);
		let apron_w = self.apron.apron_width.max(0.25);
		let bank = w + self.apron.rim_lift.max(0.0);
		let mut rim_noise = self.apron.rim_height.sample_height(p).abs();
		rim_noise = rim_noise.min(self.apron.rim_uplift_cap.max(0.0));
		let bank = bank + rim_noise;

		if phi <= 0.0 {
			// Interior: depression-only toward union bed.
			return elevation.min(bed);
		}
		if phi < rim_w {
			// Rim band: raise-only toward bank, full weight inside rim.
			let t = (1.0 - phi / rim_w).clamp(0.0, 1.0);
			let toward = elevation * (1.0 - t) + bank * t;
			return toward.max(elevation);
		}
		if phi < rim_w + apron_w {
			let u = ((phi - rim_w) / apron_w).clamp(0.0, 1.0);
			// Smoothstep out to identity.
			let fade = u * u * (3.0 - 2.0 * u);
			let toward = elevation * fade + bank * (1.0 - fade);
			return toward.max(elevation);
		}
		elevation
	}

	pub fn surface_at(&self, x: f32, z: f32) -> Option<f32> {
		self.fold_fields(Vec2::new(x, z), true).map(|(_, _, w)| w)
	}

	pub fn occupancy_at(&self, x: f32, z: f32) -> Option<f32> {
		self.fold_fields(Vec2::new(x, z), true).map(|(phi, _, _)| phi)
	}

	/// Softmask weight in `[0,1]` (`0` wet interior, `1` dry) from \(\phi_{\mathrm{union}}\).
	pub fn fill_softmask_at(&self, x: f32, z: f32) -> f32 {
		let fade = self.apron.shore_fade.max(0.25);
		match self.occupancy_at(x, z) {
			None => 1.0,
			Some(phi) if phi <= 0.0 => 0.0,
			Some(phi) if phi >= fade => 1.0,
			Some(phi) => {
				let t = (phi / fade).clamp(0.0, 1.0);
				t * t * (3.0 - 2.0 * t)
			}
		}
	}
}

/// Build a [`WaterFill`] that samples \(W\) / softmask from this prepared complex.
pub fn water_fill_from_prepared(prepared: PreparedHydroComplex) -> crate::fill::WaterFill {
	use crate::fill::{WaterFill, WaterSurface};
	use jersey_terrain_stamps::{CircleRegion, Region2D};
	let center = prepared.bounds.center();
	let radius = prepared.bounds.extent().max_element() * 0.75;
	WaterFill {
		// Softmask is driven by `WaterSurface::Hydro`; region is a conservative AABB proxy.
		region: Region2D::Circle(CircleRegion { center, radius }),
		inner_radius: 0.0,
		outer_radius: prepared.apron.shore_fade.max(0.25),
		noise: None,
		surface: WaterSurface::Hydro {
			prepared: prepared.clone(),
		},
		terrain_undercut: prepared.apron.fill_undercut.max(0.0),
	}
}

/// Decompose a graded corridor into per-segment reach primitives.
pub fn primitives_from_polyline(
	path: &[Vec2],
	levels: &[f32],
	half_width: f32,
	center_depth: f32,
	influence_pad: f32,
) -> Vec<HydroPrimitive> {
	let n = path.len().min(levels.len());
	if n < 2 {
		return Vec::new();
	}
	let hw = half_width.max(1e-3);
	let depth = center_depth.max(0.25);
	let mut out = Vec::with_capacity(n - 1);
	for i in 0..n - 1 {
		let a = path[i];
		let b = path[i + 1];
		if a.distance(b) <= 1e-4 {
			continue;
		}
		out.push(HydroPrimitive {
			footprint: HydroFootprint::ReachSegment {
				a,
				b,
				half_width: hw,
			},
			elevation: HydroElevation::ReachProfile {
				surface_a: levels[i],
				surface_b: levels[i + 1],
				center_depth: depth,
			},
			influence_pad: influence_pad.max(0.0),
		});
	}
	out
}

fn transverse_bowl(t: f32) -> f32 {
	// Cosine lobe: 1 at center, 0 at bank.
	let t = t.clamp(0.0, 1.0);
	0.5 * (1.0 + (std::f32::consts::PI * t).cos())
}

fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
	let ab = b - a;
	let len2 = ab.length_squared();
	if len2 <= 1e-12 {
		return p.distance(a);
	}
	let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
	(a + ab * t).distance(p)
}

fn reach_frame(p: Vec2, a: Vec2, b: Vec2) -> (f32, f32) {
	let ab = b - a;
	let len = ab.length();
	if len <= 1e-6 {
		return (0.0, p.distance(a));
	}
	let dir = ab / len;
	let rel = p - a;
	let z = (rel.dot(dir) / len).clamp(0.0, 1.0);
	let perp = Vec2::new(-dir.y, dir.x);
	let x = rel.dot(perp);
	(z, x)
}

fn ellipse_radial_norm(p: Vec2, center: Vec2, radii: Vec2, rotation: f32) -> f32 {
	let (s, c) = rotation.sin_cos();
	let d = p - center;
	let local = Vec2::new(c * d.x + s * d.y, -s * d.x + c * d.y);
	let rx = radii.x.max(1e-3);
	let rz = radii.y.max(1e-3);
	(local / Vec2::new(rx, rz)).length()
}

fn ellipse_sdf(p: Vec2, center: Vec2, radii: Vec2, rotation: f32) -> f32 {
	// Approximate analytic SDF (IQ style).
	let (s, c) = rotation.sin_cos();
	let d = p - center;
	let local = Vec2::new(c * d.x + s * d.y, -s * d.x + c * d.y);
	let ab = radii.max(Vec2::splat(1e-3));
	let k0 = (local / ab).length();
	if k0 < 1e-8 {
		return -ab.min_element();
	}
	let k1 = (local / (ab * ab)).length();
	k0 * (k0 - 1.0) / k1.max(1e-6)
}

/// Polynomial smooth minimum over a list (associative fold).
fn smoothmin_fold(values: &[f32], k: f32) -> f32 {
	if values.is_empty() {
		return 0.0;
	}
	let k = k.max(1e-3);
	let mut acc = values[0];
	for &v in &values[1..] {
		acc = smoothmin2(acc, v, k);
	}
	acc
}

fn smoothmin2(a: f32, b: f32, k: f32) -> f32 {
	// Exact ties must preserve the value (polynomial softmin otherwise dips by k/4).
	if (a - b).abs() <= 1e-5 {
		return a.min(b);
	}
	let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
	b.lerp(a, h) - k * h * (1.0 - h)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reach_profile_bowls_in_x_pitches_in_z() -> anyhow::Result<()> {
		let prim = HydroPrimitive {
			footprint: HydroFootprint::ReachSegment {
				a: Vec2::new(0.0, 0.0),
				b: Vec2::new(40.0, 0.0),
				half_width: 8.0,
			},
			elevation: HydroElevation::ReachProfile {
				surface_a: 50.0,
				surface_b: 40.0,
				center_depth: 4.0,
			},
			influence_pad: 2.0,
		};
		let (w_mid, bed_mid) = prim.surface_and_bed(Vec2::new(20.0, 0.0));
		assert!((w_mid - 45.0).abs() < 1e-3);
		assert!((bed_mid - (45.0 - 4.0)).abs() < 1e-3);
		let (w_bank, bed_bank) = prim.surface_and_bed(Vec2::new(20.0, 8.0));
		assert!((w_bank - 45.0).abs() < 1e-3, "W independent of X");
		assert!(
			bed_bank > bed_mid + 2.0,
			"bed rises toward bank: mid={bed_mid} bank={bed_bank}"
		);
		Ok(())
	}

	#[test]
	fn union_bed_takes_minimum() -> anyhow::Result<()> {
		let a = HydroPrimitive {
			footprint: HydroFootprint::ReachSegment {
				a: Vec2::new(0.0, 0.0),
				b: Vec2::new(40.0, 0.0),
				half_width: 6.0,
			},
			elevation: HydroElevation::ReachProfile {
				surface_a: 50.0,
				surface_b: 50.0,
				center_depth: 2.0,
			},
			influence_pad: 1.0,
		};
		let b = HydroPrimitive {
			footprint: HydroFootprint::ReachSegment {
				a: Vec2::new(0.0, 2.0),
				b: Vec2::new(40.0, 2.0),
				half_width: 6.0,
			},
			elevation: HydroElevation::ReachProfile {
				surface_a: 50.0,
				surface_b: 50.0,
				center_depth: 8.0,
			},
			influence_pad: 1.0,
		};
		let prep = PreparedHydroComplex::prepare(
			Bounds2::from_xz(-10.0, -20.0, 50.0, 30.0),
			1,
			vec![a, b],
			ComplexApronParams::default(),
		);
		let (_, bed, _) = prep
			.fold_fields(Vec2::new(20.0, 1.0), true)
			.expect("overlap");
		assert!(
			bed <= 50.0 - 7.0,
			"min bed should prefer deeper channel: {bed}"
		);
		Ok(())
	}

	#[test]
	fn index_matches_bruteforce_fold() -> anyhow::Result<()> {
		let mut prims = primitives_from_polyline(
			&[
				Vec2::new(0.0, 0.0),
				Vec2::new(30.0, 5.0),
				Vec2::new(60.0, 0.0),
				Vec2::new(60.0, 40.0),
			],
			&[40.0, 38.0, 36.0, 34.0],
			5.0,
			3.0,
			2.0,
		);
		prims.extend(primitives_from_polyline(
			&[Vec2::new(10.0, 40.0), Vec2::new(50.0, 20.0)],
			&[39.0, 35.0],
			5.0,
			3.0,
			2.0,
		));
		let prep = PreparedHydroComplex::prepare(
			Bounds2::from_xz(-20.0, -20.0, 80.0, 60.0),
			2,
			prims,
			ComplexApronParams::default(),
		);
		for i in 0..16 {
			for j in 0..16 {
				let p = Vec2::new(i as f32 * 5.0, j as f32 * 4.0);
				let indexed = prep.fold_fields(p, true);
				let brute = prep.fold_fields(p, false);
				match (indexed, brute) {
					(None, None) => {}
					(Some((p0, b0, w0)), Some((p1, b1, w1))) => {
						assert!((p0 - p1).abs() < 1e-3, "phi {p0} vs {p1} at {p:?}");
						if b0.is_finite() || b1.is_finite() {
							assert!((b0 - b1).abs() < 1e-3, "bed {b0} vs {b1} at {p:?}");
						}
						if w0.is_finite() || w1.is_finite() {
							assert!((w0 - w1).abs() < 1e-2, "W {w0} vs {w1} at {p:?}");
						}
					}
					(a, b) => panic!("mismatch presence at {p:?}: {a:?} vs {b:?}"),
				}
			}
		}
		Ok(())
	}

	#[test]
	fn no_internal_rim_in_confluence_interior() -> anyhow::Result<()> {
		let a = HydroPrimitive {
			footprint: HydroFootprint::ReachSegment {
				a: Vec2::new(0.0, 0.0),
				b: Vec2::new(40.0, 0.0),
				half_width: 8.0,
			},
			elevation: HydroElevation::ReachProfile {
				surface_a: 30.0,
				surface_b: 30.0,
				center_depth: 3.0,
			},
			influence_pad: 1.0,
		};
		let b = HydroPrimitive {
			footprint: HydroFootprint::ReachSegment {
				a: Vec2::new(20.0, -20.0),
				b: Vec2::new(20.0, 20.0),
				half_width: 8.0,
			},
			elevation: HydroElevation::ReachProfile {
				surface_a: 30.0,
				surface_b: 30.0,
				center_depth: 3.0,
			},
			influence_pad: 1.0,
		};
		let mut apron = ComplexApronParams::default();
		apron.rim_lift = 2.0;
		apron.rim_width = 3.0;
		apron.apron_width = 6.0;
		apron.rim_height = RegionNoise::from_seed(1, 0.05, 0.0);
		let prep = PreparedHydroComplex::prepare(
			Bounds2::from_xz(-30.0, -40.0, 60.0, 40.0),
			3,
			vec![a, b],
			apron,
		);
		// Junction interior should be below surface (carved), not raised.
		let h0 = 28.0;
		let h1 = prep.modify_elevation(h0, 20.0, 0.0);
		assert!(
			h1 <= h0 + 0.05,
			"confluence interior must not raise: {h0} -> {h1}"
		);
		assert!(h1 < 30.0 - 1.0, "should sit in the carved bowl: {h1}");
		Ok(())
	}
}
