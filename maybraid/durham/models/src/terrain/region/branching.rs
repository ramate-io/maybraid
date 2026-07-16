use crate::terrain::region::affine::RegionAffineModulation;
use crate::terrain::region::RegionNoise;
use procedural_common::{NoiseConfig, NoiseParams, NoiseType};

/// Expand a base affine modulation into a tree of branched regions.
pub struct BranchingPlan {
	regions: Vec<RegionAffineModulation>,
	noise: NoiseConfig,
	depth: usize,
	breadth: usize,
}

impl BranchingPlan {
	pub fn new(
		base_region: RegionAffineModulation,
		seed: u32,
		depth: usize,
		breadth: usize,
	) -> Self {
		let noise = NoiseConfig::new(NoiseParams {
			seed: seed as i32,
			frequency: 1.0,
			amplitude: 1.0,
			octaves: 1,
			noise_type: NoiseType::Perlin,
		});
		Self { regions: vec![base_region], noise, depth, breadth }
	}

	pub fn generate_regions(&self) -> Vec<RegionAffineModulation> {
		let mut total_regions = Vec::new();
		let mut last_regions = self.regions.clone();

		let fallback_noise = RegionNoise {
			noise: self.noise.clone(),
			amplitude: 1.0,
			frequency: 0.2,
		};

		for i in 0..self.depth {
			let mut new_regions = Vec::new();
			for (j, region) in last_regions.iter().enumerate() {
				for k in 0..self.breadth {
					let base = region.noise.clone().unwrap_or_else(|| fallback_noise.clone());
					let offset = (i * j * k + i + j + k) as i32;
					let noise = base.with_seed_offset(offset);
					new_regions.push(region.branch_region(&noise));
				}
			}
			total_regions.extend(new_regions.clone());
			last_regions = new_regions;
		}
		total_regions
	}
}
