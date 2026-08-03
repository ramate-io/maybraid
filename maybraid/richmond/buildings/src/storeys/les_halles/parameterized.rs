//! Noise-sampled knobs for a Les Halles floor plan.

use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FitError};

/// Where vertical shafts are allocated in the gallery ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LesHallesShaftPlacement {
	/// One shaft in each of the four corners of the gallery ring.
	Corners,
	/// One shaft in the middle of each of the four sides.
	MidSides,
}

/// One bay size to try when packing along a wall run (stall doors or windows).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LesHallesStallDoor {
	/// Preferred clear leaf / aperture width (meters).
	pub door_width: f32,
	/// Minimum jamb / reveal on each side of the leaf.
	pub jamb_min: f32,
	/// Allowed over/undershoot on the packed span (and leaf) in meters.
	pub allowed_error: f32,
}

/// A bay placed on a straight wall run (coordinates along the run).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LesHallesPlacedDoor {
	/// Distance from the run start to the leaf’s start edge.
	pub along: f32,
	/// Leaf width used.
	pub width: f32,
}

/// Resolved Les Halles plan knobs.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesParameterized {
	/// Outer commercial gallery / stall band depth (meters).
	pub gallery_width: f32,
	/// Inner balcony / walking band depth (meters).
	pub balcony_width: f32,
	/// Target courtyard share of the short footprint axis (`~0.5` → ¼ rim / ½ gap / ¼ rim).
	pub courtyard_fraction: f32,
	pub shaft_placement: LesHallesShaftPlacement,
	/// How densely to pack outer-facade apertures (`0…1`).
	pub opening_density: f32,
	/// Stall door sizes to pack along each inner-wall straight section (catalog order).
	pub doors: Vec<LesHallesStallDoor>,
	/// Exterior aperture sizes to pack along outer free runs (catalog order).
	pub windows: Vec<LesHallesStallDoor>,
}

/// Minimum gallery / stall band depth.
pub const MIN_GALLERY_WIDTH: f32 = 5.0;
/// Maximum gallery / stall band depth sampled from noise.
pub const MAX_GALLERY_WIDTH: f32 = 25.0;
/// Minimum balcony band depth.
pub const MIN_BALCONY_WIDTH: f32 = 3.0;
/// Maximum balcony band depth sampled from noise.
pub const MAX_BALCONY_WIDTH: f32 = 8.0;
/// Minimum clear courtyard on each plan axis (hard floor).
pub const MIN_COURTYARD: f32 = 2.0;
/// Minimum storey height.
pub const MIN_STOREY_HEIGHT: f32 = 2.5;
/// Nominal shaft side length (XZ) for mid-side shafts.
pub const SHAFT_SIDE: f32 = 1.8;

/// Default plan split targets ~½ courtyard (¼ rim + ½ gap + ¼ rim).
pub const MIN_COURTYARD_FRACTION: f32 = 0.40;
pub const MAX_COURTYARD_FRACTION: f32 = 0.60;
/// Stall share of the rim budget (gallery / (gallery + balcony)).
pub const MIN_STALL_RING_SHARE: f32 = 0.55;
pub const MAX_STALL_RING_SHARE: f32 = 0.75;

/// Salt lanes for spatial sampling at the confines center.
const SALT_COURTYARD: f32 = 1.0;
const SALT_STALL_SHARE: f32 = 2.0;
const SALT_SHAFT: f32 = 3.0;
const SALT_OPENINGS: f32 = 4.0;

impl LesHallesParameterized {
	/// Sample knobs at the confines center. Rejects footprints that cannot host
	/// minimum gallery + balcony + courtyard on either axis.
	///
	/// Depths are driven by a courtyard-fraction target (default ~½ the short
	/// axis), then the remaining rim is split into stall vs balcony within
	/// absolute min/max clamps. Mins win over the ratio when the footprint is tight.
	///
	/// Stall-door / window catalogs are produced by
	/// [`crate::storeys::les_halles::LesHallesFloorPlan`].
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let (extent_x, extent_z, height) = footprint_extents(confines)?;
		let min_ring = MIN_GALLERY_WIDTH + MIN_BALCONY_WIDTH;
		let min_outer = 2.0 * min_ring + MIN_COURTYARD;
		if extent_x < min_outer || extent_z < min_outer {
			return Err(FitError::TooSmall { reason: "footprint" });
		}
		if height < MIN_STOREY_HEIGHT {
			return Err(FitError::TooSmall { reason: "height" });
		}

		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let extent_min = extent_x.min(extent_z);

		let courtyard_fraction = cfg.sample_range_f32_4d(
			MIN_COURTYARD_FRACTION,
			MAX_COURTYARD_FRACTION,
			c.x,
			c.y,
			c.z,
			SALT_COURTYARD,
		);

		// Rim budget from the ratio, then clamp so mins/maxes and MIN_COURTYARD hold.
		let ideal_ring = extent_min * (1.0 - courtyard_fraction) * 0.5;
		let hard_max_ring = ((extent_min - MIN_COURTYARD) * 0.5).max(min_ring);
		let abs_max_ring = MAX_GALLERY_WIDTH + MAX_BALCONY_WIDTH;
		let ring_budget = ideal_ring.clamp(min_ring, hard_max_ring.min(abs_max_ring));

		let stall_share = cfg.sample_range_f32_4d(
			MIN_STALL_RING_SHARE,
			MAX_STALL_RING_SHARE,
			c.x,
			c.y,
			c.z,
			SALT_STALL_SHARE,
		);
		let (gallery_width, balcony_width) = split_ring_budget(ring_budget, stall_share)?;

		let ring = gallery_width + balcony_width;
		if extent_x < 2.0 * ring + MIN_COURTYARD || extent_z < 2.0 * ring + MIN_COURTYARD {
			return Err(FitError::TooSmall { reason: "footprint" });
		}

		let shaft_i = cfg.sample_range_usize_4d(0, 2, c.x, c.y, c.z, SALT_SHAFT);
		let shaft_placement = if shaft_i == 0 {
			LesHallesShaftPlacement::Corners
		} else {
			LesHallesShaftPlacement::MidSides
		};

		let opening_density = cfg.sample_unit_4d(c.x, c.y, c.z, SALT_OPENINGS);
		let doors = crate::storeys::les_halles::LesHallesFloorPlan::generate_stall_doors(&cfg, c);
		let windows = crate::storeys::les_halles::LesHallesFloorPlan::generate_windows(&cfg, c);

		Ok(Self {
			gallery_width,
			balcony_width,
			courtyard_fraction,
			shaft_placement,
			opening_density,
			doors,
			windows,
		})
	}

	pub fn ring_width(&self) -> f32 {
		self.gallery_width + self.balcony_width
	}

	/// Expected count of inner-wall straight sections for this shaft layout.
	pub fn expected_inner_section_count(&self) -> usize {
		match self.shaft_placement {
			LesHallesShaftPlacement::Corners => 4,
			LesHallesShaftPlacement::MidSides => 8,
		}
	}

	/// Corner stall-strip / shaft-clear length along each abutting inner wall.
	///
	/// Half the corner gallery square (which itself is `gallery_width` on a side).
	pub fn corner_clear_len(&self) -> f32 {
		(self.gallery_width * 0.5).max(2.0)
	}

	/// Pack [`Self::doors`] along a run; always tries to place at least one.
	pub fn fit_doors_on_run(&self, run_length: f32) -> Vec<LesHallesPlacedDoor> {
		Self::fit_bays_on_run(&self.doors, run_length, true)
	}

	/// Pack exterior windows along a run.
	///
	/// Uses a density-scaled prefix of [`Self::windows`]; does not force a window
	/// when nothing fits (sparse facades are allowed).
	pub fn fit_windows_on_run(&self, run_length: f32) -> Vec<LesHallesPlacedDoor> {
		if self.windows.is_empty() || self.opening_density < 0.08 {
			return Vec::new();
		}
		let n = self.windows.len();
		let take = ((n as f32) * self.opening_density.clamp(0.15, 1.0))
			.ceil()
			.max(1.0) as usize;
		let take = take.min(n);
		Self::fit_bays_on_run(&self.windows[..take], run_length, false)
	}

	/// Pack catalog bays along a run of `run_length` meters.
	///
	/// Walks `bays` in order; each size is placed if the remaining run can host
	/// `door_width + 2·jamb_min` within `allowed_error`, otherwise that size is
	/// skipped. When `force_one`, retries with the smallest feasible size.
	pub fn fit_bays_on_run(
		bays: &[LesHallesStallDoor],
		run_length: f32,
		force_one: bool,
	) -> Vec<LesHallesPlacedDoor> {
		let run_length = run_length.max(0.0);
		let mut placed = Self::pack_bays(bays, run_length);
		if placed.is_empty() && force_one {
			placed = Self::force_one_bay(bays, run_length);
		}
		placed
	}

	fn pack_bays(bays: &[LesHallesStallDoor], run_length: f32) -> Vec<LesHallesPlacedDoor> {
		let mut cursor = 0.0_f32;
		let mut placed = Vec::new();
		for spec in bays {
			let rem = run_length - cursor;
			let Some((pack, door_w, jamb)) = pack_span(*spec, rem) else {
				continue;
			};
			placed.push(LesHallesPlacedDoor {
				along: cursor + jamb,
				width: door_w,
			});
			cursor += pack;
		}
		placed
	}

	fn force_one_bay(bays: &[LesHallesStallDoor], run_length: f32) -> Vec<LesHallesPlacedDoor> {
		let mut best: Option<LesHallesStallDoor> = None;
		for spec in bays {
			let min_pack =
				(spec.door_width - spec.allowed_error).max(0.4) + 2.0 * spec.jamb_min.min(0.05);
			if run_length + 1e-4 < min_pack {
				continue;
			}
			best = Some(match best {
				None => *spec,
				Some(prev) => {
					if spec.door_width < prev.door_width {
						*spec
					} else {
						prev
					}
				}
			});
		}
		let Some(spec) = best else {
			let w = (run_length * 0.5).clamp(0.8, 2.0).min(run_length.max(0.8));
			if run_length < 0.8 {
				return Vec::new();
			}
			let jamb = ((run_length - w) * 0.5).max(0.05);
			return vec![LesHallesPlacedDoor { along: jamb, width: w }];
		};
		if let Some((_pack, door_w, jamb)) = pack_span(spec, run_length) {
			vec![LesHallesPlacedDoor {
				along: jamb,
				width: door_w,
			}]
		} else {
			Vec::new()
		}
	}
}

/// Split a rim budget into gallery + balcony using `stall_share`, then clamp to
/// absolute min/max depths while staying inside `ring_budget`.
fn split_ring_budget(ring_budget: f32, stall_share: f32) -> Result<(f32, f32), FitError> {
	if ring_budget + 1e-4 < MIN_GALLERY_WIDTH + MIN_BALCONY_WIDTH {
		return Err(FitError::TooSmall { reason: "footprint" });
	}
	let max_gallery = MAX_GALLERY_WIDTH
		.min(ring_budget - MIN_BALCONY_WIDTH)
		.max(MIN_GALLERY_WIDTH);
	let target_gallery = (ring_budget * stall_share.clamp(0.0, 1.0)).clamp(MIN_GALLERY_WIDTH, max_gallery);
	let rem = (ring_budget - target_gallery).max(0.0);
	let max_balcony = MAX_BALCONY_WIDTH.min(rem).max(MIN_BALCONY_WIDTH);
	let balcony_width = rem.clamp(MIN_BALCONY_WIDTH, max_balcony);
	let gallery_width = (ring_budget - balcony_width)
		.clamp(MIN_GALLERY_WIDTH, max_gallery);
	// Re-fit balcony after gallery clamp so the pair still sums to the budget when possible.
	let balcony_width = (ring_budget - gallery_width)
		.clamp(MIN_BALCONY_WIDTH, MAX_BALCONY_WIDTH.min(ring_budget - MIN_GALLERY_WIDTH));
	let gallery_width = (ring_budget - balcony_width)
		.clamp(MIN_GALLERY_WIDTH, MAX_GALLERY_WIDTH.min(ring_budget - MIN_BALCONY_WIDTH));
	if gallery_width + balcony_width > ring_budget + 1e-3 {
		return Err(FitError::TooSmall { reason: "footprint" });
	}
	Ok((gallery_width, balcony_width))
}

fn pack_span(spec: LesHallesStallDoor, remaining: f32) -> Option<(f32, f32, f32)> {
	let door_lo = (spec.door_width - spec.allowed_error).max(0.4);
	let door_hi = spec.door_width + spec.allowed_error;
	let jamb = spec.jamb_min.max(0.0);
	let min_pack = door_lo + 2.0 * jamb - spec.allowed_error.max(0.0);
	let min_pack = min_pack.max(door_lo + 0.05);
	if remaining + 1e-4 < min_pack {
		return None;
	}
	let nominal = spec.door_width + 2.0 * jamb;
	let pack = if remaining >= nominal {
		nominal
	} else {
		remaining
	};
	let door_w = (pack - 2.0 * jamb)
		.clamp(door_lo, door_hi)
		.min(pack - 0.05)
		.max(door_lo.min(pack * 0.8));
	let jamb_each = ((pack - door_w) * 0.5).max(0.0);
	Some((pack, door_w, jamb_each))
}

pub(crate) fn footprint_extents(confines: &Confines) -> Result<(f32, f32, f32), FitError> {
	let min = confines.bounds.min;
	let max = confines.bounds.max;
	let extent_x = max.x - min.x;
	let extent_z = max.z - min.z;
	let height = max.y - min.y;
	if !extent_x.is_finite() || !extent_z.is_finite() || !height.is_finite() {
		return Err(FitError::InvalidConfines {
			reason: "non_finite_bounds",
		});
	}
	if extent_x <= 0.0 || extent_z <= 0.0 || height <= 0.0 {
		return Err(FitError::InvalidConfines {
			reason: "empty_bounds",
		});
	}
	Ok((extent_x, extent_z, height))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn doors_catalog() -> Vec<LesHallesStallDoor> {
		vec![
			LesHallesStallDoor {
				door_width: 3.5,
				jamb_min: 0.25,
				allowed_error: 0.35,
			},
			LesHallesStallDoor {
				door_width: 3.0,
				jamb_min: 0.25,
				allowed_error: 0.3,
			},
			LesHallesStallDoor {
				door_width: 2.4,
				jamb_min: 0.2,
				allowed_error: 0.25,
			},
			LesHallesStallDoor {
				door_width: 1.8,
				jamb_min: 0.2,
				allowed_error: 0.2,
			},
		]
	}

	#[test]
	fn large_footprint_keeps_courtyard_near_half() {
		let confines = Confines::from_bounds(bevy_math::bounding::Aabb3d::from_min_max(
			bevy_math::Vec3::new(-24.0, 0.0, -18.0),
			bevy_math::Vec3::new(24.0, 4.0, 18.0),
		));
		let params = LesHallesParameterized::sample(&confines, NoiseParams { seed: 42, ..NoiseParams::default() })
			.unwrap();
		let extent_min = 36.0_f32;
		let ring = params.ring_width();
		let courtyard = extent_min - 2.0 * ring;
		let frac = courtyard / extent_min;
		assert!(
			frac > 0.35 && frac < 0.65,
			"courtyard fraction {frac:.2} (courtyard={courtyard:.1}, ring={ring:.1})"
		);
		assert!(params.gallery_width >= MIN_GALLERY_WIDTH - 1e-3);
		assert!(params.gallery_width <= MAX_GALLERY_WIDTH + 1e-3);
		assert!(params.balcony_width >= MIN_BALCONY_WIDTH - 1e-3);
		assert!(params.balcony_width <= MAX_BALCONY_WIDTH + 1e-3);
		// Stall depth should not devour the short axis.
		assert!(params.gallery_width < extent_min * 0.35);
	}

	#[test]
	fn fit_doors_places_multiple_on_long_run() {
		let params = LesHallesParameterized {
			gallery_width: 6.0,
			balcony_width: 4.0,
			courtyard_fraction: 0.5,
			shaft_placement: LesHallesShaftPlacement::Corners,
			opening_density: 0.5,
			doors: doors_catalog(),
			windows: Vec::new(),
		};
		let placed = params.fit_doors_on_run(14.0);
		assert!(placed.len() >= 2, "expected multiple doors, got {}", placed.len());
		assert!(placed.iter().all(|d| d.width >= 1.5));
	}

	#[test]
	fn fit_doors_skips_too_large_then_places_smaller() {
		let params = LesHallesParameterized {
			gallery_width: 6.0,
			balcony_width: 4.0,
			courtyard_fraction: 0.5,
			shaft_placement: LesHallesShaftPlacement::Corners,
			opening_density: 0.5,
			doors: vec![
				LesHallesStallDoor {
					door_width: 10.0,
					jamb_min: 0.5,
					allowed_error: 0.1,
				},
				LesHallesStallDoor {
					door_width: 2.0,
					jamb_min: 0.2,
					allowed_error: 0.2,
				},
			],
			windows: Vec::new(),
		};
		let placed = params.fit_doors_on_run(4.0);
		assert_eq!(placed.len(), 1);
		assert!((placed[0].width - 2.0).abs() < 0.25);
	}

	#[test]
	fn fit_windows_respects_low_density() {
		let params = LesHallesParameterized {
			gallery_width: 6.0,
			balcony_width: 4.0,
			courtyard_fraction: 0.5,
			shaft_placement: LesHallesShaftPlacement::Corners,
			opening_density: 0.05,
			doors: Vec::new(),
			windows: doors_catalog(),
		};
		assert!(params.fit_windows_on_run(20.0).is_empty());
	}
}
