//! Plan-space cell split, adjacency, and hallway-frontage grouping.
//!
//! Shared by storey typologies that pack multi-cell apartments (or similar
//! program groups) from residual rectangles after halls/shafts are carved.

use bevy_math::bounding::Aabb2d;
use bevy_math::Vec2;
use procedural_common::{aabb2_area, Aabb2dPack};

const EPS: f32 = 1e-3;

/// Default minimum shared-edge length (m) for grouping cells into one apartment.
///
/// Shorter contacts are treated as pinches — adjacent for walls, but not for
/// apartment connectivity / grow / absorb.
pub const MIN_GROUP_CONNECTIVITY: f32 = 1.5;

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
	shared_edge_length(a.bounds, b.bounds, eps).is_some_and(|len| len > eps)
}

/// True when two cells share an edge of at least `min_length` (well-connected).
pub fn cells_well_connected(a: &PlanCell, b: &PlanCell, min_length: f32, eps: f32) -> bool {
	let min_length = min_length.max(eps);
	shared_edge_length(a.bounds, b.bounds, eps).is_some_and(|len| len + eps >= min_length)
}

/// Length of the shared edge between two AABBs, if they touch with positive overlap.
pub fn shared_edge_length(a: Aabb2d, b: Aabb2d, eps: f32) -> Option<f32> {
	let touch_x = (a.max.x - b.min.x).abs() <= eps || (b.max.x - a.min.x).abs() <= eps;
	if touch_x {
		let lo = a.min.y.max(b.min.y);
		let hi = a.max.y.min(b.max.y);
		let len = hi - lo;
		if len > eps {
			return Some(len);
		}
	}
	let touch_y = (a.max.y - b.min.y).abs() <= eps || (b.max.y - a.min.y).abs() <= eps;
	if touch_y {
		let lo = a.min.x.max(b.min.x);
		let hi = a.max.x.min(b.max.x);
		let len = hi - lo;
		if len > eps {
			return Some(len);
		}
	}
	None
}

fn edge_adjacent_aabb2(a: Aabb2d, b: Aabb2d, eps: f32) -> bool {
	shared_edge_length(a, b, eps).is_some()
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
/// Convenience wrapper around [`pack_apartments_to_targets`] with one target and
/// [`MIN_GROUP_CONNECTIVITY`].
pub fn group_cells_to_apartments(
	cells: &[PlanCell],
	halls: &[Aabb2d],
	min_room_size: Vec2,
	target_apartment_area: f32,
) -> Vec<Vec<u32>> {
	pack_apartments_to_targets(
		cells,
		halls,
		min_room_size,
		&[target_apartment_area.max(EPS)],
		MIN_GROUP_CONNECTIVITY,
	)
}

/// Pack hall-connected apartments against a target-area catalog (Les Halles door style).
///
/// Walks `targets` in order. Each entry seeds an unassigned hall-frontage cell and
/// grows by **well-connected** neighbors (shared edge ≥ `min_connectivity`) until
/// the group area reaches the target (within ~15% undershoot). Pinch contacts
/// shorter than `min_connectivity` do not join groups. Targets that cannot seed
/// are skipped. Remaining frontage cells become force-one groups; orphans absorb
/// into neighboring well-connected groups.
///
/// All input cells are considered (no min-size discard). `min_room_size` is kept
/// for API stability but currently unused.
///
/// Never emits a landlocked group (no hall frontage).
pub fn pack_apartments_to_targets(
	cells: &[PlanCell],
	halls: &[Aabb2d],
	_min_room_size: Vec2,
	targets: &[f32],
	min_connectivity: f32,
) -> Vec<Vec<u32>> {
	let min_conn = min_connectivity.max(EPS);
	// Keep every residual cell — discarding small scraps left gaps in group
	// boundaries / wall runs after shaft-clearance carving.
	let eligible: Vec<PlanCell> = cells.to_vec();
	if eligible.is_empty() {
		return Vec::new();
	}

	let frontage: Vec<bool> = eligible
		.iter()
		.map(|c| cell_has_hall_frontage(c, halls, EPS))
		.collect();

	let mut assigned = vec![false; eligible.len()];
	let mut groups: Vec<Vec<usize>> = Vec::new();

	for &raw_target in targets {
		let target = raw_target.max(EPS);
		let Some(seed) = best_seed(&eligible, &assigned, &frontage, target) else {
			continue;
		};
		let mut group = vec![seed];
		assigned[seed] = true;
		let mut area = eligible[seed].area();
		// ~15% undershoot is acceptable (bay `allowed_error` analog).
		let accept = target * 0.85;
		while area + EPS < target {
			let Some(next) = best_grow_candidate(&eligible, &assigned, &group, min_conn) else {
				break;
			};
			let next_area = eligible[next].area();
			// Stop if already in the accept band and the next cell would overshoot hard.
			if area + EPS >= accept && area + next_area > target * 1.35 {
				break;
			}
			assigned[next] = true;
			group.push(next);
			area += next_area;
		}
		groups.push(group);
	}

	// Absorb orphans into a neighboring group that already has hall frontage.
	for i in 0..eligible.len() {
		if assigned[i] {
			continue;
		}
		if let Some(gi) = find_absorb_group(&eligible, &groups, i, min_conn) {
			groups[gi].push(i);
			assigned[i] = true;
		}
	}

	// Force-one: leftover hall-frontage cells that never matched a catalog target.
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

/// Guillotine-split cells whose area exceeds `max_area`, stopping at `min_room`.
pub fn split_oversized_cells(
	cells: &[PlanCell],
	max_area: f32,
	min_room: Vec2,
	next_id: &mut u32,
) -> Vec<PlanCell> {
	let max_area = max_area.max(EPS);
	let min_room = Vec2::new(min_room.x.max(EPS), min_room.y.max(EPS));
	let mut queue: Vec<PlanCell> = cells.to_vec();
	let mut out = Vec::new();
	while let Some(cell) = queue.pop() {
		if cell.area() <= max_area + EPS {
			out.push(cell);
			continue;
		}
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

fn best_seed(
	cells: &[PlanCell],
	assigned: &[bool],
	frontage: &[bool],
	target: f32,
) -> Option<usize> {
	// Prefer unassigned hall-frontage seeds. Large catalog entries first → start
	// from smaller seeds that still have room to grow toward the target.
	let mut best: Option<(usize, f32)> = None;
	for (i, cell) in cells.iter().enumerate() {
		if assigned[i] || !frontage[i] {
			continue;
		}
		let area = cell.area();
		// Score: prefer seeds below the target (room to grow), then larger seeds.
		let score = if area + EPS < target {
			area + target // prefer larger under-target seeds
		} else {
			// Already oversized for this slot — only use if nothing smaller remains.
			-area
		};
		match best {
			None => best = Some((i, score)),
			Some((_, bs)) if score > bs => best = Some((i, score)),
			_ => {}
		}
	}
	best.map(|(i, _)| i)
}

fn best_grow_candidate(
	cells: &[PlanCell],
	assigned: &[bool],
	group: &[usize],
	min_connectivity: f32,
) -> Option<usize> {
	let mut best: Option<(usize, f32)> = None;
	for (i, cell) in cells.iter().enumerate() {
		if assigned[i] {
			continue;
		}
		let touches = group
			.iter()
			.any(|&gi| cells_well_connected(&cells[gi], cell, min_connectivity, EPS));
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

fn find_absorb_group(
	cells: &[PlanCell],
	groups: &[Vec<usize>],
	orphan: usize,
	min_connectivity: f32,
) -> Option<usize> {
	let mut best: Option<(usize, f32)> = None;
	for (gi, group) in groups.iter().enumerate() {
		let touches = group.iter().any(|&ci| {
			cells_well_connected(&cells[ci], &cells[orphan], min_connectivity, EPS)
		});
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
	fn pack_catalog_builds_multi_cell_group() {
		// Four 3×3 cells along a hall; one 30 m² target should absorb more than one.
		let rooms = vec![
			cell(0, Vec2::new(0.0, 0.0), Vec2::new(3.0, 3.0)),
			cell(1, Vec2::new(3.0, 0.0), Vec2::new(6.0, 3.0)),
			cell(2, Vec2::new(6.0, 0.0), Vec2::new(9.0, 3.0)),
			cell(3, Vec2::new(9.0, 0.0), Vec2::new(12.0, 3.0)),
		];
		let hall = Aabb2d {
			min: Vec2::new(0.0, 3.0),
			max: Vec2::new(12.0, 5.0),
		};
		let groups = pack_apartments_to_targets(
			&rooms,
			&[hall],
			Vec2::new(2.0, 2.0),
			&[30.0, 20.0],
			MIN_GROUP_CONNECTIVITY,
		);
		assert!(
			groups.iter().any(|g| g.len() >= 2),
			"expected a multi-cell apartment, got {groups:?}"
		);
	}

	#[test]
	fn pinch_contact_does_not_group() {
		// Full-side neighbor vs pinch (0.4 m shared) — only the wide contact groups.
		let rooms = vec![
			cell(0, Vec2::new(0.0, 0.0), Vec2::new(4.0, 4.0)),
			cell(1, Vec2::new(4.0, 0.0), Vec2::new(8.0, 4.0)), // 4 m shared
			cell(2, Vec2::new(4.0, 3.6), Vec2::new(8.0, 7.6)), // 0.4 m pinch with cell 0
		];
		let hall = Aabb2d {
			min: Vec2::new(0.0, -2.0),
			max: Vec2::new(8.0, 0.0),
		};
		assert!(cells_edge_adjacent(&rooms[0], &rooms[2], EPS));
		assert!(!cells_well_connected(
			&rooms[0],
			&rooms[2],
			MIN_GROUP_CONNECTIVITY,
			EPS
		));
		assert!(cells_well_connected(
			&rooms[0],
			&rooms[1],
			MIN_GROUP_CONNECTIVITY,
			EPS
		));

		let groups = pack_apartments_to_targets(
			&rooms,
			&[hall],
			Vec2::new(2.0, 2.0),
			&[40.0],
			MIN_GROUP_CONNECTIVITY,
		);
		// Cell 0+1 may group; cell 2 must not join via the pinch alone.
		for g in &groups {
			if g.contains(&2) {
				assert!(
					!g.contains(&0),
					"pinch must not join cell 2 with cell 0: {groups:?}"
				);
			}
		}
	}

	#[test]
	fn split_oversized_respects_max_area() {
		let mut next = 1;
		let cells = vec![cell(0, Vec2::new(0.0, 0.0), Vec2::new(10.0, 8.0))];
		let out = split_oversized_cells(&cells, 20.0, Vec2::new(2.5, 2.5), &mut next);
		assert!(out.len() >= 2);
		assert!(out.iter().all(|c| c.area() <= 20.0 + 1.0));
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
