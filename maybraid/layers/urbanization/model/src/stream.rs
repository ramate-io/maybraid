//! Urbanization generate / stream glue (forest_stream parallel).
//!
//! Registers LOD generate for [`SelectedUrbanization`] and a present-keep
//! bullseye. The present-keep region is a generation input: development
//! build reads it. Presenter state lives in urbanization presentation.

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use durham_terrain_models::TerrainCellLayout;
use lod::gen::{
	GeneratingSpatialIndex, Id, LodGenerateBudget, LodGenerateKeepRegion, LodGenerateQueue,
	LodGenerateRegion, MaterializeStatus, SpatialIndex, StorageStatus,
};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion};
use lod::{LodGeneratePlugin, LodGenerateRegionPlugin, LodPresentRegionPlugin, LodViewer};
use procedural_common::NoiseParams;
use richmond_development_models::{
	BuiltDevelopment, DevelopmentCell, DevelopmentConfig, DevelopmentIndex, TerrainWithPads,
};
use richmond_urbanization::{
	SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent, UrbanizationGenerateBullseye,
	UrbanizationIndex, UrbanizationKind, UrbanizationLodChan, UrbanizationPresentBullseye,
	DEFAULT_URBANIZATION_EXTENT_XZ, DEVELOPMENT_GENERATE_RADIUS_M, DEVELOPMENT_PRESENT_RADIUS_M,
};

use crate::config::UrbanizationLayerConfig;

/// Default present ring multiplier (`1` → 1 km present / 3 km generate).
pub const DEFAULT_URBANIZATION_STREAM_RADIUS: u32 = 1;

/// Hopscotch default so neighboring 1600 m cells stay related.
pub const DEFAULT_URBANIZATION_NOISE: &str = "1337,0.0005,1,1";

/// When false, hopscotch stays off even if Durham streaming is on.
/// Training uses this so the grounds stay a grove instead of a city.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct UrbanizationStreamingEnabled(pub bool);

impl Default for UrbanizationStreamingEnabled {
	fn default() -> Self {
		Self(true)
	}
}

pub fn urbanization_streaming_enabled(enabled: Res<UrbanizationStreamingEnabled>) -> bool {
	enabled.0
}

/// Clap parser for a well-known urbanization kebab name.
pub fn parse_urbanization_kind(name: &str) -> Result<UrbanizationKind, String> {
	UrbanizationKind::from_kebab(name).ok_or_else(|| {
		let names: Vec<_> = UrbanizationKind::ALL.iter().map(|kind| kind.as_kebab()).collect();
		format!("unknown urbanization {name:?}; expected one of: {}", names.join(", "))
	})
}

/// Live urbanization-stream knobs (noise / ring / pinned kind).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UrbanizationStreamSpec {
	pub noise: NoiseParams,
	pub stream_radius: u32,
	pub kind: Option<UrbanizationKind>,
}

impl Default for UrbanizationStreamSpec {
	fn default() -> Self {
		Self {
			noise: NoiseParams {
				seed: 1337,
				frequency: 0.0005,
				amplitude: 1.0,
				octaves: 1,
				..default()
			},
			stream_radius: DEFAULT_URBANIZATION_STREAM_RADIUS,
			kind: None,
		}
	}
}

impl UrbanizationStreamSpec {
	pub fn key(self) -> String {
		let kind_key = self.kind.map(UrbanizationKind::as_kebab).unwrap_or("hopscotch");
		format!("urbanization:{kind_key}|{:?}|r={}", self.noise, self.stream_radius)
	}
}

/// Present / generate metric radii for a stream-radius multiplier.
pub fn stream_radii_m(stream_radius: u32) -> (f32, f32) {
	if stream_radius == 0 {
		return (DEFAULT_URBANIZATION_EXTENT_XZ, DEFAULT_URBANIZATION_EXTENT_XZ * 2.0);
	}
	let present = DEVELOPMENT_PRESENT_RADIUS_M * stream_radius as f32;
	(present, present + (DEVELOPMENT_GENERATE_RADIUS_M - DEVELOPMENT_PRESENT_RADIUS_M))
}

/// Generate + present-keep plugins for [`SelectedUrbanization`].
///
/// Presenter state is installed by urbanization presentation.
pub fn register_urbanization_lod_generate(app: &mut App, generate_budget: u32) {
	app.init_resource::<UrbanizationIndex>()
		.init_resource::<UrbanizationGenerateBullseye>()
		.init_resource::<UrbanizationPresentBullseye>()
		.insert_resource(LodGenerateBudget::<UrbanizationLodChan>::new(generate_budget))
		.add_plugins(LodGenerateRegionPlugin::<
			UrbanizationGenerateBullseye,
			With<LodViewer>,
			UrbanizationLodChan,
		>::default())
		.add_plugins(LodGeneratePlugin::<
			SelectedUrbanization,
			UrbanizationIndex,
			UrbanizationLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentRegionPlugin::<
			UrbanizationPresentBullseye,
			With<LodViewer>,
			UrbanizationLodChan,
		>::default());
}

/// Keep / queue / bullseye resources the stream system drives.
///
/// Presenter teardown lives in urbanization presentation so this crate does
/// not despawn hosts.
#[derive(SystemParam)]
pub struct UrbanizationStreamLod<'w> {
	index: ResMut<'w, UrbanizationIndex>,
	generate: ResMut<'w, UrbanizationGenerateBullseye>,
	present: ResMut<'w, UrbanizationPresentBullseye>,
	generate_queue: ResMut<'w, LodGenerateQueue<SelectedUrbanization>>,
	generate_regions: MessageWriter<'w, LodGenerateRegion<UrbanizationLodChan>>,
	present_regions: MessageWriter<'w, LodPresentRegion<UrbanizationLodChan>>,
	generate_keep: ResMut<'w, LodGenerateKeepRegion<UrbanizationLodChan>>,
	keep: ResMut<'w, LodPresentKeepRegion<UrbanizationLodChan>>,
}

impl UrbanizationStreamLod<'_> {
	/// Enable or tear down the urbanization stream from an optional spec and camera.
	pub fn apply_spec(
		&mut self,
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
			last_key.take();
			return;
		};

		let key = spec.key();
		let key_changed = last_key.as_ref() != Some(&key);
		if key_changed {
			self.index.clear();
			self.generate_queue.clear();
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

pub fn sync_urbanization_pin(
	config: Res<UrbanizationLayerConfig>,
	mut urbanization: ResMut<UrbanizationIndex>,
	mut development: ResMut<DevelopmentConfig>,
) {
	if let Some(spec) = config.urbanization.as_ref() {
		urbanization.kind = spec.kind.or(config.focus_urbanization);
		urbanization.noise = spec.noise;
		development.use_urbanization = true;
		development.seed = spec.noise.seed.max(0) as u32;
	} else if let Some(kind) = config.focus_urbanization {
		urbanization.kind = Some(kind);
		development.use_urbanization = true;
	}
}

/// Drive urbanization bullseyes from [`UrbanizationLayerConfig::urbanization`].
/// Disabling [`UrbanizationStreamingEnabled`] tears the stream down like an
/// absent spec.
pub fn stream_urbanization(
	config: Res<UrbanizationLayerConfig>,
	enabled: Res<UrbanizationStreamingEnabled>,
	camera: Query<&Transform, With<Camera3d>>,
	mut lod: UrbanizationStreamLod,
	mut last_key: Local<Option<String>>,
) {
	let cam = camera.single().ok().map(|t| t.translation);
	let spec = config.urbanization.as_ref().filter(|_| enabled.0);
	lod.apply_spec(spec, cam, &mut last_key);
}

/// Bounded leaf generate on the 1 km urbanization keep. Height GET miss
/// leaves the leaf `NotTracked` so the next frame retries.
#[allow(clippy::collapsible_if)]
pub fn generate_urbanization_developments(
	config: Res<UrbanizationLayerConfig>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	mut development: DevelopmentIndex,
	budget: Res<LodGenerateBudget<UrbanizationLodChan>>,
) {
	if config.urbanization.is_none() {
		return;
	}
	let Some(region) = keep.region else {
		return;
	};

	let noise = development.config().urbanization_noise();
	development.urbanization.noise = noise;
	if let Some(spec) = config.urbanization.as_ref() {
		development.urbanization.kind = spec.kind;
	}

	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};

	let mut created = 0usize;
	let cap = budget.ids_per_frame.max(1) as usize;
	let urbanization_ids: Vec<Id> =
		SpatialIndex::<SelectedUrbanization>::tracked_ids_for(&*development.urbanization, region)
			.into_iter()
			.map(|tracked| tracked.0)
			.collect();
	for id in urbanization_ids {
		if created >= cap {
			break;
		}
		let Some(selected) = development.urbanization.get(id).cloned() else {
			continue;
		};
		for leaf in &selected.leaves {
			if created >= cap {
				break;
			}
			if leaf.kind == UrbanDevelopmentKind::Empty {
				continue;
			}
			let leaf_id = leaf.id();
			if SpatialIndex::<DevelopmentCell>::storage_status(&development, leaf_id)
				== StorageStatus::NotTracked
			{
				if GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate(
					&mut development,
					leaf_id,
					&lod_ref,
				)
				.is_none()
				{
					continue;
				}
			}
			let Some(cell) = SpatialIndex::<DevelopmentCell>::get(&development, leaf_id) else {
				continue;
			};
			if !cell.is_filled() {
				continue;
			}
			if SpatialIndex::<BuiltDevelopment>::storage_status(&development, leaf_id)
				!= StorageStatus::NotTracked
			{
				continue;
			}
			if GeneratingSpatialIndex::<BuiltDevelopment>::get_or_generate(
				&mut development,
				leaf_id,
				&lod_ref,
			) == Some(MaterializeStatus::Created)
			{
				created += 1;
			}
		}
	}
}

pub(crate) fn pad_visual_region(
	layout: &TerrainCellLayout,
	urban_keep: Option<Aabb3d>,
) -> Option<Aabb3d> {
	if layout.is_streamed() {
		Some(layout.presentation_region())
	} else {
		urban_keep
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PaddedTerrainTickKey {
	pub(crate) region: Aabb3d,
	pub(crate) store_rev: u64,
	pub(crate) terrain_rev: u64,
	pub(crate) urban: bool,
	pub(crate) viewer: Option<(i32, i32)>,
}

/// Compose pads only for Durham cells that are already stored.
///
/// Pads sample the inner terrain store (`M`), never `Urbanization<M>`.
#[allow(private_interfaces)]
pub fn generate_urbanization_padded_terrain(
	config: Res<UrbanizationLayerConfig>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	layout: Res<TerrainCellLayout>,
	mut development: DevelopmentIndex,
	mut last: Local<Option<PaddedTerrainTickKey>>,
) {
	if config.urbanization.is_none() {
		*last = None;
		return;
	}
	let Some(region) = pad_visual_region(&layout, keep.region) else {
		*last = None;
		return;
	};
	let removed = development.store.invalidate_dirty_padded();
	let key = PaddedTerrainTickKey {
		region,
		store_rev: development.store.membership_revision(),
		terrain_rev: development.terrain_store().membership_revision(),
		urban: true,
		viewer: None,
	};
	if removed == 0 && last.as_ref() == Some(&key) {
		return;
	}
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &region,
	};
	for id in development.terrain_store().terrain_ids_overlapping(region) {
		let _ = GeneratingSpatialIndex::<TerrainWithPads>::get_or_generate(
			&mut development,
			id,
			&lod_ref,
		);
	}
	*last = Some(PaddedTerrainTickKey {
		region,
		store_rev: development.store.membership_revision(),
		terrain_rev: development.terrain_store().membership_revision(),
		urban: true,
		viewer: None,
	});
}

/// #720 wart: grove present generates [`DevelopmentCell`]s before sampling.
/// Call this named urbanization-generation entry point instead of reaching
/// into Richmond. Removing the present-time generate belongs to
/// <https://github.com/ramate-io/maybraid/issues/720>.
pub fn prepare_development_cells(
	development: &mut DevelopmentIndex,
	bounds: Aabb3d,
	lod_ref: &LodRef,
) {
	use lod::gen::{GenerationScheme, OriginalId};
	let ids = <DevelopmentCell as GenerationScheme<DevelopmentIndex<'_>>>::original_ids_for(
		development,
		bounds,
	);
	for OriginalId(development_id) in ids {
		let _ = GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate(
			development,
			development_id,
			lod_ref,
		);
	}
}
