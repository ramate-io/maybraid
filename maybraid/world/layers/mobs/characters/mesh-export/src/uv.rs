//! Connected-chart UV unwrap packed into a padded unit square per texture set.

use std::collections::{HashMap, VecDeque};
use std::f32::consts::PI;

use bevy::math::{Vec2, Vec3};

/// Gap between packed islands, as a fraction of the unit square (~25 px at 2048).
pub const ISLAND_GAP: f32 = 0.012;
/// Margin from the 0..1 border (~41 px at 2048).
pub const UNIT_BORDER: f32 = 0.02;
/// Grow a chart while the face normal stays within this angle of the seed.
const MAX_SEED_ANGLE: f32 = 70.0 * PI / 180.0;
const EPS: f32 = 1e-5;
/// Inflate a collapsed chart axis so slivers still have UV area.
const MIN_CHART_ASPECT: f32 = 0.08;

/// Axis-aligned UV island.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UvIsland {
	pub min: Vec2,
	pub max: Vec2,
}

impl UvIsland {
	pub fn from_uvs(uvs: &[Vec2]) -> Option<Self> {
		if uvs.is_empty() {
			return None;
		}
		let mut min = Vec2::splat(f32::MAX);
		let mut max = Vec2::splat(f32::MIN);
		for uv in uvs {
			min = min.min(*uv);
			max = max.max(*uv);
		}
		Some(Self { min, max })
	}

	/// Strict interior overlap. Edge-touch is not overlap.
	pub fn overlaps(self, other: Self) -> bool {
		self.min.x < other.max.x - EPS
			&& self.max.x > other.min.x + EPS
			&& self.min.y < other.max.y - EPS
			&& self.max.y > other.min.y + EPS
	}

	pub fn in_unit_square(self) -> bool {
		self.min.x >= -EPS
			&& self.min.y >= -EPS
			&& self.max.x <= 1.0 + EPS
			&& self.max.y <= 1.0 + EPS
	}

	pub fn size(self) -> Vec2 {
		self.max - self.min
	}
}

/// Indexed triangle in 3D.
#[derive(Debug, Clone, Copy)]
pub struct Triangle3 {
	pub a: Vec3,
	pub b: Vec3,
	pub c: Vec3,
}

impl Triangle3 {
	pub fn from_indices(positions: &[Vec3], face: [u32; 3]) -> Option<Self> {
		let a = *positions.get(face[0] as usize)?;
		let b = *positions.get(face[1] as usize)?;
		let c = *positions.get(face[2] as usize)?;
		Some(Self { a, b, c })
	}

	pub fn area(self) -> f32 {
		0.5 * (self.b - self.a).cross(self.c - self.a).length()
	}

	pub fn normal(self) -> Option<Vec3> {
		let n = (self.b - self.a).cross(self.c - self.a);
		let len = n.length();
		(len > 1e-12).then_some(n / len)
	}

	/// Scale-aware degeneracy: area compared to the mesh AABB extent squared.
	pub fn is_degenerate(self, position_scale: f32) -> bool {
		let scale = position_scale.max(1e-6);
		self.area() < 1e-12 * scale * scale
	}
}

/// Indexed triangle in UV space.
#[derive(Debug, Clone, Copy)]
pub struct Triangle2 {
	pub a: Vec2,
	pub b: Vec2,
	pub c: Vec2,
}

impl Triangle2 {
	pub fn from_indices(uvs: &[Vec2], face: [u32; 3]) -> Option<Self> {
		let a = *uvs.get(face[0] as usize)?;
		let b = *uvs.get(face[1] as usize)?;
		let c = *uvs.get(face[2] as usize)?;
		Some(Self { a, b, c })
	}

	pub fn area(self) -> f32 {
		0.5 * ((self.b - self.a).perp_dot(self.c - self.a)).abs()
	}

	/// Scale-aware UV collapse: area compared to the island span squared.
	pub fn is_collapsed(self, uv_scale: f32) -> bool {
		let scale = uv_scale.max(1e-6);
		self.area() < 1e-8 * scale * scale
	}

	/// True when a nondegenerate 3D triangle maps to ~zero UV relative to the mesh density.
	pub fn is_projection_collapsed(self, geom: Triangle3, median_density: f32) -> bool {
		let uv_area = self.area();
		if uv_area < 1e-16 {
			return true;
		}
		let geom_area = geom.area();
		if geom_area <= 1e-20 {
			return false;
		}
		let density = uv_area / geom_area;
		let floor = median_density.abs().max(1e-12) * 1e-4;
		density < floor
	}

	pub fn aabb(self) -> UvIsland {
		UvIsland { min: self.a.min(self.b).min(self.c), max: self.a.max(self.b).max(self.c) }
	}

	/// Interior overlap. Shared edges / vertices of adjacent faces are allowed.
	pub fn overlaps_interior(self, other: Self) -> bool {
		if !self.aabb().overlaps(other.aabb()) {
			return false;
		}
		if self.area() <= 1e-16 || other.area() <= 1e-16 {
			return false;
		}
		point_strictly_inside(self.a, other)
			|| point_strictly_inside(self.b, other)
			|| point_strictly_inside(self.c, other)
			|| point_strictly_inside(other.a, self)
			|| point_strictly_inside(other.b, self)
			|| point_strictly_inside(other.c, self)
			|| edges_cross_interior(self, other)
	}
}

/// Reindexed mesh whose UVs are a packed chart unwrap in 0..1.
#[derive(Debug, Clone)]
pub struct UnwrappedMesh {
	pub positions: Vec<Vec3>,
	pub normals: Vec<Vec3>,
	pub uvs: Vec<Vec2>,
	pub faces: Vec<[u32; 3]>,
	pub chart_count: usize,
}

impl UnwrappedMesh {
	/// Connected, orientation-limited charts, projected, then packed with padding.
	pub fn from_indexed(positions: &[Vec3], normals: &[Vec3], faces: &[[u32; 3]]) -> Self {
		let position_scale = aabb_extent(positions);
		let paintable: Vec<[u32; 3]> = faces
			.iter()
			.copied()
			.filter(|face| {
				Triangle3::from_indices(positions, *face)
					.is_some_and(|tri| !tri.is_degenerate(position_scale))
			})
			.collect();
		if paintable.is_empty() {
			return Self {
				positions: positions.to_vec(),
				normals: normals.to_vec(),
				uvs: vec![Vec2::splat(0.5); positions.len()],
				faces: Vec::new(),
				chart_count: 0,
			};
		}

		let mut charts = ChartGrower::new(positions, &paintable).grow();
		let mut projected: Vec<ProjectedChart> = charts
			.iter()
			.map(|chart| ProjectedChart::from_chart(positions, &paintable, chart))
			.collect();
		split_collapsed_faces(positions, &paintable, &mut charts, &mut projected);
		let packed = ChartPacker::pack(&projected);
		Self::emit(positions, normals, &paintable, &charts, &projected, &packed)
	}

	fn emit(
		positions: &[Vec3],
		normals: &[Vec3],
		paintable: &[[u32; 3]],
		charts: &[Chart],
		projected: &[ProjectedChart],
		packed: &[PackedChart],
	) -> Self {
		let mut out_positions = Vec::new();
		let mut out_normals = Vec::new();
		let mut out_uvs = Vec::new();
		let mut out_faces = Vec::new();

		for (chart, (proj, pack)) in charts.iter().zip(projected.iter().zip(packed)) {
			let mut remap: HashMap<u32, u32> = HashMap::new();
			for &(orig, local) in &proj.local {
				let uv = pack.uv(local, proj.size);
				let new_index = out_positions.len() as u32;
				out_positions.push(positions[orig as usize]);
				let normal =
					normals.get(orig as usize).copied().filter(|n| n.length_squared() > 1e-12);
				out_normals.push(normal.map(|n| n.normalize()).unwrap_or(Vec3::Y));
				out_uvs.push(uv);
				remap.insert(orig, new_index);
			}
			for &face_i in &chart.face_indices {
				let face = paintable[face_i];
				let Some(a) = remap.get(&face[0]).copied() else {
					continue;
				};
				let Some(b) = remap.get(&face[1]).copied() else {
					continue;
				};
				let Some(c) = remap.get(&face[2]).copied() else {
					continue;
				};
				out_faces.push([a, b, c]);
			}
		}

		Self {
			chart_count: charts.len(),
			positions: out_positions,
			normals: out_normals,
			uvs: out_uvs,
			faces: out_faces,
		}
	}
}

struct Chart {
	face_indices: Vec<usize>,
	seed_normal: Vec3,
}

struct ChartGrower<'a> {
	positions: &'a [Vec3],
	faces: &'a [[u32; 3]],
	normals: Vec<Vec3>,
	adjacency: Vec<Vec<usize>>,
}

impl<'a> ChartGrower<'a> {
	fn new(positions: &'a [Vec3], faces: &'a [[u32; 3]]) -> Self {
		let normals: Vec<Vec3> = faces
			.iter()
			.map(|face| {
				Triangle3::from_indices(positions, *face)
					.and_then(Triangle3::normal)
					.unwrap_or(Vec3::Y)
			})
			.collect();
		let adjacency = face_adjacency(positions, faces);
		Self { positions, faces, normals, adjacency }
	}

	fn grow(&self) -> Vec<Chart> {
		let n = self.faces.len();
		let mut assigned = vec![false; n];
		let mut order: Vec<usize> = (0..n).collect();
		order.sort_by(|&a, &b| {
			let aa = Triangle3::from_indices(self.positions, self.faces[a])
				.map(Triangle3::area)
				.unwrap_or(0.0);
			let ba = Triangle3::from_indices(self.positions, self.faces[b])
				.map(Triangle3::area)
				.unwrap_or(0.0);
			ba.partial_cmp(&aa).unwrap_or(std::cmp::Ordering::Equal)
		});

		let mut charts = Vec::new();
		for seed in order {
			if assigned[seed] {
				continue;
			}
			charts.push(self.grow_one(seed, &mut assigned));
		}
		charts
	}

	fn grow_one(&self, seed: usize, assigned: &mut [bool]) -> Chart {
		let seed_normal = self.normals[seed];
		let (tangent, bitangent) = basis_from_normal(seed_normal);
		let mut faces = vec![seed];
		let mut projected = vec![self.project_face(seed, tangent, bitangent)];
		assigned[seed] = true;
		let mut queue = VecDeque::from([seed]);
		let cos_limit = MAX_SEED_ANGLE.cos();

		while let Some(face) = queue.pop_front() {
			for &neighbor in &self.adjacency[face] {
				if assigned[neighbor] {
					continue;
				}
				if self.normals[neighbor].dot(seed_normal) < cos_limit {
					continue;
				}
				let proj = self.project_face(neighbor, tangent, bitangent);
				if projected.iter().any(|existing| existing.overlaps_interior(proj)) {
					continue;
				}
				assigned[neighbor] = true;
				faces.push(neighbor);
				projected.push(proj);
				queue.push_back(neighbor);
			}
		}
		Chart { face_indices: faces, seed_normal }
	}

	fn project_face(&self, face: usize, tangent: Vec3, bitangent: Vec3) -> Triangle2 {
		let tri = Triangle3::from_indices(self.positions, self.faces[face]).unwrap_or(Triangle3 {
			a: Vec3::ZERO,
			b: Vec3::X,
			c: Vec3::Y,
		});
		Triangle2 {
			a: Vec2::new(tri.a.dot(tangent), tri.a.dot(bitangent)),
			b: Vec2::new(tri.b.dot(tangent), tri.b.dot(bitangent)),
			c: Vec2::new(tri.c.dot(tangent), tri.c.dot(bitangent)),
		}
	}
}

struct ProjectedChart {
	/// `(original vertex index, local 2D)` after rotation into a tight AABB.
	local: Vec<(u32, Vec2)>,
	size: Vec2,
}

impl ProjectedChart {
	fn from_chart(positions: &[Vec3], paintable: &[[u32; 3]], chart: &Chart) -> Self {
		let mut used: Vec<u32> = Vec::new();
		for &face_i in &chart.face_indices {
			used.extend_from_slice(&paintable[face_i]);
		}
		used.sort_unstable();
		used.dedup();
		let (tangent, bitangent) = basis_from_normal(chart.seed_normal);
		let mut raw: Vec<(u32, Vec2)> = used
			.into_iter()
			.map(|orig| {
				let p = positions[orig as usize];
				(orig, Vec2::new(p.dot(tangent), p.dot(bitangent)))
			})
			.collect();
		let angle = best_aabb_angle(raw.iter().map(|(_, uv)| *uv));
		let (sin, cos) = angle.sin_cos();
		for (_, uv) in &mut raw {
			*uv = Vec2::new(uv.x * cos - uv.y * sin, uv.x * sin + uv.y * cos);
		}
		let mut min = Vec2::splat(f32::MAX);
		let mut max = Vec2::splat(f32::MIN);
		for (_, uv) in &raw {
			min = min.min(*uv);
			max = max.max(*uv);
		}
		let mut size = (max - min).max(Vec2::splat(EPS));
		if size.x < size.y * MIN_CHART_ASPECT {
			size.x = size.y * MIN_CHART_ASPECT;
		}
		if size.y < size.x * MIN_CHART_ASPECT {
			size.y = size.x * MIN_CHART_ASPECT;
		}
		let local = raw.into_iter().map(|(orig, uv)| (orig, uv - min)).collect();
		Self { local, size }
	}
}

fn split_collapsed_faces(
	positions: &[Vec3],
	paintable: &[[u32; 3]],
	charts: &mut Vec<Chart>,
	projected: &mut Vec<ProjectedChart>,
) {
	let position_scale = aabb_extent(positions);
	let mut extras: Vec<(Chart, ProjectedChart)> = Vec::new();
	for (chart, proj) in charts.iter_mut().zip(projected.iter_mut()) {
		if chart.face_indices.len() <= 1 {
			continue;
		}
		let lookup: HashMap<u32, Vec2> = proj.local.iter().copied().collect();
		let mut keep = Vec::new();
		let mut moved = Vec::new();
		for &face_i in &chart.face_indices {
			let face = paintable[face_i];
			let uv = match (
				lookup.get(&face[0]).copied(),
				lookup.get(&face[1]).copied(),
				lookup.get(&face[2]).copied(),
			) {
				(Some(a), Some(b), Some(c)) => Triangle2 { a, b, c },
				_ => {
					moved.push(face_i);
					continue;
				}
			};
			let Some(geom) = Triangle3::from_indices(positions, face) else {
				moved.push(face_i);
				continue;
			};
			let geom_ok = !geom.is_degenerate(position_scale);
			let local_density = proj.size.x.max(proj.size.y).max(1e-6);
			let median = 1.0 / (local_density * local_density);
			if geom_ok && uv.is_projection_collapsed(geom, median) {
				moved.push(face_i);
			} else {
				keep.push(face_i);
			}
		}
		if moved.is_empty() {
			continue;
		}
		chart.face_indices = keep;
		if !chart.face_indices.is_empty() {
			*proj = ProjectedChart::from_chart(positions, paintable, chart);
		}
		for face_i in moved {
			let face = paintable[face_i];
			let seed_normal = Triangle3::from_indices(positions, face)
				.and_then(Triangle3::normal)
				.unwrap_or(Vec3::Y);
			let extra = Chart { face_indices: vec![face_i], seed_normal };
			let extra_proj = ProjectedChart::from_chart(positions, paintable, &extra);
			extras.push((extra, extra_proj));
		}
	}
	let mut kept_charts = Vec::new();
	let mut kept_proj = Vec::new();
	for (chart, proj) in charts.drain(..).zip(projected.drain(..)) {
		if chart.face_indices.is_empty() {
			continue;
		}
		kept_charts.push(chart);
		kept_proj.push(proj);
	}
	kept_charts.extend(
		extras.iter().map(|(c, _)| Chart {
			face_indices: c.face_indices.clone(),
			seed_normal: c.seed_normal,
		}),
	);
	kept_proj.extend(extras.into_iter().map(|(_, p)| p));
	*charts = kept_charts;
	*projected = kept_proj;
}

struct PackedChart {
	offset: Vec2,
	size: Vec2,
}

impl PackedChart {
	fn uv(&self, local: Vec2, local_size: Vec2) -> Vec2 {
		let nx = if local_size.x > EPS { (local.x / local_size.x).clamp(0.0, 1.0) } else { 0.5 };
		let ny = if local_size.y > EPS { (local.y / local_size.y).clamp(0.0, 1.0) } else { 0.5 };
		Vec2::new(self.offset.x + nx * self.size.x, self.offset.y + ny * self.size.y)
	}
}

struct ChartPacker;

impl ChartPacker {
	fn pack(charts: &[ProjectedChart]) -> Vec<PackedChart> {
		if charts.is_empty() {
			return Vec::new();
		}
		let sizes: Vec<Vec2> = charts.iter().map(|c| c.size).collect();
		let mut lo = 1e-6_f32;
		let mut hi = 1.0_f32;
		for _ in 0..24 {
			let mid = (lo + hi) * 0.5;
			if Self::try_pack(&sizes, mid).is_some() {
				lo = mid;
			} else {
				hi = mid;
			}
		}
		let scale = lo;
		let offsets = Self::try_pack(&sizes, scale).unwrap_or_else(|| {
			sizes
				.iter()
				.enumerate()
				.map(|(i, _)| Vec2::new(UNIT_BORDER, UNIT_BORDER + i as f32 * 0.0))
				.collect()
		});
		offsets
			.into_iter()
			.zip(&sizes)
			.map(|(offset, size)| PackedChart { offset, size: *size * scale })
			.collect()
	}

	fn try_pack(sizes: &[Vec2], scale: f32) -> Option<Vec<Vec2>> {
		let inner = 1.0 - 2.0 * UNIT_BORDER;
		if inner <= 0.0 {
			return None;
		}
		let mut order: Vec<usize> = (0..sizes.len()).collect();
		order.sort_by(|&a, &b| {
			(sizes[b].y * scale)
				.partial_cmp(&(sizes[a].y * scale))
				.unwrap_or(std::cmp::Ordering::Equal)
		});
		let mut offsets = vec![Vec2::ZERO; sizes.len()];
		let mut x = UNIT_BORDER;
		let mut y = UNIT_BORDER;
		let mut shelf_h = 0.0_f32;
		for i in order {
			let w = sizes[i].x * scale;
			let h = sizes[i].y * scale;
			if w > inner + EPS || h > inner + EPS {
				return None;
			}
			if x > UNIT_BORDER && x + w > 1.0 - UNIT_BORDER + EPS {
				y += shelf_h + ISLAND_GAP;
				x = UNIT_BORDER;
				shelf_h = 0.0;
			}
			if y + h > 1.0 - UNIT_BORDER + EPS {
				return None;
			}
			offsets[i] = Vec2::new(x, y);
			x += w + ISLAND_GAP;
			shelf_h = shelf_h.max(h);
		}
		Some(offsets)
	}
}

pub fn aabb_extent(positions: &[Vec3]) -> f32 {
	let Some(first) = positions.first().copied() else {
		return 1.0;
	};
	let mut min = first;
	let mut max = first;
	for &p in positions.iter().skip(1) {
		min = min.min(p);
		max = max.max(p);
	}
	(max - min).max_element().max(1e-6)
}

fn face_adjacency(positions: &[Vec3], faces: &[[u32; 3]]) -> Vec<Vec<usize>> {
	let mut edge_faces: HashMap<PosEdge, Vec<usize>> = HashMap::new();
	for (i, face) in faces.iter().enumerate() {
		for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
			if let Some(key) = pos_edge_key(positions, a, b) {
				edge_faces.entry(key).or_default().push(i);
			}
		}
	}
	let mut adj = vec![Vec::new(); faces.len()];
	for faces_on_edge in edge_faces.values() {
		for (i, &a) in faces_on_edge.iter().enumerate() {
			for &b in faces_on_edge.iter().skip(i + 1) {
				adj[a].push(b);
				adj[b].push(a);
			}
		}
	}
	adj
}

type PosEdge = ((i32, i32, i32), (i32, i32, i32));

fn quantize_pos(p: Vec3) -> (i32, i32, i32) {
	const SCALE: f32 = 1e4;
	((p.x * SCALE).round() as i32, (p.y * SCALE).round() as i32, (p.z * SCALE).round() as i32)
}

fn pos_edge_key(positions: &[Vec3], a: u32, b: u32) -> Option<PosEdge> {
	let pa = *positions.get(a as usize)?;
	let pb = *positions.get(b as usize)?;
	let qa = quantize_pos(pa);
	let qb = quantize_pos(pb);
	if qa == qb {
		return None;
	}
	Some(if qa <= qb { (qa, qb) } else { (qb, qa) })
}

fn basis_from_normal(normal: Vec3) -> (Vec3, Vec3) {
	let n = normal.normalize_or(Vec3::Y);
	let helper = if n.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
	let tangent = n.cross(helper).normalize_or(Vec3::X);
	let bitangent = n.cross(tangent).normalize_or(Vec3::Z);
	(tangent, bitangent)
}

fn best_aabb_angle(points: impl Iterator<Item = Vec2>) -> f32 {
	let pts: Vec<Vec2> = points.collect();
	if pts.len() < 2 {
		return 0.0;
	}
	let mut best_angle = 0.0;
	let mut best_area = f32::MAX;
	for step in 0..18 {
		let angle = step as f32 * (PI / 18.0);
		let (sin, cos) = angle.sin_cos();
		let mut min = Vec2::splat(f32::MAX);
		let mut max = Vec2::splat(f32::MIN);
		for p in &pts {
			let r = Vec2::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos);
			min = min.min(r);
			max = max.max(r);
		}
		let area = (max.x - min.x).max(0.0) * (max.y - min.y).max(0.0);
		if area < best_area {
			best_area = area;
			best_angle = angle;
		}
	}
	best_angle
}

fn point_strictly_inside(p: Vec2, tri: Triangle2) -> bool {
	let area = (tri.b - tri.a).perp_dot(tri.c - tri.a);
	if area.abs() < 1e-16 {
		return false;
	}
	let w0 = (tri.b - p).perp_dot(tri.c - p) / area;
	let w1 = (tri.c - p).perp_dot(tri.a - p) / area;
	let w2 = (tri.a - p).perp_dot(tri.b - p) / area;
	let eps = 1e-5;
	w0 > eps && w1 > eps && w2 > eps
}

fn edges_cross_interior(a: Triangle2, b: Triangle2) -> bool {
	let ae = [(a.a, a.b), (a.b, a.c), (a.c, a.a)];
	let be = [(b.a, b.b), (b.b, b.c), (b.c, b.a)];
	for (p, q) in ae {
		for (r, s) in be {
			if segments_cross_interior(p, q, r, s) {
				return true;
			}
		}
	}
	false
}

fn segments_cross_interior(p: Vec2, q: Vec2, r: Vec2, s: Vec2) -> bool {
	let d1 = q - p;
	let d2 = s - r;
	let denom = d1.perp_dot(d2);
	if denom.abs() < 1e-16 {
		return false;
	}
	let t = (r - p).perp_dot(d2) / denom;
	let u = (r - p).perp_dot(d1) / denom;
	t > 1e-4 && t < 1.0 - 1e-4 && u > 1e-4 && u < 1.0 - 1e-4
}

#[cfg(test)]
mod tests {
	use super::*;

	fn box_mesh() -> (Vec<Vec3>, Vec<Vec3>, Vec<[u32; 3]>) {
		let p = [
			Vec3::new(-1.0, -1.0, -1.0),
			Vec3::new(1.0, -1.0, -1.0),
			Vec3::new(1.0, 1.0, -1.0),
			Vec3::new(-1.0, 1.0, -1.0),
			Vec3::new(-1.0, -1.0, 1.0),
			Vec3::new(1.0, -1.0, 1.0),
			Vec3::new(1.0, 1.0, 1.0),
			Vec3::new(-1.0, 1.0, 1.0),
		];
		let faces = vec![
			[0, 1, 2],
			[0, 2, 3],
			[4, 6, 5],
			[4, 7, 6],
			[0, 4, 5],
			[0, 5, 1],
			[3, 2, 6],
			[3, 6, 7],
			[0, 3, 7],
			[0, 7, 4],
			[1, 5, 6],
			[1, 6, 2],
		];
		let normals: Vec<Vec3> = p.iter().map(|v| v.normalize_or(Vec3::Y)).collect();
		(p.to_vec(), normals, faces)
	}

	fn planar_grid() -> (Vec<Vec3>, Vec<Vec3>, Vec<[u32; 3]>) {
		let mut positions = Vec::new();
		for y in 0..3 {
			for x in 0..3 {
				positions.push(Vec3::new(x as f32, y as f32, 0.0));
			}
		}
		let mut faces = Vec::new();
		for y in 0..2 {
			for x in 0..2 {
				let i = y * 3 + x;
				faces.push([i, i + 1, i + 3]);
				faces.push([i + 1, i + 4, i + 3]);
			}
		}
		let normals = vec![Vec3::Z; positions.len()];
		(positions, normals, faces)
	}

	#[test]
	fn box_unwrap_keeps_connected_charts_not_one_per_triangle() -> anyhow::Result<()> {
		let (positions, normals, faces) = box_mesh();
		let unwrapped = UnwrappedMesh::from_indexed(&positions, &normals, &faces);
		if unwrapped.faces.len() != 12 {
			return Err(anyhow::anyhow!("expected 12 faces, got {}", unwrapped.faces.len()));
		}
		if !(2..=8).contains(&unwrapped.chart_count) {
			return Err(anyhow::anyhow!(
				"box should unwrap into a handful of side charts, got {}",
				unwrapped.chart_count
			));
		}
		Ok(())
	}

	#[test]
	fn planar_grid_is_one_chart() -> anyhow::Result<()> {
		let (positions, normals, faces) = planar_grid();
		let unwrapped = UnwrappedMesh::from_indexed(&positions, &normals, &faces);
		if unwrapped.chart_count != 1 {
			return Err(anyhow::anyhow!("expected 1 chart, got {}", unwrapped.chart_count));
		}
		Ok(())
	}

	#[test]
	fn exploded_planar_grid_still_one_chart() -> anyhow::Result<()> {
		let (shared, normals, faces) = planar_grid();
		let mut positions = Vec::new();
		let mut exploded_faces = Vec::new();
		let mut exploded_normals = Vec::new();
		for face in faces {
			let i = positions.len() as u32;
			for &idx in &face {
				positions.push(shared[idx as usize]);
				exploded_normals.push(normals[idx as usize]);
			}
			exploded_faces.push([i, i + 1, i + 2]);
		}
		let unwrapped = UnwrappedMesh::from_indexed(&positions, &exploded_normals, &exploded_faces);
		if unwrapped.chart_count != 1 {
			return Err(anyhow::anyhow!(
				"exploded grid must still chart by position, got {}",
				unwrapped.chart_count
			));
		}
		Ok(())
	}

	#[test]
	fn unwrapped_faces_have_uv_area_and_stay_in_unit_square() -> anyhow::Result<()> {
		let (positions, normals, faces) = box_mesh();
		let unwrapped = UnwrappedMesh::from_indexed(&positions, &normals, &faces);
		let pos_scale = aabb_extent(&unwrapped.positions);
		let mut densities = Vec::new();
		for face in &unwrapped.faces {
			let Some(geom) = Triangle3::from_indices(&unwrapped.positions, *face) else {
				continue;
			};
			let Some(uv) = Triangle2::from_indices(&unwrapped.uvs, *face) else {
				continue;
			};
			if !geom.is_degenerate(pos_scale) {
				densities.push(uv.area() / geom.area().max(1e-20));
			}
		}
		densities.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
		let median = densities.get(densities.len() / 2).copied().unwrap_or(1.0);
		for face in &unwrapped.faces {
			let geom = Triangle3::from_indices(&unwrapped.positions, *face)
				.ok_or_else(|| anyhow::anyhow!("missing geom"))?;
			let uv = Triangle2::from_indices(&unwrapped.uvs, *face)
				.ok_or_else(|| anyhow::anyhow!("missing uv"))?;
			if geom.is_degenerate(pos_scale) {
				continue;
			}
			if uv.is_projection_collapsed(geom, median) {
				return Err(anyhow::anyhow!("collapsed UV on nondegenerate face {face:?}"));
			}
		}
		let island = UvIsland::from_uvs(&unwrapped.uvs).ok_or_else(|| anyhow::anyhow!("no uvs"))?;
		if !island.in_unit_square() {
			return Err(anyhow::anyhow!("island left 0..1: {island:?}"));
		}
		if island.min.x < UNIT_BORDER - 1e-3 || island.min.y < UNIT_BORDER - 1e-3 {
			return Err(anyhow::anyhow!("missing border padding: {island:?}"));
		}
		Ok(())
	}

	#[test]
	fn adjacent_uv_triangles_are_not_interior_overlaps() -> anyhow::Result<()> {
		let a = Triangle2 { a: Vec2::ZERO, b: Vec2::X, c: Vec2::Y };
		let b = Triangle2 { a: Vec2::X, b: Vec2::ONE, c: Vec2::Y };
		if a.overlaps_interior(b) {
			return Err(anyhow::anyhow!("shared-edge triangles must not count as overlap"));
		}
		let overlap =
			Triangle2 { a: Vec2::new(0.2, 0.2), b: Vec2::new(0.8, 0.2), c: Vec2::new(0.2, 0.8) };
		if !a.overlaps_interior(overlap) {
			return Err(anyhow::anyhow!("crossing triangles must count as overlap"));
		}
		Ok(())
	}

	#[test]
	fn scale_aware_collapse_ignores_true_degenerate_geometry() -> anyhow::Result<()> {
		let slim =
			Triangle3 { a: Vec3::ZERO, b: Vec3::new(2.0, 0.0, 0.0), c: Vec3::new(1.0, 1e-16, 0.0) };
		if !slim.is_degenerate(2.0) {
			return Err(anyhow::anyhow!("expected degenerate geom"));
		}
		let uv = Triangle2 { a: Vec2::ZERO, b: Vec2::ZERO, c: Vec2::ZERO };
		if !uv.is_collapsed(1.0) {
			return Err(anyhow::anyhow!("zero UV should be collapsed"));
		}
		Ok(())
	}
}
