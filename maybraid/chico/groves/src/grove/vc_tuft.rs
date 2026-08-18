//! Shared VegetationComponents host for blade / tuft-patch groves (Monster Grass pattern).
//!
//! Grow placements into unit [`TuftPatch`] plants (optional merge fold); emit High/Medium frond
//! collections, Low upright proxies, and UltraLow carpets tilted to the plant heightfield.

use std::collections::HashMap;

use bevy::prelude::*;
use chico_ball_components::tuft::{BladeTuftShape, SpearTuftShape};
use chico_sbs_trees::TuftPatch;
use chico_vegetation_components::{
	FoliageNode, FrondCollection, FrondRun, Layers, Placement, StickNode, StructuralLod,
	VegetationComponents, FROND_KIT_HALF_X,
};
use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

use super::{placement_noise, FlatTerrainSample, GroveCellVariant, GroveExtent, PaletteMix};

/// Structural LOD factors shared by tuft / grass grove hosts (× footprint).
pub const TUFT_GROVE_STRUCTURAL_HIGH_FACTOR: f32 = 2.0;
pub const TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR: f32 = 5.0;
pub const TUFT_GROVE_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

/// Keep every Nth plant for Medium (¼ density).
const MEDIUM_TUFT_STRIDE: usize = 4;
/// World-space thickness for UltraLow carpets (local Z → surface normal).
const ULTRA_CARPET_THICKNESS: f32 = 0.35;
const ULTRA_GRID: u32 = 2;
/// Square bin side in placement-cell units so area ≈ 8 cells (`√8 × √8` = `2√2`).
const LOW_CELL_STRIDE: f32 = 2.0 * std::f32::consts::SQRT_2;

/// Per-grove Low / UltraLow proxy heights (world meters).
#[derive(Clone, Copy, Debug)]
pub struct TuftGroveProxyHeights {
	pub low: f32,
	pub ultra: f32,
}

impl TuftGroveProxyHeights {
	pub const SHORT: Self = Self { low: 0.8, ultra: 0.25 };
	pub const MID: Self = Self { low: 1.6, ultra: 0.35 };
	pub const TALL: Self = Self { low: 2.5, ultra: 0.45 };
}

/// One grove-local [`TuftPatch`] collection (placement already baked when merged).
#[derive(Clone, Debug)]
pub struct TuftGrovePlant {
	pub placement: Placement,
	pub patch: TuftPatch,
	pub material: MaterialRef,
}

/// Built tuft/grass grove shared by VegetationComponents hosts.
#[derive(Clone, Debug)]
pub struct TuftGroveBody {
	pub plants: Vec<TuftGrovePlant>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
	pub cell_extent_xz: Vec2,
	pub proxy: TuftGroveProxyHeights,
}

/// Stable archetype index in `0..variants` from world XZ.
pub fn patch_variant_index(position: Vec3, variants: u32) -> u32 {
	let variants = variants.max(1);
	let h = position
		.x
		.to_bits()
		.wrapping_mul(0x9e3779b9)
		.wrapping_add(position.z.to_bits().wrapping_mul(0x85ebca77));
	h % variants
}

/// Noise keyed by variant id (not world position) so the same archetype rebuilds identically.
pub fn variant_noise(base: NoiseParams, variant: u32) -> NoiseParams {
	NoiseParams { seed: base.seed ^ (variant as i32).wrapping_mul(0x45d9f3b), ..base }
}

/// Palette → green standard material for one placement.
pub fn material_from_palette(
	mix: PaletteMix,
	position: Vec3,
	foliage_noise: NoiseParams,
) -> MaterialRef {
	let seed = placement_noise(foliage_noise, position).seed;
	mix.pick_color(seed)
		.map(|c| MaterialRef::default().with_palette([c]))
		.unwrap_or_default()
}

/// Stamp authored blade noise amplitudes onto a shape then wrap as a single-clump patch.
pub fn single_blade_patch_params(
	mut shape: BladeTuftShape,
	foliage_noise: NoiseParams,
) -> chico_sbs_trees::TuftPatchParams {
	shape.noise_amplitude = foliage_noise.amplitude;
	shape.noise_frequency = foliage_noise.frequency;
	chico_sbs_trees::TuftPatchParams::new(1, 0.0, shape)
}

/// Approximate a spear clump as a blade tuft patch (VC path has no SpearTuft host yet).
pub fn spear_as_blade_patch_params(
	spear: SpearTuftShape,
	foliage_noise: NoiseParams,
) -> chico_sbs_trees::TuftPatchParams {
	single_blade_patch_params(
		BladeTuftShape {
			blade_count: spear.spear_count,
			blade_length: spear.spear_length,
			blade_width: (spear.belly_half_width * 2.0).max(1e-4),
			max_tilt_radians: spear.max_tilt_radians,
			base_spread: 0.0,
			bend_segments: spear.bend_segments,
			noise_amplitude: foliage_noise.amplitude,
			noise_frequency: foliage_noise.frequency,
			seed: spear.seed,
		},
		foliage_noise,
	)
}

/// Apply foliage noise amp/freq onto params built from [`GroveTuftPatch::build_tuft_patch`].
pub fn stamp_foliage_noise(
	mut params: chico_sbs_trees::TuftPatchParams,
	foliage_noise: NoiseParams,
) -> chico_sbs_trees::TuftPatchParams {
	params.shape.noise_amplitude = foliage_noise.amplitude;
	params.shape.noise_frequency = foliage_noise.frequency;
	params
}

/// Grow unit [`TuftPatch`] plants from prebuilt params; fold when `merge_collections > 0`.
pub fn grow_tuft_plants(
	grown: Vec<(Placement, TuftPatch, MaterialRef)>,
	merge_collections: usize,
) -> Vec<TuftGrovePlant> {
	if merge_collections == 0 {
		return grown
			.into_iter()
			.map(|(placement, patch, material)| TuftGrovePlant { placement, patch, material })
			.collect();
	}
	let pairs: Vec<(Placement, TuftPatch)> =
		grown.iter().map(|(p, patch, _)| (*p, patch.clone())).collect();
	let materials: Vec<MaterialRef> = grown.into_iter().map(|(_, _, m)| m).collect();
	let mut remaining: Vec<(Placement, TuftPatch, MaterialRef)> =
		pairs.into_iter().zip(materials).map(|((p, patch), m)| (p, patch, m)).collect();
	remaining.sort_by(|a, b| {
		a.0.translation
			.x
			.total_cmp(&b.0.translation.x)
			.then(a.0.translation.z.total_cmp(&b.0.translation.z))
	});
	let target = merge_collections.max(1);
	let chunk_len = remaining.len().div_ceil(target);
	let mut plants = Vec::with_capacity(target.min(remaining.len()));
	while !remaining.is_empty() {
		let take = chunk_len.min(remaining.len());
		let chunk: Vec<_> = remaining.drain(..take).collect();
		let material = chunk[0].2.clone();
		let mut iter = chunk.into_iter();
		let (placement, mut merged, _) = iter.next().expect("chunk non-empty");
		merged.apply_placement(placement);
		for (placement, mut next, _) in iter {
			next.apply_placement(placement);
			merged.merge(next);
		}
		plants.push(TuftGrovePlant { placement: Placement::IDENTITY, patch: merged, material });
	}
	plants
}

/// Quantize one placement's params to a unit archetype and world placement scale.
pub fn unit_plant_from_params(
	params: chico_sbs_trees::TuftPatchParams,
	variant: u32,
	world_position: Vec3,
	placed_scale: f32,
	material: MaterialRef,
) -> (Placement, TuftPatch, MaterialRef) {
	let (unit_params, world_size) = params.into_unit_from_num(variant);
	let placement = Placement::new(world_position, 0.0)
		.with_scale(Vec3::splat((placed_scale * world_size).max(1e-4)));
	(placement, unit_params.build(), material)
}

impl TuftGroveBody {
	pub fn from_plants(
		plants: Vec<TuftGrovePlant>,
		extent: &GroveExtent,
		cell_extent_xz: Vec2,
		proxy: TuftGroveProxyHeights,
	) -> Self {
		let span = extent.max() - extent.min();
		let half = span * 0.5;
		let footprint_radius = half.x.max(half.z).max(1.0);
		Self {
			plants,
			structural_center: extent.min() + Vec3::new(half.x, half.y.max(1.0), half.z),
			footprint_radius,
			extent: *extent,
			cell_extent_xz,
			proxy,
		}
	}

	fn foliage_nodes_for_plant(
		plant: &TuftGrovePlant,
		level: LodSceneLevel,
	) -> impl Iterator<Item = FoliageNode> + '_ {
		let material = plant.material.clone();
		plant
			.patch
			.foliage_nodes_for_level(level)
			.flatten()
			.into_iter()
			.map(move |mut node| {
				node.placement = plant.placement.compose_child(node.placement);
				node.with_material(material.clone())
			})
	}

	pub fn foliage_high(&self) -> Vec<FoliageNode> {
		self.plants
			.iter()
			.flat_map(|plant| Self::foliage_nodes_for_plant(plant, LodSceneLevel::High))
			.collect()
	}

	pub fn foliage_medium(&self) -> Vec<FoliageNode> {
		self.plants
			.iter()
			.enumerate()
			.filter(|(i, _)| i % MEDIUM_TUFT_STRIDE == 0)
			.flat_map(|(_, plant)| Self::foliage_nodes_for_plant(plant, LodSceneLevel::High))
			.collect()
	}

	pub fn foliage_low(&self) -> Vec<FoliageNode> {
		self.foliage_cell_proxies(LOW_CELL_STRIDE, self.proxy.low)
	}

	fn foliage_cell_proxies(&self, cell_stride: f32, height: f32) -> Vec<FoliageNode> {
		let bin_x = (self.cell_extent_xz.x * cell_stride).max(1e-3);
		let bin_z = (self.cell_extent_xz.y * cell_stride).max(1e-3);
		let origin = self.extent.min();
		let mut bins: HashMap<(i32, i32), (Vec3, f32, u32)> = HashMap::new();
		let samples = surface_samples_from_plants(
			self.plants.iter().map(|p| (&p.placement, &p.patch)),
		);

		for plant in &self.plants {
			let patch = &plant.patch;
			let width = clump_proxy_width(patch);
			for anchor in &patch.anchors {
				let world = plant.placement.compose_child(Placement::new(*anchor, 0.0)).translation;
				let ix = ((world.x - origin.x) / bin_x).floor() as i32;
				let iz = ((world.z - origin.z) / bin_z).floor() as i32;
				let entry = bins.entry((ix, iz)).or_insert((Vec3::ZERO, 0.0, 0));
				entry.0 += world;
				entry.1 += width;
				entry.2 = entry.2.saturating_add(1);
			}
		}

		let material = self.plants.first().map(|p| p.material.clone()).unwrap_or_default();
		let mut runs = Vec::with_capacity(bins.len());
		let normal_eps = bin_x.max(bin_z) * 0.5;
		for ((ix, iz), (sum_pos, sum_width, count)) in bins {
			let n = (count as f32).max(1.0);
			let mean = sum_pos / n;
			let width = (sum_width / n).max(bin_x.max(bin_z) * 0.5) * n.sqrt();
			let cx = origin.x + (ix as f32 + 0.5) * bin_x;
			let cz = origin.z + (iz as f32 + 0.5) * bin_z;
			// Prefer bin center on XZ; keep plant mean Y so proxies sit on terrain.
			let base_xz = Vec3::new(cx, 0.0, cz).lerp(Vec3::new(mean.x, 0.0, mean.z), 0.35);
			let base = Vec3::new(base_xz.x, mean.y, base_xz.z);
			let up = surface_normal_at(&samples, base.x, base.z, normal_eps);
			if let Some(run) = upright_proxy_run(base, up, width, height) {
				runs.push(run);
			}
		}
		collection_nodes(runs, self.structural_center, self.footprint_radius, material)
	}

	pub fn foliage_ultra_low(&self) -> Vec<FoliageNode> {
		let material = self.plants.first().map(|p| p.material.clone()).unwrap_or_default();
		let samples = surface_samples_from_plants(
			self.plants.iter().map(|p| (&p.placement, &p.patch)),
		);
		horizontal_grid_proxy_placements(&self.extent, ULTRA_GRID, self.proxy.ultra, &samples)
			.into_iter()
			.map(|placement| {
				FoliageNode::straight_frond_segment(placement).with_material(material.clone())
			})
			.collect()
	}

	pub fn foliage_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let nodes = match level {
			LodSceneLevel::High => self.foliage_high(),
			LodSceneLevel::Medium => self.foliage_medium(),
			LodSceneLevel::Low => self.foliage_low(),
			LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
				self.foliage_ultra_low()
			}
		};
		Layers::from_free(nodes)
	}

	pub fn structural_lod(&self) -> StructuralLod {
		StructuralLod::new(self.structural_center, self.footprint_radius)
			.with_factors(
				TUFT_GROVE_STRUCTURAL_HIGH_FACTOR,
				TUFT_GROVE_STRUCTURAL_MEDIUM_FACTOR,
				TUFT_GROVE_STRUCTURAL_LOW_FACTOR,
			)
			.with_preserve_ultra_low(true)
	}
}

fn clump_proxy_width(patch: &TuftPatch) -> f32 {
	let n = patch.clump_count.max(1) as f32;
	if patch.patch_extent_xz > 1e-3 {
		(patch.patch_extent_xz / n.sqrt()).max(0.5)
	} else {
		1.2
	}
}

/// World-space plant roots / anchors used to lift and tilt Low / UltraLow proxies.
pub(crate) fn surface_samples_from_plants<'a>(
	plants: impl Iterator<Item = (&'a Placement, &'a TuftPatch)>,
) -> Vec<Vec3> {
	let mut out = Vec::new();
	for (placement, patch) in plants {
		out.push(placement.translation);
		for anchor in &patch.anchors {
			out.push(placement.compose_child(Placement::new(*anchor, 0.0)).translation);
		}
	}
	out
}

/// Inverse-distance height from surface samples (empty → 0).
pub(crate) fn surface_height_at(samples: &[Vec3], x: f32, z: f32) -> f32 {
	if samples.is_empty() {
		return 0.0;
	}
	let mut w_sum = 0.0;
	let mut h_sum = 0.0;
	for p in samples {
		let dx = p.x - x;
		let dz = p.z - z;
		let w = 1.0 / (dx * dx + dz * dz).max(1e-2);
		w_sum += w;
		h_sum += w * p.y;
	}
	h_sum / w_sum
}

/// Unit surface normal from a heightfield finite difference on [`surface_height_at`].
pub(crate) fn surface_normal_at(samples: &[Vec3], x: f32, z: f32, eps: f32) -> Vec3 {
	if samples.is_empty() {
		return Vec3::Y;
	}
	let eps = eps.max(1e-2);
	let h = surface_height_at(samples, x, z);
	let dhdx = (surface_height_at(samples, x + eps, z) - h) / eps;
	let dhdz = (surface_height_at(samples, x, z + eps) - h) / eps;
	let n = Vec3::new(-dhdx, 1.0, -dhdz);
	let len = n.length();
	if len < 1e-8 {
		Vec3::Y
	} else {
		n / len
	}
}

/// Carpet basis matching flat `from_cols(Z, X, Y)` when `normal == Y`.
fn carpet_basis(normal: Vec3) -> Mat3 {
	let n = {
		let len = normal.length();
		if len < 1e-8 {
			Vec3::Y
		} else {
			normal / len
		}
	};
	let mut tangent = Vec3::X - n * n.dot(Vec3::X);
	if tangent.length_squared() < 1e-8 {
		tangent = Vec3::Z - n * n.dot(Vec3::Z);
	}
	let tangent = tangent.normalize();
	let bitangent = tangent.cross(n).normalize();
	Mat3::from_cols(bitangent, tangent, n)
}

/// Upright Low proxy: base on the surface, rachis along `up` (terrain normal).
pub(crate) fn upright_proxy_run(
	base: Vec3,
	up: Vec3,
	width: f32,
	height: f32,
) -> Option<FrondRun> {
	Placement::frond_segment(base, up, height, width.max(1e-3))
		.map(|p| FrondRun::from_placements([p]))
}

/// Slope-aligned XZ carpet tiles: rachis along projected +X, thin along surface normal.
///
/// Do not use [`Placement::frond_segment`] with `dir = X` and cell-sized `width` —
/// that path leaves kit width near world up and turns carpets into walls.
pub(crate) fn horizontal_grid_proxy_placements(
	extent: &GroveExtent,
	divisions: u32,
	height: f32,
	samples: &[Vec3],
) -> Vec<Placement> {
	let divisions = divisions.max(1);
	let min = extent.min();
	let max = extent.max();
	let span = max - min;
	let cell_x = (span.x / divisions as f32).max(1e-3);
	let cell_z = (span.z / divisions as f32).max(1e-3);
	let scale_x = (cell_z * 0.5 / FROND_KIT_HALF_X).max(1e-4);
	let scale_z = (ULTRA_CARPET_THICKNESS / FROND_KIT_HALF_X).max(1e-4);
	let lift = height * 0.5;
	let normal_eps = cell_x.max(cell_z) * 0.5;
	let mut out = Vec::with_capacity((divisions * divisions) as usize);
	for ix in 0..divisions {
		for iz in 0..divisions {
			let x0 = min.x + ix as f32 * cell_x;
			let z0 = min.z + iz as f32 * cell_z;
			let cz = z0 + cell_z * 0.5;
			let ground_y = surface_height_at(samples, x0, cz);
			let normal = surface_normal_at(samples, x0, cz, normal_eps);
			let rotation = Quat::from_mat3(&carpet_basis(normal));
			let origin = Vec3::new(x0, ground_y, cz) + normal * lift;
			out.push(
				Placement::new(origin, 0.0)
					.with_rotation(rotation)
					.with_scale(Vec3::new(scale_x, cell_x, scale_z)),
			);
		}
	}
	out
}

fn collection_nodes(
	runs: Vec<FrondRun>,
	center: Vec3,
	radius: f32,
	material: MaterialRef,
) -> Vec<FoliageNode> {
	if runs.is_empty() {
		return Vec::new();
	}
	vec![FoliageNode::frond_collection(
		FrondCollection::new(runs).with_probe(center, radius),
		Placement::IDENTITY,
	)
	.with_material(material)]
}

/// Empty sticks + body foliage — standard tuft-grove VegetationComponents impl body.
pub fn tuft_grove_stick_nodes(_level: LodSceneLevel) -> Layers<StickNode> {
	Layers::new()
}

/// Helper used by Params defaults for flat terrain (elevation left to clap / Default).
pub type TuftTerrain = FlatTerrainSample;

/// Grow helper for a cell that yields [`chico_sbs_trees::TuftPatchParams`] + palette.
pub fn grow_placed_tuft_params<C, F>(
	placements: &[GroveCellVariant<C>],
	foliage_noise: NoiseParams,
	merge_collections: usize,
	patch_variants: u32,
	mut params_for: F,
) -> Vec<TuftGrovePlant>
where
	C: Copy,
	F: FnMut(C, NoiseParams) -> (chico_sbs_trees::TuftPatchParams, PaletteMix),
{
	let variants = patch_variants.max(1);
	let grown: Vec<(Placement, TuftPatch, MaterialRef)> = placements
		.iter()
		.map(|placed| {
			let variant = patch_variant_index(placed.position, variants);
			let noise = variant_noise(foliage_noise, variant);
			let (params, mix) = params_for(placed.variant, noise);
			let material = material_from_palette(mix, placed.position, foliage_noise);
			unit_plant_from_params(params, variant, placed.position, placed.scale, material)
		})
		.collect();
	grow_tuft_plants(grown, merge_collections)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn surface_normal_tilts_against_ramp_along_x() {
		// Plane y = 0.5 x → normal ∝ (-0.5, 1, 0).
		let samples: Vec<Vec3> = (0..21)
			.map(|i| {
				let x = i as f32 - 10.0;
				Vec3::new(x, 0.5 * x, 0.0)
			})
			.collect();
		let n = surface_normal_at(&samples, 0.0, 0.0, 1.0);
		assert!(n.x < -0.3, "expected uphill tilt, got {n:?}");
		assert!(n.y > 0.8, "expected mostly upright, got {n:?}");
		assert!(n.z.abs() < 0.05, "expected no Z lean, got {n:?}");
	}

	#[test]
	fn ultra_carpets_follow_plant_heightfield() {
		let extent = GroveExtent::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(20.0, 1.0, 20.0));
		let samples: Vec<Vec3> = (0..11)
			.flat_map(|ix| {
				(0..11).map(move |iz| {
					let x = ix as f32 * 2.0;
					let z = iz as f32 * 2.0;
					Vec3::new(x, 0.25 * x + 5.0, z)
				})
			})
			.collect();
		let placements = horizontal_grid_proxy_placements(&extent, 2, 0.6, &samples);
		assert_eq!(placements.len(), 4);
		for p in &placements {
			assert!(
				p.translation.y > 4.0,
				"carpet should sit near plant heights, got {}",
				p.translation.y
			);
			let up = p.rotation() * Vec3::Z;
			assert!(
				up.x < -0.1,
				"carpet thickness axis should tilt with slope, got {up:?}"
			);
			assert!(up.y > 0.85, "carpet up should stay mostly upright, got {up:?}");
			assert!(
				(up.dot(Vec3::Y) - 1.0).abs() > 1e-3,
				"carpet should not be world-horizontal on a slope"
			);
		}
	}
}
