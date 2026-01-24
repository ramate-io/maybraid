use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;

pub trait ResolutionMap: Debug + Clone + Copy {
	fn ring_to_power_of_2(&self, ring: u8) -> u8;

	fn ring_to_resolution(&self, ring: u8) -> usize {
		2_usize.pow(self.ring_to_power_of_2(ring) as u32)
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Ring {
	pub size: f32,
	pub res_2: u8,
	// the point at the lower left bottom corner of the ring
	pub lower_left_bottom: Vec3,
}

impl Ring {
	pub fn new(size: f32, res_2: u8, lower_left_bottom: Vec3) -> Self {
		Self { size, res_2, lower_left_bottom }
	}

	pub fn ring_chunks(&self) -> Result<RingChunks, String> {
		let mut chunks = Vec::new();
		for x in 0..3 {
			for y in 0..3 {
				for z in 0..3 {
					if x == 1 && y == 1 && z == 1 {
						continue;
					}

					chunks.push(CascadeChunk {
						origin: self.lower_left_bottom
							+ Vec3::new(
								x as f32 * self.size,
								y as f32 * self.size,
								z as f32 * self.size,
							),
						size: self.size,
						res_2: self.res_2,
						omit: None,
					});
				}
			}
		}
		RingChunks::try_from(chunks)
	}
}

#[derive(Debug, Clone, Copy)]
pub struct RingChunks {
	pub chunks: [CascadeChunk; 26],
}

impl TryFrom<Vec<CascadeChunk>> for RingChunks {
	type Error = String;

	fn try_from(chunks: Vec<CascadeChunk>) -> Result<Self, Self::Error> {
		if chunks.len() != 26 {
			return Err(format!("Expected 26 chunks, got {}", chunks.len()));
		}
		Ok(Self {
			chunks: chunks
				.try_into()
				.map_err(|e| format!("Failed to convert chunks to array: {:?}", e))?,
		})
	}
}

fn lex_cmp(a: &Vec3, b: &Vec3) -> std::cmp::Ordering {
	a.x.partial_cmp(&b.x)
		.unwrap_or(std::cmp::Ordering::Equal)
		.then_with(|| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
		.then_with(|| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal))
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct CascadeChunk {
	pub origin: Vec3,
	pub size: f32,
	pub res_2: u8,
	pub omit: Option<Aabb3d>,
}

impl Hash for CascadeChunk {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.origin.x.to_bits().hash(state);
		self.origin.y.to_bits().hash(state);
		self.origin.z.to_bits().hash(state);
		self.size.to_bits().hash(state);
		self.res_2.hash(state);
	}
}

impl CascadeChunk {
	pub fn resolution(&self) -> usize {
		2_usize.pow(self.res_2 as u32)
	}

	/// Creates a chunk with a bottom left corner at the origin and a size of 1.0.
	pub fn unit_chunk() -> Self {
		Self { origin: Vec3::ZERO, size: 1.0, res_2: 0, omit: None }
	}

	/// Creates a chunk with the center at the origin and diameters of 1.0.
	pub fn unit_center_chunk() -> Self {
		Self { origin: Vec3::new(-0.5, 0.0, -0.5), size: 1.0, res_2: 0, omit: None }
	}

	/// Creates a chunk with the center at the origin and a size of 1.0.
	pub fn unit_3d_center_chunk() -> Self {
		Self { origin: Vec3::new(-0.5, -0.5, -0.5), size: 1.0, res_2: 0, omit: None }
	}

	/// Updates a chunk with some Mu for the geometry that goes slightly beyond the unit.
	pub fn with_mu(self, mu: f32) -> Self {
		let origin = self.origin + Vec3::new(-mu, -mu, -mu);
		let size = self.size + 2.0 * mu;
		Self { origin, size, res_2: self.res_2, omit: self.omit }
	}

	pub fn with_res_2(mut self, res_2: u8) -> Self {
		self.res_2 = res_2;
		self
	}
}

fn vec3a_cmp(a: &bevy::math::Vec3A, b: &bevy::math::Vec3A) -> std::cmp::Ordering {
	a.x.partial_cmp(&b.x)
		.unwrap_or(std::cmp::Ordering::Equal)
		.then_with(|| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
		.then_with(|| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal))
}

fn aabb_cmp(a: &Option<Aabb3d>, b: &Option<Aabb3d>) -> std::cmp::Ordering {
	match (a, b) {
		(None, None) => std::cmp::Ordering::Equal,
		(None, Some(_)) => std::cmp::Ordering::Less,
		(Some(_), None) => std::cmp::Ordering::Greater,
		(Some(aa), Some(ab)) => {
			// Compare Aabb3d by min point, then max point
			vec3a_cmp(&aa.min, &ab.min).then_with(|| vec3a_cmp(&aa.max, &ab.max))
		}
	}
}

impl PartialOrd for CascadeChunk {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		// compare the size first, then the resolution, then the origin, then the omission
		Some(
			self.size
				.partial_cmp(&other.size)
				.unwrap_or(std::cmp::Ordering::Equal)
				.then_with(|| {
					self.res_2.partial_cmp(&other.res_2).unwrap_or(std::cmp::Ordering::Equal)
				})
				.then_with(|| lex_cmp(&self.origin, &other.origin))
				.then_with(|| aabb_cmp(&self.omit, &other.omit)),
		)
	}
}

impl Eq for CascadeChunk {}

impl Ord for CascadeChunk {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.partial_cmp(other).unwrap()
	}
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Cascade<R: ResolutionMap> {
	/// The minimum size of the chunk used in the interior of the cascade
	pub min_size: f32,
	/// The number of rings in the cascade
	pub number_of_rings: u8,
	/// The resolution map for the cascade and grid.
	pub resolution_map: R,
	/// The Manhattan radius of the grid in the cascade
	pub grid_radius: usize,
	/// The base two power of the multiple of the size of the largest ring in the cascade.
	pub grid_multiple_2: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CascadeOutput {
	pub cascade_chunks: HashSet<CascadeChunk>,
	pub grid_chunks: HashSet<CascadeChunk>,
}

impl CascadeOutput {
	pub fn all(&self) -> Vec<CascadeChunk> {
		let mut chunks = Vec::new();
		chunks.extend(self.cascade_chunks.clone());
		chunks.extend(self.grid_chunks.clone());
		chunks
	}

	pub fn cascade(&self) -> &HashSet<CascadeChunk> {
		&self.cascade_chunks
	}

	pub fn grid(&self) -> &HashSet<CascadeChunk> {
		&self.grid_chunks
	}

	pub fn contains(&self, chunk: &CascadeChunk) -> bool {
		self.cascade_chunks.contains(chunk) || self.grid_chunks.contains(chunk)
	}
}

impl<R: ResolutionMap> Cascade<R> {
	pub fn size_for_ring(&self, ring: u8) -> f32 {
		self.min_size * 3_u32.pow(ring as u32) as f32
	}

	pub fn position_to_origin(&self, position: Vec3) -> Vec3 {
		let x = (position.x / self.min_size).floor() * self.min_size;
		let y = (position.y / self.min_size).floor() * self.min_size;
		let z = (position.z / self.min_size).floor() * self.min_size;
		Vec3::new(x, y, z)
	}

	pub fn center_chunk(&self, position: Vec3) -> CascadeChunk {
		let origin = self.position_to_origin(position);
		CascadeChunk {
			origin,
			size: self.min_size,
			res_2: self.resolution_map.ring_to_power_of_2(0),
			omit: None,
		}
	}

	/// The chunks in the cascade.
	pub fn cascade_chunks(&self, position: Vec3) -> Result<Vec<CascadeChunk>, String> {
		// copmute the center chunk
		let center_chunk = self.center_chunk(position);

		// move to the lower bottom left for the 0th ring
		let mut lower_left_bottom =
			center_chunk.origin - Vec3::new(self.min_size, self.min_size, self.min_size);

		// add the center chunk to the chunks vector
		let mut chunks = Vec::new();
		chunks.push(center_chunk);

		// iterate over the rings and add the chunks to the chunks vector
		for ring in 0..self.number_of_rings {
			// slightly redundant here,
			// as this is the same as the next_size at the end of the previous iteration
			// but it's IMO clearer to have the size here
			let size = self.size_for_ring(ring);

			// create the ring chunks
			let ring_chunks =
				Ring::new(size, self.resolution_map.ring_to_power_of_2(ring), lower_left_bottom)
					.ring_chunks()?;

			// add the ring chunks to the chunks vector
			chunks.extend(ring_chunks.chunks);

			// move to the new lower bottom left for the next ring
			let next_size = self.size_for_ring(ring + 1);
			lower_left_bottom = lower_left_bottom - Vec3::new(next_size, next_size, next_size);
		}
		Ok(chunks)
	}

	/// Computes the multiple of the grid.
	pub fn grid_multiple(&self) -> usize {
		2_usize.pow(self.grid_multiple_2 as u32)
	}

	/// Computes the size of the grid chunks.
	pub fn grid_chunk_size(&self) -> f32 {
		self.span() * self.grid_multiple() as f32
	}

	/// The chunks in the grid.
	///
	/// The grid is globally defined, and the cascade chunks are carved out of it.
	/// This allows for low-cost distant features over a tight high-resolution cascade.
	/// You don't always have to cascade out to the general world resolution that you want.
	pub fn grid_chunks(&self, position: Vec3) -> Result<Vec<CascadeChunk>, String> {
		let omit = Some(self.cascade_aabb(position));
		let origin_x = (position.x / self.grid_chunk_size()).floor() * self.grid_chunk_size();
		let origin_y = self.grid_chunk_size() / -2.0;
		let origin_z = (position.z / self.grid_chunk_size()).floor() * self.grid_chunk_size();
		let origin = Vec3::new(origin_x, origin_y, origin_z);
		let mut chunks = Vec::new();

		// construct the 2D grid of chunks
		for x in -(self.grid_radius as i32)..=(self.grid_radius as i32) {
			for z in -(self.grid_radius as i32)..=(self.grid_radius as i32) {
				let chunk_origin = origin
					+ Vec3::new(
						x as f32 * self.grid_chunk_size(),
						0.0,
						z as f32 * self.grid_chunk_size(),
					);
				let chunk = CascadeChunk {
					origin: chunk_origin,
					size: self.grid_chunk_size(),
					res_2: self.resolution_map.ring_to_power_of_2(self.number_of_rings),
					omit,
				};
				chunks.push(chunk);
			}
		}

		Ok(chunks)
	}

	pub fn chunks(&self, position: Vec3) -> Result<CascadeOutput, String> {
		let cascade_chunks = self.cascade_chunks(position)?.into_iter().collect();
		let grid_chunks = self.grid_chunks(position)?.into_iter().collect();
		Ok(CascadeOutput { cascade_chunks, grid_chunks })
	}

	pub fn new_chunks(
		&self,
		prev: Vec3,
		new: Vec3,
	) -> Result<(CascadeOutput, CascadeOutput), String> {
		// compute the new chunks
		let new_chunks = self.chunks(new)?;

		// compute the previous chunks
		let prev_chunks = self.chunks(prev)?;

		// compute the new chunks that are not in the previous chunks
		let cascade_chunks = new_chunks
			.cascade_chunks
			.difference(&prev_chunks.cascade_chunks)
			.cloned()
			.collect();
		let grid_chunks =
			new_chunks.grid_chunks.difference(&prev_chunks.grid_chunks).cloned().collect();

		Ok((CascadeOutput { cascade_chunks, grid_chunks }, new_chunks))
	}

	pub fn needs_new_chunks(&self, prev: Vec3, new: Vec3) -> bool {
		self.position_to_origin(prev) != self.position_to_origin(new)
	}

	/// Computes the number of units in x = y = z that the cube formed by the cascade spans
	///
	/// This is merely the the largest of the rings in the cascade.
	///
	/// For the most part, total world size should be a multiple of this value,
	/// s.t. coordinate wrap arounds align nicely with the chunks.
	pub fn span(&self) -> f32 {
		self.min_size * 3_u32.pow(self.number_of_rings as u32) as f32
	}

	/// Computes the lower bottom left for the entire cascade.
	pub fn cascade_lower_left_bottom(&self, position: Vec3) -> Vec3 {
		let mut position = self.position_to_origin(position);
		for ring in 0..self.number_of_rings {
			position = position
				- Vec3::new(
					self.size_for_ring(ring),
					self.size_for_ring(ring),
					self.size_for_ring(ring),
				);
		}
		position
	}

	/// Computes the AaBb for the entire cascade.
	pub fn cascade_aabb(&self, position: Vec3) -> Aabb3d {
		let lower_left_bottom = self.cascade_lower_left_bottom(position);
		let upper_right_top = lower_left_bottom + Vec3::new(self.span(), self.span(), self.span());
		Aabb3d::new(lower_left_bottom, upper_right_top)
	}
}

#[derive(Debug, Clone, Copy)]
pub struct ConstantResolutionMap {
	pub res_2: u8,
}

impl ResolutionMap for ConstantResolutionMap {
	fn ring_to_power_of_2(&self, _ring: u8) -> u8 {
		self.res_2
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::BTreeSet;

	fn zeroth_ring(size: f32, res_2: u8, lower_left_bottom: Vec3) -> Vec<CascadeChunk> {
		// Explicit reference for all 27 chunks of a 3x3x3 cube
		// Organized by z-level: z = 0, z = 1, z = 2
		// Each chunk origin is relative to lower_left_bottom with offsets 0, 1, or 2
		vec![
			// z = 0 level (9 chunks)
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(0.0 * size, 0.0 * size, 0.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(1.0 * size, 0.0 * size, 0.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(2.0 * size, 0.0 * size, 0.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(0.0 * size, 1.0 * size, 0.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(1.0 * size, 1.0 * size, 0.0 * size),
				size,
				res_2,
				omit: None,
			}, // center
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(2.0 * size, 1.0 * size, 0.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(0.0 * size, 2.0 * size, 0.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(1.0 * size, 2.0 * size, 0.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(2.0 * size, 2.0 * size, 0.0 * size),
				size,
				res_2,
				omit: None,
			},
			// z = 1 level (9 chunks)
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(0.0 * size, 0.0 * size, 1.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(1.0 * size, 0.0 * size, 1.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(2.0 * size, 0.0 * size, 1.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(0.0 * size, 1.0 * size, 1.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(1.0 * size, 1.0 * size, 1.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(2.0 * size, 1.0 * size, 1.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(0.0 * size, 2.0 * size, 1.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(1.0 * size, 2.0 * size, 1.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(2.0 * size, 2.0 * size, 1.0 * size),
				size,
				res_2,
				omit: None,
			},
			// z = 2 level (9 chunks)
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(0.0 * size, 0.0 * size, 2.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(1.0 * size, 0.0 * size, 2.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(2.0 * size, 0.0 * size, 2.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(0.0 * size, 1.0 * size, 2.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(1.0 * size, 1.0 * size, 2.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(2.0 * size, 1.0 * size, 2.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(0.0 * size, 2.0 * size, 2.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(1.0 * size, 2.0 * size, 2.0 * size),
				size,
				res_2,
				omit: None,
			},
			CascadeChunk {
				origin: lower_left_bottom + Vec3::new(2.0 * size, 2.0 * size, 2.0 * size),
				size,
				res_2,
				omit: None,
			},
		]
	}

	fn nth_ring(size: f32, res_2: u8, lower_left_bottom: Vec3) -> Vec<CascadeChunk> {
		// Get the zeroth ring (all 27 chunks)
		let all_chunks = zeroth_ring(size, res_2, lower_left_bottom);

		// Convert to BTreeSet to easily remove the middle chunk
		let mut chunks_set: BTreeSet<CascadeChunk> = all_chunks.into_iter().collect();

		// Remove the middle chunk (center is at lower_left_bottom + (1*size, 1*size, 1*size))
		let center_chunk = CascadeChunk {
			origin: lower_left_bottom + Vec3::new(1.0 * size, 1.0 * size, 1.0 * size),
			size,
			res_2,
			omit: None,
		};
		chunks_set.remove(&center_chunk);

		// Convert back to Vec
		chunks_set.into_iter().collect()
	}

	#[test]
	fn test_cascade_ones() -> Result<(), String> {
		let cascade = Cascade {
			min_size: 1.0,
			number_of_rings: 1,
			resolution_map: ConstantResolutionMap { res_2: 0 },
			grid_radius: 1,
			grid_multiple_2: 0,
		};

		let position = Vec3::new(0.0, 0.0, 0.0);
		let chunks = cascade.chunks(position)?;
		let chunks = chunks.cascade();

		// the zero chunk and the 26 chunks in the first ring
		assert_eq!(chunks.len(), 27);

		// put the chunks in a BTreeSet to sort them
		let mut chunks_sorted = BTreeSet::new();
		for chunk in chunks {
			chunks_sorted.insert(chunk.clone());
		}

		// make sure there are 27 unique chunks
		assert_eq!(chunks_sorted.len(), 27);

		// now form a reference vector for the 3x3x3 cube
		// lower_left_bottom for the 0th ring is center - (min_size, min_size, min_size)
		let lower_left_bottom = Vec3::new(0.0, 0.0, 0.0) - Vec3::new(1.0, 1.0, 1.0);
		let reference_chunks_vec = zeroth_ring(1.0, 0, lower_left_bottom);
		assert_eq!(reference_chunks_vec.len(), 27);

		// Convert reference to BTreeSet for comparison
		let mut reference_sorted = BTreeSet::new();
		for chunk in reference_chunks_vec {
			reference_sorted.insert(chunk);
		}

		// Assert that we build out the 3x3x3 cube correctly
		assert_eq!(chunks_sorted, reference_sorted);

		Ok(())
	}

	#[test]
	fn test_cascade_concentric_rings() -> Result<(), String> {
		let cascade = Cascade {
			min_size: 1.0,
			number_of_rings: 2,
			resolution_map: ConstantResolutionMap { res_2: 0 },
			grid_radius: 1,
			grid_multiple_2: 0,
		};
		let position = Vec3::new(0.0, 0.0, 0.0);
		let chunks = cascade.chunks(position)?;
		let chunks = chunks.cascade();

		// Center chunk + 26 chunks from ring 0 + 26 chunks from ring 1 = 53 chunks
		assert_eq!(chunks.len(), 53);

		// Build expected reference
		let mut expected_chunks = BTreeSet::new();

		// Center chunk
		let center_chunk =
			CascadeChunk { origin: Vec3::new(0.0, 0.0, 0.0), size: 1.0, res_2: 0, omit: None };
		expected_chunks.insert(center_chunk);

		// Ring 0: lower_left_bottom = center - (min_size, min_size, min_size)
		let ring0_lower_left_bottom = Vec3::new(0.0, 0.0, 0.0) - Vec3::new(1.0, 1.0, 1.0);
		let ring0_chunks = nth_ring(1.0, 0, ring0_lower_left_bottom);
		for chunk in ring0_chunks {
			expected_chunks.insert(chunk.clone());
		}

		// Ring 1: size = 3, lower_left_bottom = ring0_lower_left_bottom - (3, 3, 3)
		let ring1_size = 3.0;
		let ring1_lower_left_bottom = ring0_lower_left_bottom - Vec3::new(3.0, 3.0, 3.0);
		let ring1_chunks = nth_ring(ring1_size, 0, ring1_lower_left_bottom);
		for chunk in ring1_chunks {
			expected_chunks.insert(chunk);
		}

		// Convert actual chunks to BTreeSet
		let mut chunks_sorted = BTreeSet::new();
		for chunk in chunks {
			chunks_sorted.insert(chunk.clone());
		}

		assert_eq!(chunks_sorted, expected_chunks);

		Ok(())
	}

	#[test]
	fn test_cascade_size_greater_than_one() -> Result<(), String> {
		let cascade = Cascade {
			min_size: 2.5,
			number_of_rings: 1,
			resolution_map: ConstantResolutionMap { res_2: 1 },
			grid_radius: 1,
			grid_multiple_2: 0,
		};
		let position = Vec3::new(0.0, 0.0, 0.0);
		let chunks = cascade.chunks(position)?;
		let chunks = chunks.cascade();

		// Center chunk + 26 chunks from ring 0 = 27 chunks
		assert_eq!(chunks.len(), 27);

		// Build expected reference
		let mut expected_chunks = BTreeSet::new();

		// Center chunk
		let center_chunk =
			CascadeChunk { origin: Vec3::new(0.0, 0.0, 0.0), size: 2.5, res_2: 1, omit: None };
		expected_chunks.insert(center_chunk);

		// Ring 0: lower_left_bottom = center - (min_size, min_size, min_size)
		let ring0_lower_left_bottom = Vec3::new(0.0, 0.0, 0.0) - Vec3::new(2.5, 2.5, 2.5);
		let ring0_chunks = nth_ring(2.5, 1, ring0_lower_left_bottom);
		for chunk in ring0_chunks {
			expected_chunks.insert(chunk.clone());
		}

		// Convert actual chunks to BTreeSet
		let mut chunks_sorted = BTreeSet::new();
		for chunk in chunks {
			chunks_sorted.insert(chunk.clone());
		}

		assert_eq!(chunks_sorted, expected_chunks);

		Ok(())
	}

	#[test]
	fn test_cascade_size_less_than_one() -> Result<(), String> {
		let cascade = Cascade {
			min_size: 0.5,
			number_of_rings: 1,
			resolution_map: ConstantResolutionMap { res_2: 2 },
			grid_radius: 1,
			grid_multiple_2: 0,
		};
		let position = Vec3::new(0.0, 0.0, 0.0);
		let chunks = cascade.chunks(position)?;
		let chunks = chunks.cascade();

		// Center chunk + 26 chunks from ring 0 = 27 chunks
		assert_eq!(chunks.len(), 27);

		// Build expected reference
		let mut expected_chunks = BTreeSet::new();

		// Center chunk
		let center_chunk =
			CascadeChunk { origin: Vec3::new(0.0, 0.0, 0.0), size: 0.5, res_2: 2, omit: None };
		expected_chunks.insert(center_chunk.clone());

		// Ring 0: lower_left_bottom = center - (min_size, min_size, min_size)
		let ring0_lower_left_bottom = Vec3::new(0.0, 0.0, 0.0) - Vec3::new(0.5, 0.5, 0.5);
		let ring0_chunks = nth_ring(0.5, 2, ring0_lower_left_bottom);
		for chunk in ring0_chunks {
			expected_chunks.insert(chunk.clone());
		}

		// Convert actual chunks to BTreeSet
		let mut chunks_sorted = BTreeSet::new();
		for chunk in chunks {
			chunks_sorted.insert(chunk.clone());
		}

		assert_eq!(chunks_sorted, expected_chunks);

		Ok(())
	}
}
