//! Urbanization generate / host glue (forest_stream parallel).
//!
//! Registers LOD generate for [`SelectedUrbanization`] and a present-keep
//! bullseye. Host spawn / cull runs in Update so it can own
//! [`DevelopmentIndex`] without conflicting with the urbanization LOD drain.

use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use durham_terrain::shaders::DurhamTerrainShader;
use durham_terrain_models::PresentedTerrainScene;
use lod::gen::{
	GeneratingSpatialIndex, GenerationScheme, Id, LodGenerateKeepRegion, LodGenerateQueue,
	LodGenerateRegion, OriginalId, SpatialIndex,
};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion, RegionPresenter};
use lod::{LodGenerateBudget, LodSceneRefreshRegionPlugin, LodViewer};
use lod_avian::AvianLodSceneRefreshPlugin;
use richmond_development_models::{
	DevelopmentEntryStore, DevelopmentIndex, PaddedStoreView, PaddedTerrainPresenter,
	PresentedPaddedTerrainScene, TerrainWithPads, UrbanizationHostPlugin,
	UrbanizationPresenterState,
};
use richmond_urbanization::{
	SelectedUrbanization, UrbanizationExtent, UrbanizationGenerateBullseye,
	UrbanizationGenerationPlugin, UrbanizationIndex, UrbanizationKind, UrbanizationLodChan,
	UrbanizationPresentBullseye, UrbanizationPresentationPlugin, DEVELOPMENT_GENERATE_RADIUS_M,
	DEVELOPMENT_PRESENT_RADIUS_M,
};

pub use richmond_development_models::UrbanizationHostBudget;
pub use richmond_urbanization::{
	parse_urbanization_kind, stream_radii_m, UrbanizationStreamSpec, DEFAULT_URBANIZATION_NOISE,
	DEFAULT_URBANIZATION_STREAM_RADIUS,
};

/// Generate + present-keep plugins for [`SelectedUrbanization`].
///
/// Generate / present plugins are `plugins_only` so [`stream_urbanization`] owns
/// bullseye enablement. Hosts come from [`UrbanizationHostPlugin`]. This also
/// registers padded-terrain scene refresh used by the playground.
pub fn register_urbanization_lod(app: &mut App) {
	#[derive(Debug, Clone, Copy, Default)]
	struct PaddedTerrainRefresh;

	if !app.is_plugin_added::<UrbanizationGenerationPlugin>() {
		app.add_plugins(UrbanizationGenerationPlugin::plugins_only());
	}
	if !app.is_plugin_added::<UrbanizationPresentationPlugin>() {
		app.add_plugins(UrbanizationPresentationPlugin::plugins_only());
	}
	if !app.is_plugin_added::<UrbanizationHostPlugin>() {
		app.add_plugins(UrbanizationHostPlugin);
	}
	app.init_resource::<UrbanizationPaddedTerrainBudget>()
		.insert_resource(LodGenerateBudget { ids_per_frame: 8 })
		.add_plugins(LodSceneRefreshRegionPlugin::<
			UrbanizationPresentBullseye,
			With<LodViewer>,
			PaddedTerrainRefresh,
		>::default())
		.add_plugins(AvianLodSceneRefreshPlugin::<
			TerrainWithPads,
			PaddedTerrainRefresh,
			With<LodViewer>,
		>::default());
}

/// Cap pad bake to this many [`TerrainWithPads`] ids per frame.
#[derive(Resource, Clone, Copy, Debug)]
pub struct UrbanizationPaddedTerrainBudget {
	pub ids_per_frame: usize,
}

impl Default for UrbanizationPaddedTerrainBudget {
	fn default() -> Self {
		Self { ids_per_frame: 1 }
	}
}

/// Padded terrain ids replacing raw Durham presentation roots this frame.
#[derive(Resource, Default)]
pub struct UrbanizationPaddedTerrainState {
	wanted: HashSet<Id>,
}

/// Keep / queue / bullseye resources the stream system drives.
#[derive(SystemParam)]
pub struct UrbanizationStreamLod<'w> {
	index: ResMut<'w, UrbanizationIndex>,
	generate: ResMut<'w, UrbanizationGenerateBullseye>,
	present: ResMut<'w, UrbanizationPresentBullseye>,
	generate_queue: ResMut<'w, LodGenerateQueue<SelectedUrbanization>>,
	presenter: ResMut<'w, UrbanizationPresenterState>,
	generate_regions: MessageWriter<'w, LodGenerateRegion<UrbanizationLodChan>>,
	present_regions: MessageWriter<'w, LodPresentRegion<UrbanizationLodChan>>,
	generate_keep: ResMut<'w, LodGenerateKeepRegion<UrbanizationLodChan>>,
	keep: ResMut<'w, LodPresentKeepRegion<UrbanizationLodChan>>,
}

impl UrbanizationStreamLod<'_> {
	/// Enable or tear down the urbanization stream from an optional spec and camera.
	pub fn apply_spec(
		&mut self,
		commands: &mut Commands,
		spec: Option<&UrbanizationStreamSpec>,
		camera: Option<Vec3>,
		last_key: &mut Option<String>,
	) {
		let Some(spec) = spec else {
			self.generate.enabled = false;
			self.present.enabled = false;
			self.generate_keep.region = None;
			self.keep.region = None;
			self.index.clear();
			self.generate_queue.clear();
			self.presenter.clear(commands);
			last_key.take();
			return;
		};

		let key = spec.key();
		let key_changed = last_key.as_ref() != Some(&key);
		if key_changed {
			self.index.clear();
			self.generate_queue.clear();
			self.presenter.clear(commands);
			*last_key = Some(key);
		}

		self.index.noise = spec.noise;
		self.index.kind = spec.kind;
		let (present_m, generate_m) = stream_radii_m(spec.stream_radius);
		self.generate.radius_m = generate_m;
		self.generate.enabled = true;
		self.present.radius_m = present_m;
		self.present.enabled = true;

		let Some(cam) = camera else {
			return;
		};
		let generate_aabb = UrbanizationExtent::xz_radius_aabb(cam, generate_m);
		let present_aabb = UrbanizationExtent::xz_radius_aabb(cam, present_m);
		self.generate_keep.region = Some(generate_aabb);
		self.keep.region = Some(present_aabb);
		if key_changed {
			self.generate_regions.write(LodGenerateRegion::new(generate_aabb));
			self.present_regions.write(LodPresentRegion::new(present_aabb));
		}
	}
}

/// Drive urbanization bullseyes from [`crate::PlaygroundConfig::urbanization`].
pub fn stream_urbanization(
	mut commands: Commands,
	config: Res<crate::PlaygroundConfig>,
	camera: Query<&Transform, With<Camera3d>>,
	mut lod: UrbanizationStreamLod,
	mut last_key: Local<Option<String>>,
) {
	let cam = camera.single().ok().map(|t| t.translation);
	lod.apply_spec(&mut commands, config.urbanization.as_ref(), cam, &mut last_key);
}

/// Bake a bounded number of padded Durham cells after hosts have written pads.
///
/// Skip ids whose Durham cell is not in the terrain store yet. Do not
/// `get_or_generate_region` the whole present keep.
pub fn generate_urbanization_padded_terrain(
	config: Res<crate::PlaygroundConfig>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	budget: Res<UrbanizationPaddedTerrainBudget>,
	mut development: DevelopmentIndex,
) {
	if config.urbanization.is_none() {
		return;
	}
	let Some(region) = keep.region else {
		return;
	};
	development.store.invalidate_dirty_padded();
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	let mut ids: Vec<Id> = TerrainWithPads::original_ids_for(&mut development, region)
		.into_iter()
		.map(|OriginalId(id)| id)
		.collect();
	ids.sort();
	let mut remaining = budget.ids_per_frame;
	for id in ids {
		if remaining == 0 {
			break;
		}
		if SpatialIndex::<TerrainWithPads>::get(&development, id).is_some() {
			continue;
		}
		if development.terrain_store().terrain(id).is_none() {
			continue;
		}
		let _ = GeneratingSpatialIndex::<TerrainWithPads>::get_or_generate(
			&mut development,
			id,
			&lod_ref,
		);
		remaining -= 1;
	}
}

/// Present padded replacements for the urbanization keep and cull stale cells.
pub fn present_urbanization_padded_terrain(
	config: Res<crate::PlaygroundConfig>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	store: Res<DevelopmentEntryStore>,
	mut presenter: PaddedTerrainPresenter,
	mut state: ResMut<UrbanizationPaddedTerrainState>,
	viewers: Query<&Transform, With<LodViewer>>,
) {
	state.wanted.clear();
	let Some(region) = keep.region.filter(|_| config.urbanization.is_some()) else {
		presenter.remove_stale(&state.wanted);
		return;
	};
	let viewer = viewers.single().copied().unwrap_or(Transform::IDENTITY);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &viewer,
		current_transform: &viewer,
		bounds: &region,
	};
	let view = PaddedStoreView::new(&store);
	RegionPresenter::<TerrainWithPads, _>::present(&mut presenter, &view, region, &lod_ref);
	state.wanted = SpatialIndex::<TerrainWithPads>::tracked_ids_for(&view, region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	presenter.remove_stale(&state.wanted);
}

/// Hide raw Durham visuals while their padded replacements are active.
///
/// Raw collision stays live: stable terrain colliders are independent of visual
/// LOD and must never be disabled during a raw/padded presentation handoff.
pub fn sync_raw_terrain_replacements(
	mut commands: Commands,
	state: Res<UrbanizationPaddedTerrainState>,
	raw_roots: Query<(Entity, &PresentedTerrainScene)>,
	padded_roots: Query<(Entity, &PresentedPaddedTerrainScene)>,
	children: Query<&Children>,
	meshes: Query<(), With<Mesh3d>>,
	raw_terrain_meshes: Query<(), With<MeshMaterial3d<DurhamTerrainShader>>>,
) {
	let ready: HashSet<Id> = padded_roots
		.iter()
		.filter(|(root, _)| children.iter_descendants(*root).any(|child| meshes.contains(child)))
		.map(|(_, presented)| presented.0)
		.collect();
	for (root, presented) in &raw_roots {
		// Keep raw collision live before, during, and after the visual handoff.
		let replaced = state.wanted.contains(&presented.0) && ready.contains(&presented.0);
		commands.entity(root).insert(Visibility::Inherited);
		for child in children.iter_descendants(root) {
			if raw_terrain_meshes.contains(child) {
				commands.entity(child).insert(if replaced {
					Visibility::Hidden
				} else {
					Visibility::Inherited
				});
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use procedural_common::noise_params_from_scalar_str;

	#[test]
	fn default_urbanization_noise_parses() -> Result<()> {
		let noise = noise_params_from_scalar_str(DEFAULT_URBANIZATION_NOISE)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(noise.seed, 1337);
		assert!((noise.frequency - 0.0005).abs() < 1e-8);
		Ok(())
	}

	#[test]
	fn parse_urbanization_kind_accepts_kebab() -> Result<()> {
		assert_eq!(
			parse_urbanization_kind("frontier").map_err(|e| anyhow::anyhow!("{e}"))?,
			UrbanizationKind::Frontier
		);
		assert!(parse_urbanization_kind("not-a-city").is_err());
		Ok(())
	}

	#[test]
	fn default_stream_radii_are_one_and_three_kilometres() -> Result<()> {
		let (present, generate) = stream_radii_m(DEFAULT_URBANIZATION_STREAM_RADIUS);
		assert!((present - DEVELOPMENT_PRESENT_RADIUS_M).abs() < 1e-3);
		assert!((generate - DEVELOPMENT_GENERATE_RADIUS_M).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn default_spec_matches_noise_string() -> Result<()> {
		let parsed = noise_params_from_scalar_str(DEFAULT_URBANIZATION_NOISE)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let spec = UrbanizationStreamSpec::default();
		assert_eq!(spec.noise.seed, parsed.seed);
		assert!((spec.noise.frequency - parsed.frequency).abs() < 1e-8);
		assert_eq!(spec.stream_radius, DEFAULT_URBANIZATION_STREAM_RADIUS);
		Ok(())
	}

	#[test]
	fn host_and_pad_budgets_default_to_one_id_per_frame() {
		assert_eq!(UrbanizationHostBudget::default().cells_per_frame, 1);
		assert_eq!(UrbanizationPaddedTerrainBudget::default().ids_per_frame, 1);
	}
}
