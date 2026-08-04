//! Plan-space cell split, adjacency, and hallway-frontage grouping.
//!
//! Shared by storey typologies that pack multi-cell apartments (or similar
//! program groups) from residual rectangles after halls/shafts are carved.

use bevy_math::bounding::Aabb2d;
use bevy_math::Vec2;
use procedural_common::{aabb2_area, Aabb2dPack};

const EPS: f32 = 1e-3;

/// One axis-aligned plan cell (`Aabb2d` uses \(x → X\), \(y → Z\)).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanCell {
	pub id: u32,
	pub bounds: Aabb2d,
}

impl PlanCell {
	pub fn new(id: u32, bounds: Aabb2d) -> Self {
		Self { id, bounds }
	}

	pub fn area(self) -> f32 {
		aabb2_area(self.bounds)
	}

	pub fn size(self) -> Vec2 {
		self.bounds.max - self.bounds.min
	}

	pub fn center(self) -> Vec2 {
		0.5 * (self.bounds.min + self.bounds.max)
	}
}

/// True when two cells share an edge (closed contact with positive overlap length).
pub fn cells_edge_adjacent(a: &PlanCell, b: &PlanCell, eps: f32) -> bool {
	edge_adjacent_aabb2(a.bounds, b.bounds, eps)
}

fn edge_adjacent_aabb2(a: Aabb2d, b: Aabb2d, eps: f32) -> bool {
	let x_overlap = a.min.x.min(b.max.x) < a.max.x.max(b.min.x) - eps
		&& (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)) > eps;
	let y_overlap = a.min.y.min(b.max.y) < a.max.y.max(b.min.y) - eps
		&& (a.max.y.min(b.max.y) - a.min.y.max(b.min.y)) > eps;
	let touch_x = (a.max.x - b.min.x).abs() <= eps || (b.max.x - a.min.x).abs() <= eps;
	let touch_y = (a.max.y - b.min.y).abs() <= eps || (b.max.y - a.min.y).abs() <= eps;
	(touch_x && y_overlap) || (touch_y && x_overlap)
}

/// True when `cell` shares an edge with any hallway band.
pub fn cell_has_hall_frontage(cell: &PlanCell, halls: &[Aabb2d], eps: f32) -> bool {
	halls.iter().any(|h| edge_adjacent_aabb2(cell.bounds, *h, eps))
}

/// Guillotine-split oversized cells toward `min_room` extents.
///
/// Prefer cutting the longer axis near half when both halves stay ≥ `min_room`.
/// Stops when a cell cannot be split without violating mins.
pub fn split_toward_min_room(
	cells: &[PlanCell],
	min_room: Vec2,
	next_id: &mut u32,
) -> Vec<PlanCell> {
	let min_room = Vec2::new(min_room.x.max(EPS), min_room.y.max(EPS));
	let mut queue: Vec<PlanCell> = cells.to_vec();
	let mut out = Vec::new();
	while let Some(cell) = queue.pop() {
		let size = cell.size();
		let can_cut_x = size.x >= 2.0 * min_room.x - EPS;
		let can_cut_z = size.y >= 2.0 * min_room.y - EPS;
		if !can_cut_x && !can_cut_z {
			out.push(cell);
			continue;
		}
		let cut_x = if can_cut_x && can_cut_z {
			size.x >= size.y
		} else {
			can_cut_x
		};
		let (a, b) = cell.bounds.bipartition_by_area(cut_x, true, 0.5);
		let sa = a.max - a.min;
		let sb = b.max - b.min;
		if sa.x + EPS < min_room.x
			|| sa.y + EPS < min_room.y
			|| sb.x + EPS < min_room.x
			|| sb.y + EPS < min_room.y
		{
			out.push(cell);
			continue;
		}
		let id_a = cell.id;
		let id_b = *next_id;
		*next_id = next_id.saturating_add(1);
		queue.push(PlanCell::new(id_a, a));
		queue.push(PlanCell::new(id_b, b));
	}
	out
}

/// Group room cells into edge-connected apartments that touch a hallway.
///
/// - Seeds from hall-frontage cells.
/// - Grows by absorbing edge-adjacent cells until `target_apartment_area`.
/// - Absorbs remaining orphans into a neighboring hall-reaching group when possible.
/// - Never emits a landlocked group (no hall frontage).
pub fn group_cells_to_apartments(
	cells: &[PlanCell],
	halls: &[Aabb2d],
	min_room_size: Vec2,
	target_apartment_area: f32,
) -> Vec<Vec<u32>> {
	let target = target_apartment_area.max(EPS);
	let min_room = Vec2::new(min_room_size.x.max(EPS), min_room_size.y.max(EPS));
	let eligible: Vec<PlanCell> = cells
		.iter()
		.copied()
		.filter(|c| {
			let s = c.size();
			s.x + EPS >= min_room.x && s.y + EPS >= min_room.y
		})
		.collect();
	if eligible.is_empty() {
		return Vec::new();
	}

	let frontage: Vec<bool> = eligible
		.iter()
		.map(|c| cell_has_hall_frontage(c, halls, EPS))
		.collect();

	let mut assigned = vec![false; eligible.len()];
	let mut groups: Vec<Vec<usize>> = Vec::new();

	// Grow from hall-frontage seeds first (largest frontage cell first).
	let mut seed_order: Vec<usize> = (0..eligible.len()).filter(|&i| frontage[i]).collect();
	seed_order.sort_by(|&a, &b| {
		eligible[b]
			.area()
			.partial_cmp(&eligible[a].area())
			.unwrap_or(std::cmp::Ordering::Equal)
	});

	for seed in seed_order {
		if assigned[seed] {
			continue;
		}
		let mut group = vec![seed];
		assigned[seed] = true;
		let mut area = eligible[seed].area();
		while area + EPS < target {
			let Some(next) = best_grow_candidate(&eligible, &assigned, &group) else {
				break;
			};
			assigned[next] = true;
			group.push(next);
			area += eligible[next].area();
		}
		groups.push(group);
	}

	// Absorb orphans into a neighboring group that already has hall frontage.
	for i in 0..eligible.len() {
		if assigned[i] {
			continue;
		}
		if let Some(gi) = find_absorb_group(&eligible, &groups, i) {
			groups[gi].push(i);
			assigned[i] = true;
		}
	}

	// Singleton hall-frontage leftovers that never seeded (should be rare).
	for i in 0..eligible.len() {
		if assigned[i] {
			continue;
		}
		if frontage[i] {
			groups.push(vec![i]);
			assigned[i] = true;
		}
	}

	groups
		.into_iter()
		.filter(|g| g.iter().any(|&i| frontage[i]))
		.map(|g| g.into_iter().map(|i| eligible[i].id).collect())
		.collect()
}

fn best_grow_candidate(
	cells: &[PlanCell],
	assigned: &[bool],
	group: &[usize],
) -> Option<usize> {
	let mut best: Option<(usize, f32)> = None;
	for (i, cell) in cells.iter().enumerate() {
		if assigned[i] {
			continue;
		}
		let touches = group
			.iter()
			.any(|&gi| cells_edge_adjacent(&cells[gi], cell, EPS));
		if !touches {
			continue;
		}
		let area = cell.area();
		match best {
			None => best = Some((i, area)),
			Some((_, ba)) if area > ba => best = Some((i, area)),
			_ => {}
		}
	}
	best.map(|(i, _)| i)
}

fn find_absorb_group(cells: &[PlanCell], groups: &[Vec<usize>], orphan: usize) -> Option<usize> {
	let mut best: Option<(usize, f32)> = None;
	for (gi, group) in groups.iter().enumerate() {
		let touches = group
			.iter()
			.any(|&ci| cells_edge_adjacent(&cells[ci], &cells[orphan], EPS));
		if !touches {
			continue;
		}
		let area: f32 = group.iter().map(|&ci| cells[ci].area()).sum();
		match best {
			None => best = Some((gi, area)),
			Some((_, ba)) if area < ba => best = Some((gi, area)),
			_ => {}
		}
	}
	best.map(|(gi, _)| gi)
}

/// Subtract axis-aligned `cuts` from `host`, returning residual rectangles.
///
/// Uses a simple guillotine difference against each cut in order.
pub fn subtract_aabb2(host: Aabb2d, cuts: &[Aabb2d]) -> Vec<Aabb2d> {
	let mut regions = vec![host];
	for cut in cuts {
		let mut next = Vec::new();
		for r in regions {
			next.extend(subtract_one(r, *cut));
		}
		regions = next;
	}
	regions
		.into_iter()
		.filter(|r| aabb2_area(*r) > EPS * EPS)
		.collect()
}

fn subtract_one(host: Aabb2d, cut: Aabb2d) -> Vec<Aabb2d> {
	let x0 = host.min.x.max(cut.min.x);
	let x1 = host.max.x.min(cut.max.x);
	let y0 = host.min.y.max(cut.min.y);
	let y1 = host.max.y.min(cut.max.y);
	if x1 - x0 <= EPS || y1 - y0 <= EPS {
		return vec![host];
	}
	let mut out = Vec::new();
	// Left
	if x0 - host.min.x > EPS {
		out.push(Aabb2d {
			min: host.min,
			max: Vec2::new(x0, host.max.y),
		});
	}
	// Right
	if host.max.x - x1 > EPS {
		out.push(Aabb2d {
			min: Vec2::new(x1, host.min.y),
			max: host.max,
		});
	}
	// Bottom (between left/right slabs)
	if y0 - host.min.y > EPS {
		out.push(Aabb2d {
			min: Vec2::new(x0, host.min.y),
			max: Vec2::new(x1, y0),
		});
	}
	// Top
	if host.max.y - y1 > EPS {
		out.push(Aabb2d {
			min: Vec2::new(x0, y1),
			max: Vec2::new(x1, host.max.y),
		});
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	fn cell(id: u32, min: Vec2, max: Vec2) -> PlanCell {
		PlanCell::new(
			id,
			Aabb2d {
				min,
				max,
			},
		)
	}

	#[test]
	fn edge_adjacent_detects_shared_side() {
		let a = cell(0, Vec2::new(0.0, 0.0), Vec2::new(4.0, 4.0));
		let b = cell(1, Vec2::new(4.0, 0.0), Vec2::new(8.0, 4.0));
		let c = cell(2, Vec2::new(5.0, 0.0), Vec2::new(9.0, 4.0));
		assert!(cells_edge_adjacent(&a, &b, EPS));
		assert!(!cells_edge_adjacent(&a, &c, EPS));
	}

	#[test]
	fn split_toward_min_room_halves_long_run() {
		let mut next = 1;
		let cells = vec![cell(0, Vec2::new(0.0, 0.0), Vec2::new(12.0, 4.0))];
		let out = split_toward_min_room(&cells, Vec2::new(3.0, 3.0), &mut next);
		assert!(out.len() >= 2);
		assert!(out.iter().all(|c| c.size().x + EPS >= 3.0 && c.size().y + EPS >= 3.0));
	}

	#[test]
	fn group_requires_hall_frontage() {
		let rooms = vec![
			cell(0, Vec2::new(0.0, 0.0), Vec2::new(4.0, 4.0)),
			cell(1, Vec2::new(4.0, 0.0), Vec2::new(8.0, 4.0)),
			// Isolated: no hall edge, no adjacency to other rooms.
			cell(2, Vec2::new(20.0, 20.0), Vec2::new(24.0, 24.0)),
		];
		let hall = Aabb2d {
			min: Vec2::new(0.0, 4.0),
			max: Vec2::new(8.0, 6.0),
		};
		let groups = group_cells_to_apartments(&rooms, &[hall], Vec2::new(2.0, 2.0), 20.0);
		let flat: Vec<u32> = groups.iter().flatten().copied().collect();
		assert!(flat.contains(&0));
		assert!(flat.contains(&1));
		assert!(!flat.contains(&2));
		assert!(groups.iter().all(|g| !g.is_empty()));
		// Every emitted group still touches the hall (via at least one piece).
		for g in &groups {
			assert!(g.iter().any(|&id| {
				let c = rooms.iter().find(|c| c.id == id).unwrap();
				cell_has_hall_frontage(c, &[hall], EPS)
			}));
		}
	}

	#[test]
	fn subtract_carves_corridor() {
		let host = Aabb2d {
			min: Vec2::new(0.0, 0.0),
			max: Vec2::new(10.0, 10.0),
		};
		let hall = Aabb2d {
			min: Vec2::new(0.0, 4.0),
			max: Vec2::new(10.0, 6.0),
		};
		let rem = subtract_aabb2(host, &[hall]);
		assert!(rem.len() >= 2);
		assert!(rem.iter().all(|r| aabb2_area(*r) > 0.0));
	}
}
