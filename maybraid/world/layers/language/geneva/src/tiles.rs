//! Large language tiles and guillotine small tiles.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use comproc::guillotine::{Bounds2, DepthRange, GuillotineConfig, VariableGuillotine};
use comproc::noise::config::NoiseConfig;
use lod::gen::Id;
use maybraid_language_core::lexicalizer::mix;
use noise::Perlin;

use crate::bundle::LanguageBundle;

/// Large-tile side in game units (~25 km).
pub const LARGE_TILE: f32 = 25_000.0;
/// Preferred small-tile step lower bound.
pub const SMALL_STEP_MIN: f32 = 5_000.0;
/// Preferred small-tile step upper bound.
pub const SMALL_STEP_MAX: f32 = 8_000.0;

const MIN_LANGUAGES: u8 = 3;
const MAX_LANGUAGES: u8 = 12;

/// Large tile: composed language set plus nested small tiles.
#[derive(Clone, Debug, PartialEq)]
pub struct LargeTile {
	pub ix: i32,
	pub iz: i32,
	pub languages: Vec<LanguageBundle>,
	pub small: Vec<SmallTile>,
}

/// Guillotine leaf that holds a nonempty subset of the parent languages.
#[derive(Clone, Debug, PartialEq)]
pub struct SmallTile {
	pub bounds: Bounds2,
	pub language_ids: Vec<u8>,
}

impl LargeTile {
	pub fn generate(world_seed: u64, ix: i32, iz: i32) -> Self {
		let count = language_count(world_seed, ix, iz);
		let coarse = (ix.div_euclid(2), iz.div_euclid(2));
		let languages = (0..count)
			.map(|index| LanguageBundle::compose(world_seed, coarse, (ix, iz), index))
			.collect::<Vec<_>>();
		let small = partition_small(world_seed, ix, iz, languages.len() as u8);
		Self { ix, iz, languages, small }
	}

	pub fn bounds(&self) -> Bounds2 {
		let origin = large_tile_origin(self.ix, self.iz);
		Bounds2::from_origin_extent([origin.0, origin.1], [LARGE_TILE, LARGE_TILE])
	}

	pub fn language(&self, id: u8) -> Option<&LanguageBundle> {
		self.languages.get(id as usize)
	}

	pub fn small_tile_at(&self, x: f32, z: f32) -> Option<&SmallTile> {
		self.small.iter().find(|tile| contains_xz(tile.bounds, x, z))
	}

	/// HCSG id of tile `(ix, iz)`: its footprint cell.
	pub fn id(ix: i32, iz: i32) -> Id {
		Id::from_cell(large_tile_aabb(ix, iz))
	}

	/// Tile index of an id minted by [`Self::id`].
	pub fn index_of(id: Id) -> Option<(i32, i32)> {
		let cell = id.origin_cell_bounds()?;
		let half = LARGE_TILE * 0.5;
		let (ix, iz) = (large_tile_index(cell.min.x + half), large_tile_index(cell.min.z + half));
		(Self::id(ix, iz) == id).then_some((ix, iz))
	}
}

/// Footprint of large tile `(ix, iz)`, one unit either side of `y = 0`.
pub fn large_tile_aabb(ix: i32, iz: i32) -> Aabb3d {
	let (x, z) = large_tile_origin(ix, iz);
	Aabb3d::from_min_max(Vec3::new(x, -1.0, z), Vec3::new(x + LARGE_TILE, 1.0, z + LARGE_TILE))
}

/// Large-tile index containing world `x` (or `z`).
pub fn large_tile_index(axis: f32) -> i32 {
	(axis / LARGE_TILE).floor() as i32
}

/// World-space origin of large tile `(ix, iz)`.
pub fn large_tile_origin(ix: i32, iz: i32) -> (f32, f32) {
	(ix as f32 * LARGE_TILE, iz as f32 * LARGE_TILE)
}

pub fn language_count(world_seed: u64, ix: i32, iz: i32) -> u8 {
	let span = u64::from(MAX_LANGUAGES - MIN_LANGUAGES + 1);
	MIN_LANGUAGES + (mix(world_seed ^ mix(ix as u64) ^ mix(iz as u64).wrapping_mul(3)) % span) as u8
}

fn partition_small(world_seed: u64, ix: i32, iz: i32, language_count: u8) -> Vec<SmallTile> {
	let origin = large_tile_origin(ix, iz);
	let root = Bounds2::from_origin_extent([origin.0, origin.1], [LARGE_TILE, LARGE_TILE]);
	let seed = mix(world_seed ^ mix(ix as u64) ^ mix((iz as u64).wrapping_mul(17))) as u32;
	let noise = NoiseConfig::new(Perlin::default())
		.with_seed(seed)
		.with_frequency(0.0002)
		.with_amplitude(1.0)
		.with_octaves(1);
	let cutter = VariableGuillotine::<2, _>::new(
		noise,
		GuillotineConfig::new(SMALL_STEP_MIN, SMALL_STEP_MAX),
		DepthRange::new(10, 16),
	);
	cutter
		.regions_vec(root)
		.into_iter()
		.enumerate()
		.map(|(leaf, bounds)| SmallTile {
			bounds,
			language_ids: language_subset(world_seed, ix, iz, leaf, language_count),
		})
		.collect()
}

fn language_subset(world_seed: u64, ix: i32, iz: i32, leaf: usize, language_count: u8) -> Vec<u8> {
	let n = language_count.max(1);
	let mut ids = (0..n).collect::<Vec<_>>();
	let mut rng = mix(world_seed ^ mix(ix as u64) ^ mix(iz as u64) ^ mix(leaf as u64));
	for i in 0..ids.len() {
		rng = mix(rng);
		let j = (rng as usize) % ids.len();
		ids.swap(i, j);
	}
	let keep = 1 + (mix(rng ^ 0x51) as usize % n as usize);
	ids.truncate(keep);
	ids.sort_unstable();
	ids
}

fn contains_xz(bounds: Bounds2, x: f32, z: f32) -> bool {
	x >= bounds.min[0] && x < bounds.max[0] && z >= bounds.min[1] && z < bounds.max[1]
}
