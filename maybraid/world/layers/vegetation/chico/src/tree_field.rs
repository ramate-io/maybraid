//! GPU forest visuals: every grove kind becomes trees, bushes, or grass.
//!
//! World-route kit spawners (grep, 2026-11-22):
//!
//! Converted — `GrownGrove` → [`crate::ChicoGroveHost`] (`hcsg.rs`) is the only forest
//! present path. Every [`crate::ForestGroveKind`] claims a GPU field from existing
//! roots. GPU CPU [`lod::scene::LodScene`] is a single High band that only culls
//! leftover Mid/Low/far roots. Realizing High admits one catalog entry
//! (`TreeGrove` / `BushPatch` / `GrassPatch`); leaving High retires it. Kit
//! chunks stay empty, so nested `StickNode` / `FoliageNode` kit meshes never
//! spawn on the GPU arm. Stream in/out is still the 1 km
//! [`crate::GROVE_PRESENT_RADIUS_M`] present ring.
//!
//! Intentionally out of scope — not grove tiles, so they keep their own meshes:
//! - `BumpOutPresentationPlugin` `CanopyBumpOut` / `MediumCanopyBumpOut`
//!   (1–5 km cheap-ball proxies in the grove-fill hole)
//! - Isolated `/show` `flattened_plant_host!` plants
//! - Typed grove `LodScene` hosts (`gimme_host!(Orchard)` and siblings) used by
//!   `/show orchard`, not hopscotch streaming
//! - Terrain turf / POM ground cover

#[cfg(test)]
mod authored;
#[cfg(test)]
mod far_bands;
#[cfg(test)]
mod parity;
mod visuals;

use bevy::prelude::*;
use bevy::render::renderer::{RenderAdapter, RenderDevice};
use bevy::render::RenderApp;
use vegetation_bush_field::BushFieldPlugin;
use vegetation_grass_field::GrassFieldPlugin;
use vegetation_groves::temperate_lower_massives::{
	configure_storybook_field_area, scout_storybook_field_tile, StorybookFieldArea,
};
use vegetation_tree_field::{
	requested_world_tree_mode, supports_world_tree_field, TreeFieldPlugin, TreeFieldSettings,
	BRANCH_DESCRIPTOR_STRIDE,
};

use crate::{ForestGroveTile, ForestLayer};

pub use visuals::{GpuFieldKind, GroveGpuVisuals};

const OWNER_DESCRIPTOR_CAPACITY: u32 = 262_144;

fn owner_settings() -> TreeFieldSettings {
	TreeFieldSettings {
		max_plants: 512,
		max_branches: OWNER_DESCRIPTOR_CAPACITY,
		max_leaves: OWNER_DESCRIPTOR_CAPACITY,
		max_per_lod: OWNER_DESCRIPTOR_CAPACITY,
		lod: vegetation_shaders::wesl::LodThresholds::grove_tree(),
		diagnostics: false,
		..default()
	}
}

impl GroveGpuVisuals {
	/// Admission is atomic: rejected content keeps its existing CPU visual path.
	pub fn for_tile(tile: &ForestGroveTile, layer: ForestLayer) -> Self {
		let visuals = Self::claim(tile, layer, &owner_settings());
		if visuals.replaces_kits() {
			info!(
				target: "maybraid.tree_field",
				?layer,
				kind = ?Self::field_kind(tile, layer),
				sites = tile.plant_sites().len(),
				trees = visuals.trees.as_ref().map(|grove| grove.0.plants.len()).unwrap_or(0),
				bushes = visuals.bushes.as_ref().map(|patch| patch.samples.len()).unwrap_or(0),
				grass = visuals.grass.as_ref().map(|patch| patch.samples.len()).unwrap_or(0),
				"grove GPU visuals admitted"
			);
		}
		visuals
	}
}

#[derive(Resource)]
struct GpuForestTrial {
	tile: Option<Vec2>,
	area: StorybookFieldArea,
}

pub(super) fn register_world_tree_field(app: &mut App) {
	let mode = requested_world_tree_mode();
	if mode.is_none() && std::env::var("MAYBRAID_EXPERIMENT_TREE_FIELD_SCOUT").as_deref() == Ok("1")
	{
		scout_storybook_field_tile();
		return;
	}
	let tile = std::env::var("MAYBRAID_EXPERIMENT_TREE_FIELD_TILE")
		.ok()
		.as_deref()
		.and_then(parse_tile);
	let area = StorybookFieldArea::from_env().unwrap_or(StorybookFieldArea::Neighborhood);
	if mode.is_none() {
		if let Some(tile) = tile {
			configure_storybook_field_area(tile, area, false);
		}
		return;
	}
	if std::env::var("MAYBRAID_SHADOWS").as_deref() != Ok("off")
		|| app.get_sub_app(RenderApp).is_none()
	{
		warn!("GPU forest trial requires a renderer and shadows off; keeping grove kits");
		return;
	}
	app.add_plugins(TreeFieldPlugin)
		.add_plugins(BushFieldPlugin)
		.add_plugins(GrassFieldPlugin)
		.insert_resource(GpuForestTrial { tile, area })
		.add_systems(Startup, admit_gpu_forest);
}

fn parse_tile(raw: &str) -> Option<Vec2> {
	let (x, z) = raw.split_once(',')?;
	let tile = Vec2::new(x.trim().parse().ok()?, z.trim().parse().ok()?);
	tile.is_finite().then_some(tile)
}

fn admit_gpu_forest(
	trial: Res<GpuForestTrial>,
	adapter: Option<Res<RenderAdapter>>,
	device: Option<Res<RenderDevice>>,
	mut settings: ResMut<TreeFieldSettings>,
) {
	let supported = adapter.zip(device).is_some_and(|(adapter, device)| {
		supports_world_tree_field(&adapter, &device, trial.area)
			&& device.limits().max_storage_buffer_binding_size
				>= u64::from(OWNER_DESCRIPTOR_CAPACITY) * BRANCH_DESCRIPTOR_STRIDE
			&& device.limits().max_buffer_size
				>= u64::from(OWNER_DESCRIPTOR_CAPACITY) * BRANCH_DESCRIPTOR_STRIDE
			&& device.limits().max_compute_workgroups_per_dimension
				>= OWNER_DESCRIPTOR_CAPACITY.div_ceil(64)
	});
	if !supported {
		warn!("GPU forest trial is unavailable; keeping grove kits");
		GroveGpuVisuals::admit(false);
		return;
	}
	if let Some(tile) = trial.tile {
		let _ = configure_storybook_field_area(tile, trial.area, true);
	}
	GroveGpuVisuals::admit(true);
	*settings = owner_settings();
	info!(
		target: "maybraid.tree_field",
		tile = ?trial.tile, area = ?trial.area, capacity = settings.max_plants,
		"GPU forest visuals admitted for every grove kind"
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::forest_world_sample;
	use crate::{ForestGroveKind, ForestGroveRecipe, ForestLayer};
	use vegetation_groves::GroveExtent;

	#[test]
	fn exact_finite_tile_coordinate_is_required() {
		assert_eq!(parse_tile("1746.8, 1037.9"), Some(Vec2::new(1746.8, 1037.9)));
		assert!(parse_tile("1,2,3").is_none());
		assert!(parse_tile("1,inf").is_none());
	}

	fn grow(kind: ForestGroveKind) -> ForestGroveTile {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		ForestGroveRecipe::uniform(kind, extent).grow(&forest_world_sample())
	}

	fn lod_at(translation: Vec3) -> (Transform, bevy::shape::Aabb3d) {
		(
			Transform::from_translation(translation),
			bevy::shape::Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE),
		)
	}

	fn with_lod<R>(translation: Vec3, f: impl FnOnce(&lod::lod_ref::LodRef<'_>) -> R) -> R {
		use lod::lod_ref::LodRef;
		let (xf, bounds) = lod_at(translation);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &xf,
			current_transform: &xf,
			bounds: &bounds,
		};
		f(&lod_ref)
	}

	fn assert_empty_kit_chunks(host: &crate::ChicoGroveHost) {
		use lod::scene::LodScene;
		use lod::scene::LodSceneLevel;
		with_lod(Vec3::new(0.0, 20.0, 40.0), |lod_ref| {
			for level in [
				LodSceneLevel::High,
				LodSceneLevel::Medium,
				LodSceneLevel::Low,
				LodSceneLevel::UltraLow,
			] {
				let chunks = host.scene_chunks_with_level(lod_ref, level);
				assert_eq!(
					chunks.total_primitives(),
					1,
					"GPU host must not emit kit chunks at {level:?}"
				);
			}
			assert_eq!(host.scene_lod_level(lod_ref), LodSceneLevel::High);
			assert_eq!(host.scene_lod_level_from_levels(&[]), LodSceneLevel::High);
		});
		with_lod(Vec3::new(0.0, 20.0, crate::GROVE_PRESENT_RADIUS_M + 500.0), |lod_ref| {
			assert_eq!(host.scene_lod_level(lod_ref), LodSceneLevel::High);
			assert!(!host
				.scene_lod_culls(lod_ref, LodSceneLevel::High)
				.should_cull(LodSceneLevel::High));
			assert!(host
				.scene_lod_culls(lod_ref, LodSceneLevel::High)
				.should_cull(LodSceneLevel::UltraLow));
		});
	}

	#[test]
	fn gpu_claim_swaps_every_grove_kind_and_drops_kit_chunks() {
		use crate::ChicoGroveHost;
		use lod::scene::LodSceneLevel;

		let _lock = GroveGpuVisuals::lock_admission();
		GroveGpuVisuals::admit(true);
		assert_eq!(ForestGroveKind::ALL.len(), 41);

		let mut swapped = 0usize;
		let mut empty = 0usize;
		for kind in ForestGroveKind::ALL {
			let tile = grow(*kind);
			let sites = tile.plant_sites();
			let layer = kind.gpu_layer();
			let visuals = GroveGpuVisuals::claim(&tile, layer, &owner_settings());
			if sites.is_empty() {
				empty += 1;
				assert!(!visuals.replaces_kits(), "{kind:?} with no sites must keep kits");
				continue;
			}
			assert!(visuals.replaces_kits(), "{kind:?} on {layer:?} must swap");
			match GroveGpuVisuals::field_kind(&tile, layer) {
				GpuFieldKind::Tree => {
					assert_eq!(
						visuals.trees.as_ref().map(|grove| grove.0.plants.len()).unwrap_or(0),
						sites.len().min(owner_settings().max_plants as usize),
						"{kind:?} tree count"
					);
					assert!(visuals.bushes.is_none());
					assert!(visuals.grass.is_none());
				}
				GpuFieldKind::Bush => {
					assert_eq!(
						visuals.bushes.as_ref().map(|patch| patch.samples.len()).unwrap_or(0),
						sites.len().min(4_096),
						"{kind:?} bush count"
					);
					assert!(visuals.trees.is_none());
				}
				GpuFieldKind::Grass => {
					assert_eq!(
						visuals.grass.as_ref().map(|patch| patch.samples.len()).unwrap_or(0),
						sites.len(),
						"{kind:?} grass count"
					);
					assert!(visuals.trees.is_none());
				}
			}
			let host = ChicoGroveHost::new(tile, layer);
			assert!(host.gpu_visuals().replaces_kits(), "{kind:?} host");
			assert_empty_kit_chunks(&host);
			swapped += 1;
		}
		assert!(
			swapped >= 35,
			"expected most grove kinds to place plants, swapped={swapped} empty={empty}"
		);

		let trees = grow(ForestGroveKind::TemperateLowerMassives);
		let tree_sites = trees.plant_sites();
		assert!(tree_sites.len() > 1, "TLM should place oak, storybook, and rory");
		let heights: Vec<f32> = tree_sites.iter().map(|site| site.height_m).collect();
		assert!(
			heights.iter().copied().max_by(f32::total_cmp).unwrap()
				- heights.iter().copied().min_by(f32::total_cmp).unwrap()
				> 1.0,
			"storybook scales should vary with authored heights, got {heights:?}"
		);

		GroveGpuVisuals::admit(false);
		assert!(!GroveGpuVisuals::claim(&trees, ForestLayer::LowerCanopy, &owner_settings())
			.replaces_kits());
		with_lod(Vec3::new(0.0, 20.0, 40.0), |lod_ref| {
			assert!(trees.scene_chunks_with_level(lod_ref, LodSceneLevel::High).total_weight() > 1);
		});
	}

	#[test]
	fn world_route_neighborhood_has_no_kit_chunks_on_gpu() {
		use crate::ChicoGroveHost;
		use lod::scene::LodScene;
		use lod::scene::LodSceneLevel;
		use vegetation_groves::GroveExtent;

		let _lock = GroveGpuVisuals::lock_admission();
		GroveGpuVisuals::admit(true);
		let anchor = Vec3::new(-4950.0, 0.0, 3150.0);
		let stacked = [
			(ForestGroveKind::TallGrass, ForestLayer::Tufts),
			(ForestGroveKind::LowBush, ForestLayer::Understory),
			(ForestGroveKind::TemperateLowerMassives, ForestLayer::LowerCanopy),
		];
		let mut hosts = 0usize;
		let mut trees = 0usize;
		let mut bushes = 0usize;
		let mut grass = 0usize;
		let mut kit_high = 0u32;
		for x in -1..=1 {
			for z in -1..=1 {
				let origin = anchor + Vec3::new(x as f32 * 100.0, 0.0, z as f32 * 100.0);
				let extent = GroveExtent::new(origin, origin + Vec3::new(100.0, 1.0, 100.0));
				for (kind, layer) in stacked {
					let tile =
						ForestGroveRecipe::uniform(kind, extent).grow(&forest_world_sample());
					let host = ChicoGroveHost::new(tile, layer);
					assert!(
						host.gpu_visuals().replaces_kits() || host.tile.plant_sites().is_empty(),
						"{kind:?} at {origin}"
					);
					if host.gpu_visuals().replaces_kits() {
						assert_empty_kit_chunks(&host);
					}
					hosts += 1;
					trees += host
						.gpu_visuals()
						.trees
						.as_ref()
						.map(|grove| grove.0.plants.len())
						.unwrap_or(0);
					bushes += host
						.gpu_visuals()
						.bushes
						.as_ref()
						.map(|patch| patch.samples.len())
						.unwrap_or(0);
					grass += host
						.gpu_visuals()
						.grass
						.as_ref()
						.map(|patch| patch.samples.len())
						.unwrap_or(0);
					kit_high += with_lod(origin + Vec3::new(50.0, 20.0, 80.0), |lod_ref| {
						host.scene_chunks_with_level(lod_ref, LodSceneLevel::High).total_weight()
					});
				}
			}
		}
		assert_eq!(hosts, 27);
		assert!(trees > 0, "world-route canopy must admit storybook trees");
		assert!(bushes > 0, "world-route understory must admit bushes");
		assert!(grass > 0, "world-route tufts must admit grass");
		assert_eq!(kit_high, hosts as u32, "each GPU host High band is one catalog primitive");
		eprintln!(
			"world-route GPU census: hosts={hosts} trees={trees} bushes={bushes} grass={grass} kit_high_weight={kit_high}"
		);
		GroveGpuVisuals::admit(false);
	}
}
