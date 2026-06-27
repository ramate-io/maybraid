use std::collections::HashSet;

use bevy_math::bounding::{Aabb3d, BoundingVolume};
use bevy_math::Vec3;

use crate::chunk::Chunk;

/// Optional coarse outer grid (RFC §3.1.2).
///
/// Tile edge \(G\) should satisfy \(G \geq \max(\sigma_x,\sigma_y,\sigma_z)\) where \(\boldsymbol\sigma\) is [`Cascade::hull_extent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridConfig {
	edge_multiple_log2: u8,
	radius: [u32; 3],
}

impl GridConfig {
	pub fn new(edge_multiple_log2: u8, radius: [u32; 3]) -> Self {
		Self { edge_multiple_log2, radius }
	}

	#[inline]
	pub fn edge_multiple_log2(&self) -> u8 {
		self.edge_multiple_log2
	}

	#[inline]
	pub fn set_edge_multiple_log2(&mut self, value: u8) {
		self.edge_multiple_log2 = value;
	}

	#[inline]
	pub fn radius(&self) -> [u32; 3] {
		self.radius
	}

	#[inline]
	pub fn set_radius(&mut self, radius: [u32; 3]) {
		self.radius = radius;
	}
}

/// Pure-geometry cascade: nested hollow shells plus optional skirt grid (RFC §3.1).
///
/// This matches the layout from Maybraid’s exploratory cascade (`util/chunk`) while dropping resolution tags.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cascade {
	leaf_scale: Vec3,
	rings: u8,
	grid: Option<GridConfig>,
}

impl Cascade {
	pub fn new(leaf_scale: Vec3, rings: u8, grid: Option<GridConfig>) -> Self {
		Self { leaf_scale, rings, grid }
	}

	/// Finest lattice spacing \(\mathbf s_0\) along each world axis.
	#[inline]
	pub fn leaf_scale(&self) -> Vec3 {
		self.leaf_scale
	}

	#[inline]
	pub fn set_leaf_scale(&mut self, leaf_scale: Vec3) {
		self.leaf_scale = leaf_scale;
	}

	/// Number of hollow shells \(K\) at extents \(\mathbf s_0\,3^k\) for \(k = 0 .. K-1\) (per axis).
	#[inline]
	pub fn rings(&self) -> u8 {
		self.rings
	}

	#[inline]
	pub fn set_rings(&mut self, rings: u8) {
		self.rings = rings;
	}

	#[inline]
	pub fn ring_count(&self) -> u8 {
		self.rings
	}

	#[inline]
	pub fn grid(&self) -> Option<GridConfig> {
		self.grid
	}

	#[inline]
	pub fn set_grid(&mut self, grid: Option<GridConfig>) {
		self.grid = grid;
	}

	#[inline]
	pub fn grid_config_mut(&mut self) -> Option<&mut GridConfig> {
		self.grid.as_mut()
	}

	/// Hollow \(3 \times 3 \times 3\) shell at `anchor`: 26 axis-aligned cells sharing extent `cell_extent`, omitting the center cell.
	pub fn hollow_shell(anchor: Vec3, cell_extent: Vec3) -> impl Iterator<Item = Chunk> {
		(0_u32..3).flat_map(move |x| {
			(0_u32..3).flat_map(move |y| {
				(0_u32..3).filter_map(move |z| {
					if x == 1 && y == 1 && z == 1 {
						return None;
					}
					let corner = anchor
						+ Vec3::new(
							x as f32 * cell_extent.x,
							y as f32 * cell_extent.y,
							z as f32 * cell_extent.z,
						);
					Some(Chunk::from_min_max(corner, corner + cell_extent, None))
				})
			})
		})
	}

	/// Ring shell cell extent \(\mathbf s_k = \mathbf s_0\,3^k\) per axis.
	#[inline]
	pub fn ring_cell_extent(&self, ring: u8) -> Vec3 {
		let m = 3_u32.pow(ring as u32) as f32;
		self.leaf_scale * m
	}

	#[inline]
	pub fn leaf_origin(&self, focal: Vec3) -> Vec3 {
		let s = self.leaf_scale;
		(focal / s).floor() * s
	}

	/// Hull extent \(\boldsymbol\sigma = \mathbf s_0\,3^{K}\) per axis (edges of the solid hull box).
	#[inline]
	pub fn hull_extent(&self) -> Vec3 {
		let m = 3_u32.pow(self.rings as u32) as f32;
		self.leaf_scale * m
	}

	#[inline]
	pub fn span_max_axis(&self) -> f32 {
		let e = self.hull_extent();
		e.x.max(e.y).max(e.z)
	}

	/// Minimum corner of the solid hull box \(H(\mathbf p)\) at focal \(\mathbf p\).
	pub fn hull_lower_corner(&self, focal: Vec3) -> Vec3 {
		let mut p = self.leaf_origin(focal);
		for k in 0..self.rings {
			p -= self.ring_cell_extent(k);
		}
		p
	}

	pub fn hull(&self, focal: Vec3) -> Aabb3d {
		let ll = self.hull_lower_corner(focal);
		Aabb3d::from_min_max(ll, ll + self.hull_extent())
	}

	/// Scalar grid tile edge \(G = \max(\boldsymbol\sigma)\,2^{\mathrm{mult}}\) when a grid is configured.
	pub fn grid_chunk_edge(&self) -> Option<f32> {
		let cfg = self.grid?;
		Some(self.span_max_axis() * 2_f32.powi(cfg.edge_multiple_log2() as i32))
	}

	/// Cheap leaf-cell recentring test (RFC §3.1.3).
	pub fn needs_leaf_recenter(&self, focal_prev: Vec3, focal_new: Vec3) -> bool {
		self.leaf_origin(focal_prev) != self.leaf_origin(focal_new)
	}

	/// Full cascade footprints \(\mathcal W_{\mathrm{cascade}}\) at `focal`.
	pub fn cascade_footprints(&self, focal: Vec3) -> HashSet<Chunk> {
		let mut out = HashSet::new();
		let leaf_origin = self.leaf_origin(focal);
		out.insert(Chunk::from_min_max(leaf_origin, leaf_origin + self.leaf_scale, None));

		let mut anchor = leaf_origin - self.leaf_scale;
		for k in 0..self.rings {
			let extent = self.ring_cell_extent(k);
			out.extend(Self::hollow_shell(anchor, extent));
			anchor -= self.ring_cell_extent(k + 1);
		}
		out
	}

	/// Grid footprints \(\mathcal W_{\mathrm{grid}}\), each omitting the hull \(H(\mathbf p)\).
	pub fn grid_footprints(&self, focal: Vec3) -> HashSet<Chunk> {
		let Some(cfg) = self.grid else {
			return HashSet::new();
		};
		let g = self.span_max_axis() * 2_f32.powi(cfg.edge_multiple_log2() as i32);
		let omit_hull = self.hull(focal);
		let [rx, ry, rz] = cfg.radius();

		let anchor = Vec3::new((focal.x / g).floor() * g, g * -0.5, (focal.z / g).floor() * g);

		let mut out = HashSet::new();
		for xi in -(rx as i32)..=(rx as i32) {
			for yi in -(ry as i32)..=(ry as i32) {
				for zi in -(rz as i32)..=(rz as i32) {
					let corner = anchor + Vec3::new(xi as f32 * g, yi as f32 * g, zi as f32 * g);
					out.insert(Chunk::cube(corner, g, Some(omit_hull)));
				}
			}
		}
		out
	}

	/// \(\mathcal W(\mathbf p) = \mathcal W_{\mathrm{cascade}} \cup \mathcal W_{\mathrm{grid}}\) using focal \(\mathbf p\).
	pub fn work_set_at_focal(&self, focal: Vec3) -> HashSet<Chunk> {
		let mut w = self.cascade_footprints(focal);
		w.extend(self.grid_footprints(focal));
		w
	}

	fn work_set_at_bounds(&self, world_bounds: &Aabb3d) -> HashSet<Chunk> {
		let focal = Vec3::from(world_bounds.center());
		self.work_set_at_focal(focal)
	}

	/// Footprints newly covered when bounds move from `previous` to `current` (set difference on both cascade and grid parts).
	///
	/// Uses [`Aabb3d::center`] as the cascade focal for each snapshot (`CascadePosition` carries `AaBb` in RFC §3.2).
	pub fn new_chunks(&self, previous: Option<Aabb3d>, current: Aabb3d) -> Vec<Chunk> {
		let curr = self.work_set_at_bounds(&current);
		match previous {
			None => curr.into_iter().collect(),
			Some(prev) => {
				let prev_set = self.work_set_at_bounds(&prev);
				curr.difference(&prev_set).copied().collect()
			}
		}
	}

	/// Footprints that left the active set when bounds moved from `previous` to `current`:
	/// \(\mathcal W(\mathrm{prev}) \setminus \mathcal W(\mathrm{current})\).
	pub fn expired_chunks(&self, previous: Option<Aabb3d>, current: Aabb3d) -> Vec<Chunk> {
		match previous {
			None => Vec::new(),
			Some(prev) => {
				let prev_set = self.work_set_at_bounds(&prev);
				let curr_set = self.work_set_at_bounds(&current);
				prev_set.difference(&curr_set).copied().collect()
			}
		}
	}

	/// Candidate chunks for overlap queries when an entity’s bounds move (RFC §3.4.4).
	pub fn all_possible_new_chunks(&self, previous: Option<Aabb3d>, current: Aabb3d) -> Vec<Chunk> {
		let mut u = self.work_set_at_bounds(&current);
		if let Some(p) = previous {
			u.extend(self.work_set_at_bounds(&p));
		}
		u.into_iter().collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::hash_map::DefaultHasher;
	use std::collections::BTreeSet;
	use std::hash::{Hash, Hasher};

	fn cubic(s: f32, rings: u8, grid: Option<GridConfig>) -> Cascade {
		Cascade::new(Vec3::splat(s), rings, grid)
	}

	fn cube_bb(center: Vec3, half: f32) -> Aabb3d {
		Aabb3d::from_min_max(center - Vec3::splat(half), center + Vec3::splat(half))
	}

	#[test]
	fn one_ring_is_leaf_plus_hollow_shell() {
		let c = cubic(1.0, 1, None);
		let p = Vec3::ZERO;
		let leaf = Chunk::cube(Vec3::ZERO, 1.0, None);
		let anchor = Vec3::splat(-1.0);
		let mut expected = HashSet::new();
		expected.insert(leaf);
		expected.extend(Cascade::hollow_shell(anchor, Vec3::ONE));
		let actual = c.cascade_footprints(p);
		assert_eq!(actual.len(), 27);
		assert_eq!(actual, expected);
	}

	#[test]
	fn two_rings_shell_anchors_match_legacy_chunk_tests() {
		let c = cubic(1.0, 2, None);
		let mut expected = HashSet::new();
		expected.insert(Chunk::cube(Vec3::ZERO, 1.0, None));
		expected.extend(Cascade::hollow_shell(Vec3::splat(-1.0), Vec3::ONE));
		expected.extend(Cascade::hollow_shell(Vec3::new(-4.0, -4.0, -4.0), Vec3::splat(3.0)));
		let actual = c.cascade_footprints(Vec3::ZERO);
		assert_eq!(actual.len(), 53);
		assert_eq!(actual, expected);
	}

	#[test]
	fn ring_sizes_other_than_one() {
		for &(scale, rings) in &[(2.5_f32, 1_u8), (0.5_f32, 1_u8)] {
			let c = cubic(scale, rings, None);
			let leaf = Chunk::cube(Vec3::ZERO, scale, None);
			let anchor = Vec3::splat(-scale);
			let ext = Vec3::splat(scale);
			let mut expected = HashSet::new();
			expected.insert(leaf);
			expected.extend(Cascade::hollow_shell(anchor, ext));
			let actual = c.cascade_footprints(Vec3::ZERO);
			assert_eq!(actual.len(), 27, "scale={scale}");
			assert_eq!(actual, expected);
		}
	}

	#[test]
	fn grid_tiles_carry_hull_omission_and_cover_focal() {
		let grid = Some(GridConfig::new(0, [0, 0, 0]));
		let c = cubic(1.0, 1, grid);
		let focal = Vec3::ZERO;
		let hull = c.hull(focal);
		let tiles = c.grid_footprints(focal);
		assert_eq!(tiles.len(), 1);
		let tile = tiles.into_iter().next().unwrap();
		assert_eq!(tile.omit(), Some(hull));
		let g = c.grid_chunk_edge().unwrap();
		assert!((tile.max_extent_component() - g).abs() < 1e-5);
	}

	#[test]
	fn grid_tile_overlap_outside_hull_is_positive() {
		let c = cubic(1.0, 1, Some(GridConfig::new(0, [1, 1, 1])));
		let focal = Vec3::ZERO;
		let hull = c.hull(focal);
		let hx = hull.max.x;
		let skirt_tile = c
			.grid_footprints(focal)
			.into_iter()
			.find(|ch| ch.bounds_min().x >= hx - 1e-5)
			.expect("expected a grid tile beyond +x hull face");
		let center = (skirt_tile.bounds_min() + skirt_tile.bounds_max()) * 0.5;
		let query = cube_bb(center, 0.25);
		assert!(
			skirt_tile.overlap_volume(&query) > 0.0,
			"skirt footprint should overlap an interior probe away from the hull hole"
		);
	}

	#[test]
	fn new_and_expired_are_disjoint_and_cover_set_difference() {
		let c = cubic(1.0, 1, None);
		let prev = cube_bb(Vec3::new(0.25, 0.0, 0.0), 0.05);
		let curr = cube_bb(Vec3::new(2.5, 0.0, 0.0), 0.05);
		let new_set: HashSet<_> = c.new_chunks(Some(prev), curr).into_iter().collect();
		let expired_set: HashSet<_> = c.expired_chunks(Some(prev), curr).into_iter().collect();
		assert!(new_set.is_disjoint(&expired_set));
		let wp = c.work_set_at_focal(Vec3::from(prev.center()));
		let wc = c.work_set_at_focal(Vec3::from(curr.center()));
		let diff_new: HashSet<_> = wc.difference(&wp).copied().collect();
		let diff_exp: HashSet<_> = wp.difference(&wc).copied().collect();
		assert_eq!(new_set, diff_new);
		assert_eq!(expired_set, diff_exp);
	}

	#[test]
	fn new_chunks_empty_when_focal_unchanged_across_snapshots() {
		let c = cubic(1.0, 1, None);
		let bb = cube_bb(Vec3::ZERO, 0.25);
		let delta = c.new_chunks(Some(bb), bb);
		assert!(delta.is_empty(), "same work set should yield no new footprints");
	}

	#[test]
	fn new_chunks_nonempty_when_leaf_cell_changes() {
		let c = cubic(1.0, 1, None);
		let prev = cube_bb(Vec3::new(0.25, 0.0, 0.0), 0.05);
		let curr = cube_bb(Vec3::new(2.5, 0.0, 0.0), 0.05);
		let delta = c.new_chunks(Some(prev), curr);
		assert!(!delta.is_empty(), "crossing a leaf boundary should introduce new chunks");
	}

	#[test]
	fn all_possible_new_chunks_unions_work_sets() {
		let c = cubic(1.0, 1, None);
		let a = cube_bb(Vec3::ZERO, 0.2);
		let b = cube_bb(Vec3::new(5.0, 0.0, 0.0), 0.2);
		let union_keys: HashSet<Chunk> =
			c.all_possible_new_chunks(Some(a), b).into_iter().collect();
		let wa = c.work_set_at_focal(a.center().into());
		let wb = c.work_set_at_focal(b.center().into());
		let mut manual = wa;
		manual.extend(wb);
		assert_eq!(union_keys, manual);
	}

	#[test]
	fn chunk_hash_stable_for_identical_geometry() {
		let ch = Chunk::cube(Vec3::new(1.0, -2.0, 3.0), 4.0, None);
		let mut h1 = DefaultHasher::new();
		let mut h2 = DefaultHasher::new();
		ch.hash(&mut h1);
		ch.hash(&mut h2);
		assert_eq!(h1.finish(), h2.finish());
	}

	#[test]
	fn chunk_sort_order_deterministic() {
		let c = cubic(1.0, 1, None);
		let set = c.cascade_footprints(Vec3::ZERO);
		let v1: Vec<_> = set.iter().copied().collect::<BTreeSet<_>>().into_iter().collect();
		let v2: Vec<_> = set.iter().copied().collect::<BTreeSet<_>>().into_iter().collect();
		assert_eq!(v1, v2);
	}
}
