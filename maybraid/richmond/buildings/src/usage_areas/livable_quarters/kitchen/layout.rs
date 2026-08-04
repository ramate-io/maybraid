//! Kitchen plan packer: counter layouts (galley / L / peninsula) + optional island.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec3;
use procedural_common::{aabb3_to_plan, NoiseConfig, NoiseParams, PlanAxes};

use crate::fit::{Confines, FitError};
use crate::placer::predicates::{clear_of_keep_outs, in_host, long_face_on_wall};
use crate::placer::{try_free_extent, try_wall_long, FreeExtentKnobs, WallLongKnobs};

use crate::usage_areas::livable_quarters::pack::{init_host, xz_area, PackHost};

pub const MIN_AREA: f32 = 2.4 * 2.0;

const WALL_EPS: f32 = 0.08;
const COUNTER_DEPTH: f32 = 0.6;
const COUNTER_HEIGHT: f32 = 0.9;

/// Counter program subtype — drives which solid runs are authored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KitchenCounterLayout {
	/// Single wall run.
	Galley,
	/// Two wall runs that meet at a host corner.
	LShape,
	/// Wall run + peninsula stub into the room from one end.
	Peninsula,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct KitchenPacked {
	pub counter_runs: Vec<Aabb3d>,
	pub peninsulas: Vec<Aabb3d>,
	pub islands: Vec<Aabb3d>,
	pub fillers: Vec<Aabb3d>,
	pub layout: Option<KitchenCounterLayout>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KitchenRegions {
	pub spaciousness: f32,
	pub occupancy: f32,
	pub layout: Option<KitchenCounterLayout>,
}

impl KitchenRegions {
	pub fn pack(&self, confines: &Confines, noise: NoiseParams) -> Result<KitchenPacked, FitError> {
		let mut host = init_host(confines)?;
		if host.room_area + 1e-3 < MIN_AREA {
			return Err(FitError::TooSmall {
				reason: "kitchen",
			});
		}
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let layout = self.layout.unwrap_or_else(|| pick_layout(&cfg, c, host.room_area));

		let depth = (COUNTER_DEPTH * self.spaciousness).clamp(0.5, 0.8);
		let height = COUNTER_HEIGHT * self.spaciousness.min(1.15);
		let along_a = sample_along(&cfg, c, self.spaciousness, host.host, 32.0);
		let along_b = sample_along(&cfg, c, self.spaciousness, host.host, 33.0);

		let mut packed = KitchenPacked {
			layout: Some(layout),
			..KitchenPacked::default()
		};

		match layout {
			KitchenCounterLayout::Galley => {
				let Some(primary) = try_wall_long(
					&host.host3,
					host.host,
					&host.clearances,
					&cfg,
					10,
					WallLongKnobs {
						extent: Vec3::new(along_a, height, depth),
						wall_eps: WALL_EPS,
						attempts: 20,
					},
				) else {
					return Err(FitError::TooSmall {
						reason: "kitchen counter",
					});
				};
				commit_solid(&mut host, &primary);
				packed.counter_runs.push(primary);
			}
			KitchenCounterLayout::LShape => {
				let Some((a, b)) = try_l_shape(
					&host.host3,
					host.host,
					&host.clearances,
					&cfg,
					along_a,
					along_b,
					depth,
					height,
				) else {
					// Soft-fall back to galley if L cannot clear passages.
					let Some(primary) = try_wall_long(
						&host.host3,
						host.host,
						&host.clearances,
						&cfg,
						11,
						WallLongKnobs {
							extent: Vec3::new(along_a, height, depth),
							wall_eps: WALL_EPS,
							attempts: 20,
						},
					) else {
						return Err(FitError::TooSmall {
							reason: "kitchen counter",
						});
					};
					commit_solid(&mut host, &primary);
					packed.counter_runs.push(primary);
					packed.layout = Some(KitchenCounterLayout::Galley);
					return finalize_optional(self, &mut host, &mut packed, &cfg, c);
				};
				commit_solid(&mut host, &a);
				commit_solid(&mut host, &b);
				packed.counter_runs.push(a);
				packed.counter_runs.push(b);
			}
			KitchenCounterLayout::Peninsula => {
				let Some(primary) = try_wall_long(
					&host.host3,
					host.host,
					&host.clearances,
					&cfg,
					12,
					WallLongKnobs {
						extent: Vec3::new(along_a, height, depth),
						wall_eps: WALL_EPS,
						attempts: 20,
					},
				) else {
					return Err(FitError::TooSmall {
						reason: "kitchen counter",
					});
				};
				commit_solid(&mut host, &primary);
				packed.counter_runs.push(primary);
				let pen_along = sample_along(&cfg, c, self.spaciousness * 0.85, host.host, 34.0)
					.clamp(0.9, 2.6);
				if let Some(pen) = try_peninsula_from_run(
					&host.host3,
					host.host,
					&host.clearances,
					&packed.counter_runs[0],
					pen_along,
					depth,
					height,
					&cfg,
					35,
				) {
					commit_solid(&mut host, &pen);
					packed.peninsulas.push(pen);
				}
			}
		}

		finalize_optional(self, &mut host, &mut packed, &cfg, c)
	}
}

fn finalize_optional(
	regions: &KitchenRegions,
	host: &mut PackHost,
	packed: &mut KitchenPacked,
	cfg: &NoiseConfig,
	c: Vec3,
) -> Result<KitchenPacked, FitError> {
	let layout = packed.layout.unwrap_or(KitchenCounterLayout::Galley);
	let height = COUNTER_HEIGHT * regions.spaciousness.min(1.15);

	let island_ok = host.room_area >= 18.0
		&& matches!(
			layout,
			KitchenCounterLayout::Galley | KitchenCounterLayout::LShape
		)
		&& cfg.sample_unit_4d(c.x, c.y, c.z, 36.0) > 0.45;
	if island_ok {
		let ix = cfg.sample_range_f32_4d(0.9, 1.5, c.x, c.y, c.z, 37.0)
			* regions.spaciousness.min(1.2);
		let iz = cfg.sample_range_f32_4d(0.7, 1.1, c.x, c.y, c.z, 38.0)
			* regions.spaciousness.min(1.2);
		if let Some(island) = try_free_extent(
			&host.host3,
			host.host,
			&host.clearances,
			cfg,
			40,
			FreeExtentKnobs {
				extent: Vec3::new(ix, height, iz),
				prefer_wall: false,
				wall_eps: WALL_EPS,
				attempts: 16,
			},
		) {
			if packed_area_ratio(packed, host.room_area) + xz_area(&island) / host.room_area
				<= regions.occupancy + 1e-3
			{
				commit_solid(host, &island);
				packed.islands.push(island);
			}
		}
	}

	if packed_area_ratio(packed, host.room_area) < regions.occupancy * 0.85 {
		if let Some(f) = try_free_extent(
			&host.host3,
			host.host,
			&host.clearances,
			cfg,
			50,
			FreeExtentKnobs {
				extent: Vec3::new(
					0.4 * regions.spaciousness,
					0.5,
					0.4 * regions.spaciousness,
				),
				prefer_wall: true,
				wall_eps: WALL_EPS,
				attempts: 10,
			},
		) {
			if packed_area_ratio(packed, host.room_area) + xz_area(&f) / host.room_area
				<= regions.occupancy + 1e-3
			{
				commit_solid(host, &f);
				packed.fillers.push(f);
			}
		}
	}

	if packed.counter_runs.is_empty() {
		return Err(FitError::TooSmall {
			reason: "kitchen counter",
		});
	}
	Ok(packed.clone())
}

fn sample_along(cfg: &NoiseConfig, c: Vec3, spaciousness: f32, host: Aabb2d, w: f32) -> f32 {
	let max_span = (host.max.x - host.min.x)
		.max(host.max.y - host.min.y)
		.max(1.5);
	cfg.sample_range_f32_4d(
		1.5 * spaciousness,
		(3.2 * spaciousness).min(max_span - 0.15),
		c.x,
		c.y,
		c.z,
		w,
	)
	.clamp(1.35, 4.5)
}

fn pick_layout(cfg: &NoiseConfig, c: Vec3, room_area: f32) -> KitchenCounterLayout {
	let t = cfg.sample_unit_4d(c.x, c.y, c.z, 31.0);
	if room_area < 12.0 {
		return if t < 0.55 {
			KitchenCounterLayout::Galley
		} else {
			KitchenCounterLayout::LShape
		};
	}
	if t < 0.28 {
		KitchenCounterLayout::Galley
	} else if t < 0.62 {
		KitchenCounterLayout::LShape
	} else {
		KitchenCounterLayout::Peninsula
	}
}

fn packed_area_ratio(packed: &KitchenPacked, room_area: f32) -> f32 {
	let a = packed
		.counter_runs
		.iter()
		.chain(packed.peninsulas.iter())
		.chain(packed.islands.iter())
		.chain(packed.fillers.iter())
		.map(xz_area)
		.sum::<f32>();
	a / room_area.max(1e-4)
}

fn commit_solid(host: &mut PackHost, solid: &Aabb3d) {
	host.clearances.push(aabb3_to_plan(solid, PlanAxes::XZ));
}

/// Corner-seeded L: both runs start at the same host corner so they meet.
fn try_l_shape(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	cfg: &NoiseConfig,
	along_a: f32,
	along_b: f32,
	depth: f32,
	height: f32,
) -> Option<(Aabb3d, Aabb3d)> {
	let start = (cfg.sample_unit_4d(0.0, 0.0, 0.0, 45.0) * 4.0).floor() as u32 % 4;
	let y0 = host3.min.y;
	for k in 0..4u32 {
		let corner = (start + k) % 4;
		let (a, b) = corner_l_runs(host, corner, along_a, along_b, depth, height, y0)?;
		let pa = aabb3_to_plan(&a, PlanAxes::XZ);
		let pb = aabb3_to_plan(&b, PlanAxes::XZ);
		if !in_host(host, pa, 1e-3)
			|| !in_host(host, pb, 1e-3)
			|| !long_face_on_wall(host, pa, WALL_EPS)
			|| !long_face_on_wall(host, pb, WALL_EPS)
			|| !clear_of_keep_outs(pa, clearances)
			|| !clear_of_keep_outs(pb, clearances)
		{
			continue;
		}
		// Corner contact: footprints must touch (overlap only in the depth×depth corner square).
		if !plans_touch(pa, pb) {
			continue;
		}
		return Some((a, b));
	}
	None
}

/// `corner`: 0=SW (−X,−Z), 1=SE (+X,−Z), 2=NW (−X,+Z), 3=NE (+X,+Z).
fn corner_l_runs(
	host: Aabb2d,
	corner: u32,
	along_x: f32,
	along_z: f32,
	depth: f32,
	height: f32,
	y0: f32,
) -> Option<(Aabb3d, Aabb3d)> {
	let span_x = (host.max.x - host.min.x).max(0.0);
	let span_z = (host.max.y - host.min.y).max(0.0);
	let ax = along_x.min(span_x);
	let az = along_z.min(span_z);
	if ax < depth + 0.2 || az < depth + 0.2 {
		return None;
	}
	let (run_x_min, run_z_min) = match corner {
		0 => {
			// SW: +X run on −Z wall, +Z run on −X wall
			let run_x = Aabb3d::from_min_max(
				Vec3::new(host.min.x, y0, host.min.y),
				Vec3::new(host.min.x + ax, y0 + height, host.min.y + depth),
			);
			let run_z = Aabb3d::from_min_max(
				Vec3::new(host.min.x, y0, host.min.y),
				Vec3::new(host.min.x + depth, y0 + height, host.min.y + az),
			);
			(run_x, run_z)
		}
		1 => {
			// SE
			let run_x = Aabb3d::from_min_max(
				Vec3::new(host.max.x - ax, y0, host.min.y),
				Vec3::new(host.max.x, y0 + height, host.min.y + depth),
			);
			let run_z = Aabb3d::from_min_max(
				Vec3::new(host.max.x - depth, y0, host.min.y),
				Vec3::new(host.max.x, y0 + height, host.min.y + az),
			);
			(run_x, run_z)
		}
		2 => {
			// NW
			let run_x = Aabb3d::from_min_max(
				Vec3::new(host.min.x, y0, host.max.y - depth),
				Vec3::new(host.min.x + ax, y0 + height, host.max.y),
			);
			let run_z = Aabb3d::from_min_max(
				Vec3::new(host.min.x, y0, host.max.y - az),
				Vec3::new(host.min.x + depth, y0 + height, host.max.y),
			);
			(run_x, run_z)
		}
		_ => {
			// NE
			let run_x = Aabb3d::from_min_max(
				Vec3::new(host.max.x - ax, y0, host.max.y - depth),
				Vec3::new(host.max.x, y0 + height, host.max.y),
			);
			let run_z = Aabb3d::from_min_max(
				Vec3::new(host.max.x - depth, y0, host.max.y - az),
				Vec3::new(host.max.x, y0 + height, host.max.y),
			);
			(run_x, run_z)
		}
	};
	Some((run_x_min, run_z_min))
}

fn plans_touch(a: Aabb2d, b: Aabb2d) -> bool {
	const EPS: f32 = 0.05;
	let gap_x = (a.min.x - b.max.x).max(b.min.x - a.max.x);
	let gap_y = (a.min.y - b.max.y).max(b.min.y - a.max.y);
	gap_x <= EPS && gap_y <= EPS
}

/// Which host wall a plan AABB is flush against (0=−Z, 1=+Z, 2=−X, 3=+X).
fn wall_of(plan: Aabb2d, host: Aabb2d) -> Option<u32> {
	if (plan.min.y - host.min.y).abs() <= WALL_EPS {
		Some(0)
	} else if (plan.max.y - host.max.y).abs() <= WALL_EPS {
		Some(1)
	} else if (plan.min.x - host.min.x).abs() <= WALL_EPS {
		Some(2)
	} else if (plan.max.x - host.max.x).abs() <= WALL_EPS {
		Some(3)
	} else {
		None
	}
}

/// Peninsula stub from one end of a wall run into the room (perpendicular).
fn try_peninsula_from_run(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	primary: &Aabb3d,
	along: f32,
	depth: f32,
	height: f32,
	cfg: &NoiseConfig,
	salt: u32,
) -> Option<Aabb3d> {
	let p = aabb3_to_plan(primary, PlanAxes::XZ);
	let wall = wall_of(p, host)?;
	let use_hi = cfg.sample_unit_4d(salt as f32, 0.0, 0.0, 44.0) >= 0.5;
	let y0 = host3.min.y;
	let candidate = match wall {
		0 => {
			let x0 = if use_hi { p.max.x - depth } else { p.min.x };
			let min = Vec3::new(x0, y0, p.max.y);
			Aabb3d::from_min_max(min, min + Vec3::new(depth, height, along))
		}
		1 => {
			let x0 = if use_hi { p.max.x - depth } else { p.min.x };
			let min = Vec3::new(x0, y0, p.min.y - along);
			Aabb3d::from_min_max(min, min + Vec3::new(depth, height, along))
		}
		2 => {
			let z0 = if use_hi { p.max.y - depth } else { p.min.y };
			let min = Vec3::new(p.max.x, y0, z0);
			Aabb3d::from_min_max(min, min + Vec3::new(along, height, depth))
		}
		_ => {
			let z0 = if use_hi { p.max.y - depth } else { p.min.y };
			let min = Vec3::new(p.min.x - along, y0, z0);
			Aabb3d::from_min_max(min, min + Vec3::new(along, height, depth))
		}
	};
	let plan = aabb3_to_plan(&candidate, PlanAxes::XZ);
	if !in_host(host, plan, 1e-3) || !plans_touch(plan, p) {
		return None;
	}
	let others: Vec<Aabb2d> = clearances
		.iter()
		.copied()
		.filter(|c| !aabb2_near_eq(*c, p, 1e-3))
		.collect();
	if !clear_of_keep_outs(plan, &others) {
		return None;
	}
	// Reject if the long face lands on a second host wall (that is an L, not a peninsula).
	let w = plan.max.x - plan.min.x;
	let d = plan.max.y - plan.min.y;
	let long_on_wall = if w >= d {
		(plan.min.y - host.min.y).abs() <= WALL_EPS || (plan.max.y - host.max.y).abs() <= WALL_EPS
	} else {
		(plan.min.x - host.min.x).abs() <= WALL_EPS || (plan.max.x - host.max.x).abs() <= WALL_EPS
	};
	if long_on_wall {
		return None;
	}
	Some(candidate)
}

fn aabb2_near_eq(a: Aabb2d, b: Aabb2d, eps: f32) -> bool {
	(a.min.x - b.min.x).abs() < eps
		&& (a.min.y - b.min.y).abs() < eps
		&& (a.max.x - b.max.x).abs() < eps
		&& (a.max.y - b.max.y).abs() < eps
}
