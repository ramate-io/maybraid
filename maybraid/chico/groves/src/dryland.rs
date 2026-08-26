//! Dryland — very-low-density arid upper-canopy grove with Liam's Conifer and Vase Tree
//! ([RFC-183 §3.4.7.13], [#335](https://github.com/ramate-io/maybraid/issues/335)).
//!
//! Sparse dry highland canopy with evenly common Liam's Conifer and Vase Tree forms. Forest-layer
//! attachment remains a follow-up.

use bevy_math::Vec2;
use procedural_common::UnitRange;

pub mod variants;

use crate::grove::{
	GroveBucket, GroveDefinition, GroveDistribution, GrovePlacementRanges, PaletteMix, PaletteSlot,
	PlacementConstraints,
};

/// Sparse sampled canopy-density band ([`0.0`, `0.35`]).
const SPARSE_CANOPY_DENSITY: UnitRange = UnitRange::new(0.0, 0.35);

/// Authored Dryland grove definition.
///
/// Cell footprint sits at the RFC midpoint (`35.0` m). The offset range is signed and ± one cell so
/// placements break the underlying grid instead of clustering near cell centers.
pub fn definition() -> GroveDefinition<DrylandCell> {
	GroveDefinition {
		cell_extent_xz: Vec2::splat(35.0),
		placement: GrovePlacementRanges::new(
			UnitRange::new(0.85, 1.15),
			UnitRange::new(-35.0, 35.0),
		),
		distribution: DrylandCell::distribution(),
	}
}

/// Ordered dryland varietals ([RFC-183 §3.4.7.13]); the explicit `None` bucket lives only in the
/// distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrylandCell {
	DrylandLiamsConifer,
	DrylandVaseTree,
}

/// Typed authored geometry for one dryland varietal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DrylandItem {
	LiamsConifer(&'static DrylandLiamsConifer),
	VaseTree(&'static DrylandVaseTree),
}

/// Authored geometry ranges for one dry Liam's Conifer form.
#[derive(Debug, Clone, PartialEq)]
pub struct DrylandLiamsConifer {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_density: UnitRange,
}

/// Authored geometry ranges for one dry Vase Tree form.
#[derive(Debug, Clone, PartialEq)]
pub struct DrylandVaseTree {
	pub height: UnitRange,
	pub stalk_radius: UnitRange,
	pub canopy_spread: UnitRange,
	pub canopy_density: UnitRange,
}

const DRYLAND_LIAMS: DrylandLiamsConifer = DrylandLiamsConifer {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.25, 0.50),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const DRYLAND_VASE: DrylandVaseTree = DrylandVaseTree {
	height: UnitRange::new(10.0, 20.0),
	stalk_radius: UnitRange::new(0.34, 0.68),
	canopy_spread: UnitRange::new(2.0, 5.5),
	canopy_density: SPARSE_CANOPY_DENSITY,
};

const DRYLAND_LIAMS_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("dry_conifer_bark", "tan_bark"),
	PaletteSlot::new("gray_brown", "dark_bark"),
]);

const DRYLAND_LIAMS_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sage_green", "dusty_green"),
	PaletteSlot::new("deep_green", "olive_green"),
]);

const DRYLAND_VASE_STICK_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("sun_baked_bark", "tan_bark"),
	PaletteSlot::new("red_brown", "gray_brown"),
]);

const DRYLAND_VASE_CANOPY_MIX: PaletteMix = PaletteMix::new(&[
	PaletteSlot::new("olive_green", "dusty_green"),
	PaletteSlot::new("yellow_green", "dry_green"),
]);

impl DrylandCell {
	/// Authored ordered distribution: explicit `None`, then variants in declaration order.
	///
	/// Placed weights total `2.0`; the `None` weight of `24.7` puts the placed share at
	/// `2.0 / 26.7 ≈ 0.075`, mid RFC `DENSITY_RANGE` (`0.03..0.12`).
	pub fn distribution() -> GroveDistribution<Self> {
		let liams = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.82));
		let vase = PlacementConstraints::new(UnitRange::new(0.0, 1.0), UnitRange::new(0.0, 0.70));
		GroveDistribution::new(vec![
			GroveBucket::none(24.7),
			GroveBucket::placed(1.0, liams, Self::DrylandLiamsConifer),
			GroveBucket::placed(1.0, vase, Self::DrylandVaseTree),
		])
	}

	pub fn item(self) -> DrylandItem {
		match self {
			Self::DrylandLiamsConifer => DrylandItem::LiamsConifer(&DRYLAND_LIAMS),
			Self::DrylandVaseTree => DrylandItem::VaseTree(&DRYLAND_VASE),
		}
	}

	pub fn stick_palette_mix(self) -> PaletteMix {
		match self {
			Self::DrylandLiamsConifer => DRYLAND_LIAMS_STICK_MIX,
			Self::DrylandVaseTree => DRYLAND_VASE_STICK_MIX,
		}
	}

	pub fn canopy_palette_mix(self) -> PaletteMix {
		match self {
			Self::DrylandLiamsConifer => DRYLAND_LIAMS_CANOPY_MIX,
			Self::DrylandVaseTree => DRYLAND_VASE_CANOPY_MIX,
		}
	}
}

#[cfg(feature = "render")]
mod vc {
	use std::sync::Arc;

	
	use bevy::prelude::*;
	use chico_sbs_trees::{
		LiamsConifer, LiamsConiferParams, QuantizedPlant, VaseTree, VaseTreeParams,
	};
	use chico_vegetation_components::{
		Placement, VegetationComponents,
	};
	use clap::Args;
	#[cfg(test)]
	use bevy::math::bounding::Aabb3d;
	#[cfg(test)]
	use lod::gen::{LodScene, LodSceneLevel};
	use lod::lod_ref::LodRef;
	use lod::SceneChunk;
	use material_ref::MaterialRef;
	use procedural_common::NoiseParams;

	use super::{definition, DrylandCell, DrylandItem, DRYLAND_LIAMS, DRYLAND_VASE};
	use crate::grove::vc_tuft::patch_variant_index;
	use crate::grove::{
		canopy_ball_material_from_palette, canopy_proxy_column, canopy_proxy_site, frond_material_from_palette,
		grove_structural_footprint, nest_flattened_plant_chunk, placement_noise,
		remixed_sbs_plant, stick_material_from_palette, CanopyProxySite,
		FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
		WoodyGroveLod,
	};

	pub const DRYLAND_STRUCTURAL_HIGH_FACTOR: f32 = 5.0;
	pub const DRYLAND_STRUCTURAL_MEDIUM_FACTOR: f32 = 10.0;
	pub const DRYLAND_STRUCTURAL_LOW_FACTOR: f32 = 20.0;

	const WOODY_LOD: WoodyGroveLod = WoodyGroveLod::ordinary(
		DRYLAND_STRUCTURAL_HIGH_FACTOR,
		DRYLAND_STRUCTURAL_MEDIUM_FACTOR,
		DRYLAND_STRUCTURAL_LOW_FACTOR,
	);

	#[derive(Clone, Debug, Args)]
	#[command(rename_all = "kebab-case")]
	pub struct DrylandParams {
		#[command(flatten)]
		pub preview: GrovePreviewParams<DrylandCell>,
	}

	impl Default for DrylandParams {
		fn default() -> Self {
			Self {
				preview: GrovePreviewParams::default()
					.with_terrain(FlatTerrainSample { elevation: 0.40, steepness: 0.35 }),
			}
		}
	}

	crate::impl_grove_preview_params!(DrylandParams, DrylandCell);

	impl DrylandParams {
		// preview accessors via impl_grove_preview_params!
		pub fn build(&self) -> Dryland {
			self.build_on(&self.terrain)
		}

		/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
		pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> Dryland {
			Dryland::from_placements(
				&self.placements_on(world),
				self.grove.noise,
				&self.extent,
				self.tree_variants,
			)
		}
	}

	remixed_sbs_plant!(DrylandLiams, LiamsConifer, LiamsConiferParams, DRYLAND_LIAMS);
	remixed_sbs_plant!(DrylandVase, VaseTree, VaseTreeParams, DRYLAND_VASE);

	#[derive(Clone)]
	enum DrylandKind {
		Liams(Arc<LiamsConifer>),
		Vase(Arc<VaseTree>),
	}

	#[derive(Clone)]
	pub struct DrylandPlant {
		pub placement: Placement,
		kind: DrylandKind,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	}

	#[derive(Clone, Component)]
	pub struct Dryland {
		pub plants: Arc<[DrylandPlant]>,
		pub structural_center: Vec3,
		pub footprint_radius: f32,
		pub extent: GroveExtent,
	}

	impl Dryland {
		pub fn from_placements(
			placements: &[GroveCellVariant<DrylandCell>],
			grove_noise: NoiseParams,
			extent: &GroveExtent,
			tree_variants: u32,
		) -> Self {
			let plants: Arc<[DrylandPlant]> = placements
				.iter()
				.map(|placed| grow_plant(placed, grove_noise, tree_variants))
				.collect::<Vec<_>>()
				.into();
			let (structural_center, footprint_radius) = grove_structural_footprint(extent);
			Self { plants, structural_center, footprint_radius, extent: *extent }
		}

		fn nest_plant_chunks(&self, lod_ref: &LodRef) -> Vec<SceneChunk> {
			if self.plants.is_empty() {
				return Vec::new();
			}
			let n = self.plants.len();
			let plants = Arc::clone(&self.plants);
			let prev = *lod_ref.previous_transform;
			let curr = *lod_ref.current_transform;
			let bounds = *lod_ref.bounds;
			let entity = lod_ref.entity;
			let mut index = 0usize;
			vec![SceneChunk::lazy(n as u32, n, move || {
				if index >= plants.len() {
					return None;
				}
				let plant = &plants[index];
				index += 1;
				let plant_lod = LodRef {
					entity,
					previous_transform: &prev,
					current_transform: &curr,
					bounds: &bounds,
				};
				Some(match &plant.kind {
					DrylandKind::Liams(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
					DrylandKind::Vase(t) => nest_flattened_plant_chunk(
						Arc::clone(t),
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
						&plant.frond_material,
						&plant_lod,
					),
				})
			})]
		}

		fn canopy_sites(&self) -> Vec<CanopyProxySite> {
			self.plants
				.iter()
				.filter_map(|plant| {
					let material = &plant.ball_material;
					match &plant.kind {
						DrylandKind::Liams(t) => canopy_proxy_column(t, plant.placement, material),
						DrylandKind::Vase(t) => canopy_proxy_site(t, plant.placement, material),
					}
				})
				.collect()
		}
	}

	fn grow_plant(
		placed: &GroveCellVariant<DrylandCell>,
		grove_noise: NoiseParams,
		tree_variants: u32,
	) -> DrylandPlant {
		let variant = patch_variant_index(placed.position, tree_variants);
		let palette_noise = placement_noise(grove_noise, placed.position);
		let stick_seed = palette_noise.seed;
		let canopy_seed = palette_noise.seed.wrapping_add(31);
		let stick_material =
			stick_material_from_palette(Some(placed.variant.stick_palette_mix()), stick_seed);
		let ball_material = canopy_ball_material_from_palette(
			Some(placed.variant.canopy_palette_mix()),
			canopy_seed,
		);
		let frond_material =
			frond_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed);

		let (kind, world_size) = match placed.variant.item() {
			DrylandItem::LiamsConifer(_) => {
				let (tree, world_size) = DrylandLiams::grow_num(variant);
				(DrylandKind::Liams(tree), world_size)
			}
			DrylandItem::VaseTree(_) => {
				let (tree, world_size) = DrylandVase::grow_num(variant);
				(DrylandKind::Vase(tree), world_size)
			}
		};

		DrylandPlant {
			placement: Placement::new(placed.position, 0.0)
				.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
			kind,
			stick_material,
			ball_material,
			frond_material,
		}
	}

	crate::impl_woody_grove_lod!(Dryland, WOODY_LOD);

	#[cfg(test)]
	mod tests {
		use super::*;
		use anyhow::Result;

		fn small_grove() -> Dryland {
			DrylandParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(220.0, 1.0, 220.0)))
				.build()
		}

		fn plant_height(plant: &DrylandPlant) -> f32 {
			match &plant.kind {
				DrylandKind::Liams(t) => t.geometry.scale.stalk_height,
				DrylandKind::Vase(t) => t.geometry.height(),
			}
		}

		fn plant_seed(plant: &DrylandPlant) -> i32 {
			match &plant.kind {
				DrylandKind::Liams(t) => t.geometry.canopy_noise.seed,
				DrylandKind::Vase(t) => t.geometry.canopy_noise.seed,
			}
		}

		#[test]
		fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
			let grove = small_grove();
			assert!(!grove.plants.is_empty(), "expected placed dryland plants");

			assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::High).len(), 0);
			assert_eq!(grove.foliage_nodes_for_level(LodSceneLevel::High).len(), 0);
			assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::Medium).len(), 0);
			assert_eq!(grove.foliage_nodes_for_level(LodSceneLevel::Medium).len(), 0);

			let camera = Transform::from_translation(Vec3::new(40.0, 2.0, 40.0));
			let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
			let lod_ref = LodRef {
				entity: Entity::PLACEHOLDER,
				previous_transform: &camera,
				current_transform: &camera,
				bounds: &bounds,
			};
			let high = grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::High);
			let lod::SceneChunk::SubChunks(parts) = high else {
				anyhow::bail!("High dryland should wrap plant chunks");
			};
			assert_eq!(parts.len(), 1, "expected one lazy plant producer");
			let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0]
			else {
				anyhow::bail!("High dryland plants should be SceneChunk::Lazy");
			};
			assert_eq!(*remaining_primitives, grove.plants.len());
			assert_eq!(*remaining_weight as usize, grove.plants.len());

			assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::Low).len(), 0);
			let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).len();
			assert_eq!(low_foliage, grove.plants.len());
			assert!(grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len() <= low_foliage);
			let lod::SceneChunk::Primitive { weight, .. } =
				grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low)
			else {
				anyhow::bail!("Low dryland should emit one flattened canopy collection");
			};
			assert_eq!(weight, chico_vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
			Ok(())
		}

		#[test]
		fn tree_variants_quantize_archetypes() -> Result<()> {
			use std::collections::HashSet;

			let mut params = DrylandParams::default()
				.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(220.0, 1.0, 220.0)));
			params.tree_variants = 4;
			let grove = params.build();
			assert!(!grove.plants.is_empty(), "expected placed dryland plants");
			for plant in grove.plants.iter() {
				assert!(
					(plant_height(plant) - 1.0).abs() < 1e-4,
					"expected unit height, got {}",
					plant_height(plant)
				);
			}
			let seeds: HashSet<i32> = grove.plants.iter().map(plant_seed).collect();
			assert!(seeds.len() <= 4, "expected ≤4 unique unit seeds, got {}", seeds.len());
			Ok(())
		}
	}
}

#[cfg(feature = "render")]
pub use vc::{
	Dryland, DrylandParams, DrylandPlant, DRYLAND_STRUCTURAL_HIGH_FACTOR,
	DRYLAND_STRUCTURAL_LOW_FACTOR, DRYLAND_STRUCTURAL_MEDIUM_FACTOR,
};

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{
		FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent,
	};
	use anyhow::Result;
	use bevy_math::Vec3;
	use gimme_gen::Cell;
	use procedural_common::NoiseParams;

	#[test]
	fn distribution_matches_rfc_order_and_weights() -> Result<()> {
		let dist = DrylandCell::distribution();
		assert_eq!(dist.len(), 3);
		assert!(dist.buckets[0].item.is_none());
		assert_eq!(dist.buckets[0].weight, 24.7);
		assert_eq!(dist.buckets[1].item, Some(DrylandCell::DrylandLiamsConifer));
		assert_eq!(dist.buckets[1].weight, 1.0);
		assert_eq!(dist.buckets[2].item, Some(DrylandCell::DrylandVaseTree));
		assert_eq!(dist.buckets[2].weight, 1.0);
		Ok(())
	}

	#[test]
	fn placed_share_sits_in_rfc_density_range() -> Result<()> {
		let dist = DrylandCell::distribution();
		let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
		let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
		let share = placed / total;
		assert!((0.03..=0.12).contains(&share), "placed share {share} outside RFC density");
		Ok(())
	}

	#[test]
	fn geometry_follows_authored_bands() -> Result<()> {
		let DrylandItem::LiamsConifer(liams) = DrylandCell::DrylandLiamsConifer.item() else {
			anyhow::bail!("expected liams item");
		};
		assert_eq!(liams.height, UnitRange::new(10.0, 20.0));
		assert_eq!(liams.canopy_density, SPARSE_CANOPY_DENSITY);

		let DrylandItem::VaseTree(vase) = DrylandCell::DrylandVaseTree.item() else {
			anyhow::bail!("expected vase item");
		};
		assert_eq!(vase.height, UnitRange::new(10.0, 20.0));
		assert_eq!(vase.canopy_density, SPARSE_CANOPY_DENSITY);
		Ok(())
	}

	#[test]
	fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
		let dist = DrylandCell::distribution();
		for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
			assert_eq!(bucket.constraints.elevation.start, 0.0);
			assert_eq!(bucket.constraints.elevation.end, 1.0);
		}
		let liams = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(DrylandCell::DrylandLiamsConifer))
			.ok_or_else(|| anyhow::anyhow!("missing liams bucket"))?;
		assert_eq!(liams.constraints.steepness.end, 0.82);

		let vase = dist
			.buckets
			.iter()
			.find(|b| b.item == Some(DrylandCell::DrylandVaseTree))
			.ok_or_else(|| anyhow::anyhow!("missing vase bucket"))?;
		assert_eq!(vase.constraints.steepness.end, 0.70);
		Ok(())
	}

	#[test]
	#[ignore = "placement constraints deferred to forest-layer normalization"]
	fn steep_slope_rejects_vase_but_allows_liams() -> Result<()> {
		let prepared =
			DrylandCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
		let moderate = FlatTerrainSample { elevation: 0.40, steepness: 0.55 };
		let vase_outcome = prepared.select_from(
			2,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&moderate,
		);
		match vase_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, DrylandCell::DrylandVaseTree);
			}
			other => anyhow::bail!("expected DrylandVaseTree on moderate slope, got {other:?}"),
		}
		let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.75 };
		let liams_outcome = prepared.select_from(
			1,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&steep,
		);
		match liams_outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, DrylandCell::DrylandLiamsConifer);
			}
			other => anyhow::bail!("expected DrylandLiamsConifer on steep slope, got {other:?}"),
		}
		match prepared.select_from(
			2,
			Vec3::new(5.0, 0.40, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&steep,
		) {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, DrylandCell::DrylandVaseTree);
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
		Ok(())
	}

	#[test]
	fn palette_resolves_for_all_varietals() -> Result<()> {
		for cell in [DrylandCell::DrylandLiamsConifer, DrylandCell::DrylandVaseTree] {
			for (palette, label) in
				[(cell.stick_palette_mix(), "stick"), (cell.canopy_palette_mix(), "canopy")]
			{
				let mut allowed = Vec::new();
				for slot in palette.slots {
					allowed.extend(slot.start.resolve());
					allowed.extend(slot.end.resolve());
				}
				assert!(!allowed.is_empty(), "unresolved {label} tokens for {cell:?}");
			}
		}
		Ok(())
	}

	#[test]
	fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
		let noise = crate::grove::GroveFrontend::default().noise;
		let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(280.0, 1.0, 280.0));
		let terrain = FlatTerrainSample::default();
		let a = grove.populate(&extent, &terrain);
		let b = grove.populate(&extent, &terrain);
		assert_eq!(a, b);
		assert!(!a.is_empty());
		Ok(())
	}
}
