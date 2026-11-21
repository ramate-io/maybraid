//! CPU patch ownership: bounds, seeds, species, and surface samples.
//!
//! One entity per patch. Blades are compact descriptors, never ECS entities.

use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use vegetation_shaders::wesl::{hash_u32, plant_id, BladeDescriptor, GrassPatchHeader};

use crate::bounds::GrassBounds;
use crate::field::GrassFieldSettings;
use crate::host::GrassPatchLod;
use crate::species::GrassSpecies;

/// World-space surface sample. Works for height maps *and* SDF meshes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceSample {
	/// World metres.
	pub root: Vec3,
	/// Unit surface normal.
	pub normal: Vec3,
}

impl SurfaceSample {
	pub fn planar(root: Vec3) -> Self {
		Self { root, normal: Vec3::Y }
	}

	pub fn on_slope(root: Vec3, normal: Vec3) -> Self {
		Self { root, normal: normal.normalize_or_zero().normalize_or(Vec3::Y) }
	}
}

/// Bounded spatial patch. Upload compact roots when this changes.
#[derive(Component, Clone, Debug)]
pub struct GrassPatch {
	pub seed: u32,
	pub species: GrassSpecies,
	pub coverage: f32,
	pub samples: Vec<SurfaceSample>,
	pub lod: GrassPatchLod,
}

impl GrassPatch {
	pub fn new(
		seed: u32,
		species: GrassSpecies,
		coverage: f32,
		samples: Vec<SurfaceSample>,
	) -> Self {
		Self {
			seed,
			species,
			coverage: coverage.clamp(0.0, 1.0),
			samples,
			lod: GrassPatchLod::High,
		}
	}

	pub fn isolated_clump(seed: u32, center: Vec3, count: u32) -> Self {
		Self::new(seed, GrassSpecies::meadow(), 1.0, clump_samples(center, count, 0.52, seed))
	}

	pub fn dense_field(seed: u32, origin: Vec3, extent_xz: Vec2, spacing_m: f32) -> Self {
		Self::new(
			seed,
			GrassSpecies::meadow(),
			1.0,
			grid_samples(origin, extent_xz, spacing_m, seed, Vec3::Y),
		)
	}

	/// Same grid as [`Self::dense_field`] but with a constant surface normal (slopes / SDF).
	pub fn sloped_field(
		seed: u32,
		origin: Vec3,
		extent_xz: Vec2,
		spacing_m: f32,
		normal: Vec3,
	) -> Self {
		Self::new(
			seed,
			GrassSpecies::shade(),
			1.0,
			grid_samples(origin, extent_xz, spacing_m, seed, normal),
		)
	}

	/// A raised crest of samples, used for skyline / hill-crest playground views.
	pub fn hill_crest(seed: u32, origin: Vec3, length_m: f32, height_m: f32, count: u32) -> Self {
		let mut samples = Vec::with_capacity(count as usize);
		for i in 0..count {
			let t = if count <= 1 { 0.5 } else { i as f32 / (count - 1) as f32 };
			let x = (t - 0.5) * length_m;
			let y = height_m * (1.0 - (t - 0.5).abs() * 2.0).max(0.0);
			let slope = Vec3::new(-(t - 0.5) * 0.8, 1.0, 0.0);
			samples.push(SurfaceSample::on_slope(origin + Vec3::new(x, y, 0.0), slope));
		}
		Self::new(seed, GrassSpecies::shade(), 1.0, samples)
	}

	pub fn with_lod(mut self, lod: GrassPatchLod) -> Self {
		self.lod = lod;
		self
	}

	pub fn with_species(mut self, species: GrassSpecies) -> Self {
		self.species = species;
		self
	}

	pub fn is_empty(&self) -> bool {
		self.samples.is_empty() || self.coverage <= 0.0 || self.lod.density_scale() <= 0.0
	}

	pub fn bounds(&self, settings: &GrassFieldSettings) -> GrassBounds {
		GrassBounds::from_samples(
			&self.samples,
			self.species.height_m * (1.0 + self.species.height_jitter),
			settings.wind.strength_m,
			self.species.stiffness,
		)
	}

	/// Compact descriptors for this patch. Identity is `plant_id(seed, i)`.
	pub fn descriptors(&self) -> Vec<BladeDescriptor> {
		if self.is_empty() {
			return Vec::new();
		}
		let density = (self.coverage * self.lod.density_scale()).clamp(0.0, 1.0);
		self.samples
			.iter()
			.enumerate()
			.filter_map(|(i, sample)| {
				let id = plant_id(self.seed, i as u32);
				let rank = vegetation_shaders::wesl::density_rank(id);
				if rank >= density {
					return None;
				}
				Some(self.descriptor_at(i as u32, *sample, id, rank))
			})
			.collect()
	}

	pub fn header(
		&self,
		blade_offset: u32,
		blade_count: u32,
		settings: &GrassFieldSettings,
	) -> GrassPatchHeader {
		let bounds = self.bounds(settings);
		GrassPatchHeader {
			aabb_min: bounds.min,
			blade_offset,
			aabb_max: bounds.max,
			blade_count,
			seed: self.seed,
			coverage: self.coverage * self.lod.density_scale(),
			species: 0,
			flags: if blade_count == 0 { 0 } else { GrassPatchHeader::FLAG_ACTIVE },
		}
	}

	fn descriptor_at(
		&self,
		index: u32,
		sample: SurfaceSample,
		id: u32,
		rank: f32,
	) -> BladeDescriptor {
		let jitter = 0.65 + 0.35 * vegetation_shaders::wesl::hash_u32_to_unit(id ^ 0x27d4eb2d);
		let height =
			self.species.height_m * (1.0 + self.species.height_jitter * (jitter - 0.5) * 2.0);
		let yaw = hash_u32(id ^ 0x165667b1) as f32 * (core::f32::consts::TAU / 4_294_967_295.0);
		let (sin_y, cos_y) = yaw.sin_cos();
		let tangent = sample.normal.any_orthonormal_vector();
		let facing = (tangent * cos_y + sample.normal.cross(tangent) * sin_y).normalize_or(Vec3::X);
		let bend_u = vegetation_shaders::wesl::hash_u32_to_unit(id ^ 0x3c6ef372);
		let bend = self.species.bend * (0.48 + 1.22 * bend_u);
		let width_u = vegetation_shaders::wesl::hash_u32_to_unit(id ^ 0xa0761d65);
		let width = self.species.width_m * (0.82 + 0.36 * width_u);
		let pal = self.species.palette.varied(sample.root.xz(), id);
		BladeDescriptor {
			root: sample.root,
			height_m: height.max(0.05),
			normal: sample.normal.normalize_or(Vec3::Y),
			width_m: width,
			facing,
			plant_id: id,
			shape: Vec4::new(bend, self.species.taper, self.species.stiffness, rank),
			palette: Vec4::new(pal.tip_rgb.x, pal.tip_rgb.y, pal.tip_rgb.z, index as f32),
		}
	}
}

fn clump_samples(center: Vec3, count: u32, radius_m: f32, seed: u32) -> Vec<SurfaceSample> {
	(0..count)
		.map(|i| {
			let h = hash_u32(seed ^ plant_id(seed, i));
			let a = (h as f32) * (core::f32::consts::TAU / 4_294_967_295.0);
			let r = radius_m * ((hash_u32(h ^ 0x9e3779b9) as f32) / 4_294_967_295.0).sqrt();
			SurfaceSample::planar(center + Vec3::new(a.cos() * r, 0.0, a.sin() * r))
		})
		.collect()
}

fn grid_samples(
	origin: Vec3,
	extent_xz: Vec2,
	spacing_m: f32,
	seed: u32,
	normal: Vec3,
) -> Vec<SurfaceSample> {
	let spacing = spacing_m.max(0.05);
	let nx = (extent_xz.x / spacing).ceil().max(1.0) as u32;
	let nz = (extent_xz.y / spacing).ceil().max(1.0) as u32;
	let mut samples = Vec::with_capacity((nx * nz) as usize);
	for z in 0..nz {
		for x in 0..nx {
			let i = z * nx + x;
			let jitter = hash_u32(seed ^ i);
			let jx = ((jitter & 0xff) as f32 / 255.0 - 0.5) * spacing * 0.7;
			let jz = (((jitter >> 8) & 0xff) as f32 / 255.0 - 0.5) * spacing * 0.7;
			let root = origin
				+ Vec3::new((x as f32 + 0.5) * spacing + jx, 0.0, (z as f32 + 0.5) * spacing + jz);
			samples.push(SurfaceSample::on_slope(root, normal));
		}
	}
	samples
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn zero_coverage_emits_no_blades() -> anyhow::Result<()> {
		let patch =
			GrassPatch::dense_field(1, Vec3::ZERO, Vec2::splat(4.0), 0.5).with_coverage(0.0);
		assert!(patch.descriptors().is_empty());
		Ok(())
	}

	#[test]
	fn negative_roots_are_preserved() -> anyhow::Result<()> {
		let patch = GrassPatch::isolated_clump(4, Vec3::new(-12.0, 3.0, -8.0), 8);
		let desc = patch.descriptors();
		assert!(!desc.is_empty());
		assert!(desc.iter().all(|d| d.root.x < 0.0 && d.root.z < 0.0));
		Ok(())
	}

	#[test]
	fn plant_ids_do_not_depend_on_vec_order_after_filter() -> anyhow::Result<()> {
		let a = GrassPatch::dense_field(9, Vec3::ZERO, Vec2::splat(3.0), 0.4);
		let b = GrassPatch::dense_field(9, Vec3::ZERO, Vec2::splat(3.0), 0.4);
		let ids_a: Vec<_> = a.descriptors().iter().map(|d| d.plant_id).collect();
		let ids_b: Vec<_> = b.descriptors().iter().map(|d| d.plant_id).collect();
		assert_eq!(ids_a, ids_b);
		Ok(())
	}
}

impl GrassPatch {
	pub fn with_coverage(mut self, coverage: f32) -> Self {
		self.coverage = coverage;
		self
	}
}
