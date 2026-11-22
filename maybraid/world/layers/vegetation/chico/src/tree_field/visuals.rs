//! GPU forest recomposition: trees, bushes, and grass at existing grove roots.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};

use bevy::prelude::*;
use lod::scene::{LodSceneCull, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use vegetation_bush_field::{BushPatch, BushRecipe};
use vegetation_grass_field::GrassPatch;
use vegetation_groves::GrovePlantSite;
use vegetation_tree_field::{TreeField, TreeFieldSettings, TreeGrove, TreePatch, TreeRecipe};

use crate::{ForestGroveTile, ForestLayer};

static GPU_FOREST_VISUALS: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static ADMIT_LOCK: Mutex<()> = Mutex::new(());

/// Persistent GPU visuals that replace grove kit meshes when the experiment is on.
#[derive(Clone, Default)]
pub struct GroveGpuVisuals {
	pub trees: Option<TreeGrove>,
	pub bushes: Option<BushPatch>,
	pub grass: Option<GrassPatch>,
}

impl GroveGpuVisuals {
	pub fn admit(admitted: bool) {
		GPU_FOREST_VISUALS.store(admitted, Ordering::Relaxed);
	}

	pub fn admitted() -> bool {
		GPU_FOREST_VISUALS.load(Ordering::Relaxed)
	}

	/// Hold this while flipping admission so parallel tests do not race.
	#[cfg(test)]
	pub fn lock_admission() -> MutexGuard<'static, ()> {
		ADMIT_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
	}

	pub fn replaces_kits(&self) -> bool {
		self.trees.is_some() || self.bushes.is_some() || self.grass.is_some()
	}

	/// GPU shaders own vegetation LOD. CPU presentation is a single High band.
	pub fn cpu_scene_lod_level(&self) -> Option<LodSceneLevel> {
		self.replaces_kits().then_some(LodSceneLevel::High)
	}

	/// No Mid/Low/far CPU band swaps while GPU fields stay nested at High.
	pub fn cpu_scene_lod_status(&self) -> Option<LodSceneStatus> {
		self.replaces_kits().then_some(LodSceneStatus::Unchanged)
	}

	/// Cull leftover non-High roots. Host stream-out is the 1 km present ring.
	pub fn cpu_scene_lod_culls(&self) -> Option<LodSceneCulls> {
		self.replaces_kits().then_some(Self::cpu_high_only_culls())
	}

	pub fn cpu_high_only_culls() -> LodSceneCulls {
		LodSceneCulls::AllOf(vec![
			LodSceneCull::Level(LodSceneLevel::Medium),
			LodSceneCull::Level(LodSceneLevel::Low),
			LodSceneCull::Level(LodSceneLevel::UltraLow),
		])
		.with_customs()
	}

	pub fn claim(tile: &ForestGroveTile, layer: ForestLayer, settings: &TreeFieldSettings) -> Self {
		if !Self::admitted() {
			return Self::default();
		}
		let sites = tile.plant_sites();
		if sites.is_empty() {
			return Self::default();
		}
		let seed = TreePatch::seed_from_origin(sites[0].root);
		match Self::field_kind(tile, layer) {
			GpuFieldKind::Tree => {
				Self { trees: Self::tree_grove(&sites, seed, settings), ..Self::default() }
			}
			GpuFieldKind::Bush => {
				Self { bushes: Some(Self::bush_patch(&sites, seed)), ..Self::default() }
			}
			GpuFieldKind::Grass => Self {
				grass: Some(
					GrassPatch::from_oriented_height_sites(
						seed,
						sites.iter().copied().map(GrovePlantSite::oriented_height),
					)
					.with_grove_residency(),
				),
				..Self::default()
			},
		}
	}

	/// Tuft tiles are always grass. Other plants follow the stacked forest layer:
	/// understory and leftover tuft-layer woody become bushes; canopy becomes trees.
	pub fn field_kind(tile: &ForestGroveTile, layer: ForestLayer) -> GpuFieldKind {
		if tile.is_tuft() {
			return GpuFieldKind::Grass;
		}
		match layer {
			ForestLayer::Tufts | ForestLayer::Understory => GpuFieldKind::Bush,
			ForestLayer::LowerCanopy | ForestLayer::UpperCanopy => GpuFieldKind::Tree,
		}
	}

	fn tree_grove(
		sites: &[GrovePlantSite],
		seed: u32,
		settings: &TreeFieldSettings,
	) -> Option<TreeGrove> {
		let recipe = TreeRecipe::storybook().pin_authored_size();
		let patch = TreePatch::from_oriented_height_sites(
			seed,
			&Self::capped_sites(sites, settings.max_plants),
			recipe,
		);
		match TreeField::from_plants(patch.plants(), settings) {
			Ok(field) => Some(TreeGrove(Arc::new(field))),
			Err(reason) => {
				warn!(target: "maybraid.tree_field", reason, "grove retained legacy visuals");
				None
			}
		}
	}

	fn bush_patch(sites: &[GrovePlantSite], seed: u32) -> BushPatch {
		let recipe = BushRecipe::small_scrub().pin_authored_size();
		BushPatch::from_oriented_height_sites(seed, &Self::capped_sites(sites, 4_096), recipe)
			.with_grove_residency()
	}

	fn capped_sites(sites: &[GrovePlantSite], max_plants: u32) -> Vec<(Vec3, Vec3, f32)> {
		sites
			.iter()
			.take(max_plants as usize)
			.map(|site| site.oriented_height())
			.collect()
	}
}

/// Which GPU field a forest layer uses. Large canopies become storybook trees;
/// understory becomes bushes; tufts become grass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuFieldKind {
	Tree,
	Bush,
	Grass,
}

impl ForestLayer {
	pub fn gpu_field_kind(self) -> GpuFieldKind {
		match self {
			Self::Tufts => GpuFieldKind::Grass,
			Self::Understory => GpuFieldKind::Bush,
			Self::LowerCanopy | Self::UpperCanopy => GpuFieldKind::Tree,
		}
	}
}
