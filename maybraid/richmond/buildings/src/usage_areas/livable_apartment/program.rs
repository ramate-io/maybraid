//! Residential room-count program and per-rect kind distribution.

use bevy_math::bounding::Aabb2d;
use bevy_math::Vec3;
use procedural_common::{aabb2_area, NoiseConfig, NoiseParams};

use crate::usage_areas::rectangular_livable_area::RectQuarterKind;

const EPS: f32 = 1e-3;

/// Counts of program rooms for an apartment footprint.
#[derive(Debug, Clone, Copy)]
pub struct ProgramCounts {
	pub bedrooms: u8,
	pub bathrooms: u8,
	pub half_baths: u8,
	pub kitchens: u8,
	pub dining: u8,
	pub living: u8,
	pub sitting: u8,
	pub studies: u8,
}

/// Area-tier program: kitchen almost always; living always; dining from mid size.
pub fn program_from_area(area: f32, noise: NoiseParams, center: Vec3) -> ProgramCounts {
	let cfg = NoiseConfig::new(noise);
	let jitter = cfg.sample_range_f32_4d(0.0, 1.0, center.x, center.y, center.z, 44.0);
	let want_kitchen = area >= 22.0 || jitter > 0.2;
	if area < 36.0 {
		ProgramCounts {
			bedrooms: 0,
			bathrooms: 1,
			half_baths: 0,
			kitchens: if want_kitchen { 1 } else { 0 },
			dining: 0,
			living: 1,
			sitting: 0,
			studies: 0,
		}
	} else if area < 58.0 {
		ProgramCounts {
			bedrooms: 1,
			bathrooms: 1,
			half_baths: 0,
			kitchens: 1,
			dining: if jitter > 0.35 { 1 } else { 0 },
			living: 1,
			sitting: 0,
			studies: 0,
		}
	} else if area < 95.0 {
		ProgramCounts {
			bedrooms: 2,
			bathrooms: 1,
			half_baths: if jitter > 0.55 { 1 } else { 0 },
			kitchens: 1,
			dining: 1,
			living: 1,
			sitting: if jitter > 0.7 { 1 } else { 0 },
			studies: 0,
		}
	} else {
		ProgramCounts {
			bedrooms: if area > 120.0 { 3 } else { 2 },
			bathrooms: if area > 110.0 { 2 } else { 1 },
			half_baths: if jitter > 0.45 { 1 } else { 0 },
			kitchens: 1,
			dining: 1,
			living: 1,
			sitting: if jitter > 0.5 { 1 } else { 0 },
			studies: if jitter > 0.55 { 1 } else { 0 },
		}
	}
}

/// Flatten counts to a kind list (Eating before Living).
pub fn full_kind_list(p: ProgramCounts) -> Vec<RectQuarterKind> {
	let mut out = Vec::new();
	if p.kitchens > 0 || p.dining > 0 {
		out.push(RectQuarterKind::Eating);
	}
	for _ in 0..p.living {
		out.push(RectQuarterKind::Living);
	}
	for _ in 0..p.sitting {
		out.push(RectQuarterKind::Sitting);
	}
	for _ in 0..p.bedrooms {
		out.push(RectQuarterKind::Bedroom);
	}
	for _ in 0..p.bathrooms {
		out.push(RectQuarterKind::Bathroom);
	}
	for _ in 0..p.half_baths {
		out.push(RectQuarterKind::HalfBath);
	}
	for _ in 0..p.studies {
		out.push(RectQuarterKind::Study);
	}
	if out.is_empty() {
		out.push(RectQuarterKind::Living);
	}
	out
}

/// Assign kinds across max-rects. Living prefers large rects; within a rect,
/// Eating packs before Living so the kitchen carves compactly.
pub fn distribute_program(kinds: &[RectQuarterKind], rects: &[Aabb2d]) -> Vec<Vec<RectQuarterKind>> {
	let n = rects.len();
	let mut slices = vec![Vec::new(); n];
	if n == 0 {
		return slices;
	}
	let areas: Vec<f32> = rects.iter().map(|r| aabb2_area(*r)).collect();
	let total: f32 = areas.iter().sum::<f32>().max(EPS);
	let mut ordered: Vec<RectQuarterKind> = kinds.to_vec();
	ordered.sort_by_key(|k| match k {
		k if k.is_closed() => 0u8,
		RectQuarterKind::Living | RectQuarterKind::Sitting => 1u8,
		RectQuarterKind::Eating => 2u8,
		_ => 3u8,
	});
	let mut load = vec![0.0_f32; n];
	let targets: Vec<f32> = areas.iter().map(|a| a / total).collect();
	for kind in ordered {
		let mut best = 0usize;
		let mut best_score = f32::NEG_INFINITY;
		for i in 0..n {
			let closed_bonus = if kind.is_closed() { areas[i] * 0.02 } else { 0.0 };
			let living_bonus =
				if matches!(kind, RectQuarterKind::Living | RectQuarterKind::Sitting) {
					areas[i] * 0.04
				} else {
					0.0
				};
			let eating_penalty = if matches!(kind, RectQuarterKind::Eating) {
				areas[i] * 0.015
			} else {
				0.0
			};
			let score = targets[i] - load[i] / total.max(1.0) + areas[i] * 1e-4 + closed_bonus
				+ living_bonus
				- eating_penalty;
			if score > best_score {
				best_score = score;
				best = i;
			}
		}
		slices[best].push(kind);
		load[best] += if kind.is_closed() {
			1.5
		} else if matches!(kind, RectQuarterKind::Living | RectQuarterKind::Sitting) {
			1.35
		} else {
			1.0
		};
	}
	for s in &mut slices {
		if s.is_empty() {
			s.push(RectQuarterKind::Living);
		}
		s.sort_by_key(|k| match k {
			k if k.is_closed() => 0u8,
			RectQuarterKind::Eating | RectQuarterKind::Kitchen => 1u8,
			RectQuarterKind::Living | RectQuarterKind::Sitting => 2u8,
			_ => 3u8,
		});
	}
	slices
}
