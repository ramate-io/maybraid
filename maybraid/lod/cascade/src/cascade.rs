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
		Self {
			edge_multiple_log2,
			radius,
		}
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
		Self {
			leaf_scale,
			rings,
			grid,
		}
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
		Some(
			self.span_max_axis() * 2_f32.powi(cfg.edge_multiple_log2() as i32),
		)
	}

	/// Cheap leaf-cell recentring test (RFC §3.1.3).
	pub fn needs_leaf_recenter(&self, focal_prev: Vec3, focal_new: Vec3) -> bool {
		self.leaf_origin(focal_prev) != self.leaf_origin(focal_new)
	}

	/// Full cascade footprints \(\mathcal W_{\mathrm{cascade}}\) at `focal`.
	pub fn cascade_footprints(&self, focal: Vec3) -> HashSet<Chunk> {
		let mut out = HashSet::new();
		let leaf_origin = self.leaf_origin(focal);
		out.insert(Chunk::from_min_max(
			leaf_origin,
			leaf_origin + self.leaf_scale,
			None,
		));

		let mut anchor = leaf_origin - self.leaf_scale;
		for k in 0..self.rings {
			let extent = self.ring_cell_extent(k);
			out.extend(hollow_shell(anchor, extent));
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

		let anchor = Vec3::new(
			(focal.x / g).floor() * g,
			g * -0.5,
			(focal.z / g).floor() * g,
		);

		let mut out = HashSet::new();
		for xi in -(rx as i32)..=(rx as i32) {
			for yi in -(ry as i32)..=(ry as i32) {
				for zi in -(rz as i32)..=(rz as i32) {
					let corner =
						anchor + Vec3::new(xi as f32 * g, yi as f32 * g, zi as f32 * g);
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

	/// Candidate chunks for overlap queries when an entity’s bounds move (RFC §3.4.4).
	pub fn all_possible_new_chunks(
		&self,
		previous: Option<Aabb3d>,
		current: Aabb3d,
	) -> Vec<Chunk> {
		let mut u = self.work_set_at_bounds(&current);
		if let Some(p) = previous {
			u.extend(self.work_set_at_bounds(&p));
		}
		u.into_iter().collect()
	}
}

fn hollow_shell(anchor: Vec3, extent: Vec3) -> impl Iterator<Item = Chunk> {
	(0_u32..3).flat_map(move |x| {
		(0_u32..3).flat_map(move |y| {
			(0_u32..3).filter_map(move |z| {
				if x == 1 && y == 1 && z == 1 {
					return None;
				}
				let corner = anchor
					+ Vec3::new(
						x as f32 * extent.x,
						y as f32 * extent.y,
						z as f32 * extent.z,
					);
				Some(Chunk::from_min_max(corner, corner + extent, None))
			})
		})
	})
}
