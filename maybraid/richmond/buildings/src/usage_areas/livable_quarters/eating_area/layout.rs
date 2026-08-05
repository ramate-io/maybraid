//! Pack kitchen + dining side-by-side, or kitchen alone.

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
use bevy_math::{Vec2, Vec3};
use procedural_common::{aabb2_area, Aabb2dPack, NoiseParams};

use crate::fit::{Confines, FitError};
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::usage_areas::livable_quarters::dining_room::{
	DiningRoomParameterized, DiningRoomPlan,
};
use crate::usage_areas::livable_quarters::kitchen::{KitchenParameterized, KitchenPlan};

use super::parameterized::{
	EatingAreaPacked, EatingAreaParameterized, EatingAreaPlan, SCOPE,
};

/// Minimum combined footprint for a kitchen+dining split (m²).
pub const MIN_PAIR_AREA: f32 = 12.0;
const EPS: f32 = 1e-3;
const DOOR_WIDTH: f32 = 1.0;
const MIN_HALF_DIM: f32 = 2.0;

impl EatingAreaPlan {
	pub fn from_parameterized(
		params: EatingAreaParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<Self, FitError> {
		let packed = pack_eating_area(&params, confines, noise)?;
		Ok(Self {
			parameterized: params,
			packed,
		})
	}
}

fn pack_eating_area(
	params: &EatingAreaParameterized,
	confines: &Confines,
	noise: NoiseParams,
) -> Result<EatingAreaPacked, FitError> {
	let host = host_xz(&confines.bounds);
	let host_a = aabb2_area(host);
	if host_a + EPS >= MIN_PAIR_AREA {
		if let Some(pair) = try_kitchen_dining(params, confines, host, noise) {
			return Ok(pair);
		}
	}
	// Fallback: kitchen claims the whole host.
	kitchen_only(params, confines, noise)
}

fn try_kitchen_dining(
	params: &EatingAreaParameterized,
	confines: &Confines,
	host: Aabb2d,
	noise: NoiseParams,
) -> Option<EatingAreaPacked> {
	let y0 = Vec3::from(confines.bounds.min).y;
	let y1 = Vec3::from(confines.bounds.max).y;
	let roll = confines.roll;
	let frac = params.kitchen_frac.clamp(0.3, 0.7);
	let candidates = [
		host.bipartition_by_area(true, true, frac),
		host.bipartition_by_area(false, true, frac),
		host.bipartition_by_area(true, false, frac),
		host.bipartition_by_area(false, false, frac),
	];
	for (a, b) in candidates {
		if !half_usable(a) || !half_usable(b) {
			continue;
		}
		// Prefer kitchen on the larger half when areas differ a lot.
		let orders = if aabb2_area(a) >= aabb2_area(b) {
			[(a, b), (b, a)]
		} else {
			[(b, a), (a, b)]
		};
		for (k_xz, d_xz) in orders {
			let k_open = child_openings(confines, k_xz, d_xz, y0, y1, true);
			let d_open = child_openings(confines, d_xz, k_xz, y0, y1, false);
			let k_conf = confines_from_xz(k_xz, y0, y1, roll, &k_open);
			let d_conf = confines_from_xz(d_xz, y0, y1, roll, &d_open);
			let k_params = KitchenParameterized {
				style: params.style,
				spaciousness: params.spaciousness,
				occupancy: params.occupancy,
				layout: params.kitchen_layout,
			};
			let d_params = DiningRoomParameterized::with_fill(params.spaciousness, params.occupancy);
			let Ok(kitchen) = KitchenPlan::from_parameterized(k_params, &k_conf, noise) else {
				continue;
			};
			let Ok(dining) = DiningRoomPlan::from_parameterized(d_params, &d_conf, noise) else {
				continue;
			};
			return Some(EatingAreaPacked::KitchenDining {
				kitchen,
				dining,
				kitchen_xz: k_xz,
				dining_xz: d_xz,
			});
		}
	}
	None
}

fn kitchen_only(
	params: &EatingAreaParameterized,
	confines: &Confines,
	noise: NoiseParams,
) -> Result<EatingAreaPacked, FitError> {
	let k_params = KitchenParameterized {
		style: params.style,
		spaciousness: params.spaciousness,
		occupancy: params.occupancy,
		layout: params.kitchen_layout,
	};
	let kitchen = KitchenPlan::from_parameterized(k_params, confines, noise)?;
	Ok(EatingAreaPacked::KitchenOnly { kitchen })
}

fn half_usable(r: Aabb2d) -> bool {
	let s = r.max - r.min;
	s.x + EPS >= MIN_HALF_DIM && s.y + EPS >= MIN_HALF_DIM && aabb2_area(r) >= 4.5
}

fn child_openings(
	host: &Confines,
	self_xz: Aabb2d,
	neighbor_xz: Aabb2d,
	y0: f32,
	y1: f32,
	kitchen: bool,
) -> Openings {
	let mut openings = Openings::new();
	// Inherit host passages that sit nearer this half.
	for (id, o) in host.openings.iter() {
		if !matches!(o.label, OpeningLabel::Passage) {
			continue;
		}
		let dmin = Vec3::from(o.bounds.min);
		let dmax = Vec3::from(o.bounds.max);
		let c = Vec2::new(0.5 * (dmin.x + dmax.x), 0.5 * (dmin.z + dmax.z));
		let d_self = (c - self_xz.center()).length_squared();
		let d_nb = (c - neighbor_xz.center()).length_squared();
		if d_self <= d_nb {
			openings.insert(id.clone(), o.clone());
		}
	}
	// Shared-edge door so both halves clear placer passage keep-outs.
	if let Some((along_x, lo, hi, mid)) = shared_edge(self_xz, neighbor_xz) {
		if let Some((oid, opening)) =
			connecting_passage(along_x, lo, hi, mid, y0, y1, if kitchen { 0 } else { 1 })
		{
			openings.insert(oid, opening);
		}
	}
	if openings
		.iter()
		.any(|(_, o)| matches!(o.label, OpeningLabel::Passage))
	{
		return openings;
	}
	// Last resort: edge door on the half itself.
	edge_passage(self_xz, y0, y1, if kitchen { 0 } else { 1 }, &mut openings);
	openings
}

fn shared_edge(a: Aabb2d, b: Aabb2d) -> Option<(bool, f32, f32, f32)> {
	let touch_x = (a.max.x - b.min.x).abs() <= EPS || (b.max.x - a.min.x).abs() <= EPS;
	if touch_x {
		let mid = if (a.max.x - b.min.x).abs() <= EPS {
			a.max.x
		} else {
			b.max.x
		};
		let lo = a.min.y.max(b.min.y);
		let hi = a.max.y.min(b.max.y);
		if hi - lo > EPS {
			return Some((false, lo, hi, mid));
		}
	}
	let touch_y = (a.max.y - b.min.y).abs() <= EPS || (b.max.y - a.min.y).abs() <= EPS;
	if touch_y {
		let mid = if (a.max.y - b.min.y).abs() <= EPS {
			a.max.y
		} else {
			b.max.y
		};
		let lo = a.min.x.max(b.min.x);
		let hi = a.max.x.min(b.max.x);
		if hi - lo > EPS {
			return Some((true, lo, hi, mid));
		}
	}
	None
}

fn connecting_passage(
	along_x: bool,
	lo: f32,
	hi: f32,
	mid: f32,
	y0: f32,
	y1: f32,
	tag: u32,
) -> Option<(OpeningId, Opening)> {
	let shared = hi - lo;
	if shared < DOOR_WIDTH * 0.7 + EPS {
		return None;
	}
	let clear = DOOR_WIDTH.min(shared - 0.1).max(0.7);
	let center = 0.5 * (lo + hi);
	let half = clear * 0.5;
	let door_lo = (center - half).max(lo);
	let door_hi = (center + half).min(hi);
	let half_d = 0.12_f32;
	let door_h = (y1 - y0).min(2.2);
	let bounds = if along_x {
		Aabb3d::from_min_max(
			Vec3::new(door_lo, y0, mid - half_d),
			Vec3::new(door_hi, y0 + door_h, mid + half_d),
		)
	} else {
		Aabb3d::from_min_max(
			Vec3::new(mid - half_d, y0, door_lo),
			Vec3::new(mid + half_d, y0 + door_h, door_hi),
		)
	};
	Some((
		OpeningId::scoped(SCOPE, "pair", format!("{tag}")),
		Opening::new(bounds, OpeningLabel::Passage),
	))
}

fn edge_passage(xz: Aabb2d, y0: f32, y1: f32, tag: u32, openings: &mut Openings) {
	let sx = xz.max.x - xz.min.x;
	let sz = xz.max.y - xz.min.y;
	let door_w = DOOR_WIDTH.min(sx.max(sz) - 0.25).clamp(0.7, 1.15);
	let half = door_w * 0.5;
	let door_h = (y1 - y0).min(2.15).max(1.9);
	let half_d = 0.12_f32;
	let bounds = if sx >= sz {
		let cx = 0.5 * (xz.min.x + xz.max.x);
		let z = xz.min.y;
		Aabb3d::from_min_max(
			Vec3::new(cx - half, y0, z - half_d),
			Vec3::new(cx + half, y0 + door_h, z + half_d),
		)
	} else {
		let cz = 0.5 * (xz.min.y + xz.max.y);
		let x = xz.min.x;
		Aabb3d::from_min_max(
			Vec3::new(x - half_d, y0, cz - half),
			Vec3::new(x + half_d, y0 + door_h, cz + half),
		)
	};
	openings.insert(
		OpeningId::scoped(SCOPE, "edge", format!("{tag}")),
		Opening::new(bounds, OpeningLabel::Passage),
	);
}

fn host_xz(bounds: &Aabb3d) -> Aabb2d {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	Aabb2d {
		min: Vec2::new(min.x, min.z),
		max: Vec2::new(max.x, max.z),
	}
}

fn confines_from_xz(xz: Aabb2d, y0: f32, y1: f32, roll: f32, openings: &Openings) -> Confines {
	Confines::new(
		Aabb3d::from_min_max(
			Vec3::new(xz.min.x, y0, xz.min.y),
			Vec3::new(xz.max.x, y1, xz.max.y),
		),
		roll,
		openings.clone(),
	)
}
