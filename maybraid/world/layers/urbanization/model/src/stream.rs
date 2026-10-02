//! Urbanization generate / stream glue (forest_stream parallel).
//!
//! Registers LOD generate for [`SelectedUrbanization`] and a present-keep
//! bullseye. The present-keep region is a generation input: development
//! build reads it. Presenter state lives in urbanization presentation.

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use durham::TerrainCellLayout;
use lod::gen::{
	GeneratingSpatialIndex, Id, LodGenerateBudget, LodGenerateKeepRegion, LodGenerateQueue,
	LodGenerateRegion, MaterializeStatus, SpatialIndex, StorageStatus,
};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion};
use lod::{LodGeneratePlugin, LodGenerateRegionPlugin, LodPresentRegionPlugin, LodViewer};
use procedural_common::NoiseParams;
use richmond::{
	BuiltDevelopment, DevelopmentCell, DevelopmentConfig, DevelopmentIndex, TerrainWithPads,
};
use urbanization_cells::{
	SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent, UrbanizationGenerateBullseye,
	UrbanizationIndex, UrbanizationKind, UrbanizationLodChan, UrbanizationPresentBullseye,
	DEFAULT_URBANIZATION_EXTENT_XZ, DEVELOPMENT_GENERATE_RADIUS_M, DEVELOPMENT_PRESENT_RADIUS_M,
};

use crate::config::UrbanizationLayerConfig;
use crate::generation::UrbanizationLayerRegion;
use crate::generation::UrbanizationModeConfig;
use layer_stack::GenerationMode;

/// Default present ring multiplier (`1` → 1 km present / 3 km generate).
pub const DEFAULT_URBANIZATION_STREAM_RADIUS: u32 = 1;

/// Hopscotch default so neighboring 1600 m cells stay related.
pub const DEFAULT_URBANIZATION_NOISE: &str = "1337,0.0005,1,1";

/// Stream spec fingerprint. A resource so leaving a mode can clear it.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct UrbanizationStreamKey(pub Option<String>);

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

/// Spec the stream and pin write: `focus_urbanization` fills an open kind.
fn focused_spec(config: &UrbanizationLayerConfig) -> Option<UrbanizationStreamSpec> {
	config.urbanization.map(|mut spec| {
		spec.kind = spec.kind.or(config.focus_urbanization);
		spec
	})
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

/// Hopscotch stream for a mode that owns a spec.
pub fn install_urbanization_stream<Mode: GenerationMode>(
	app: &mut App,
	config: &UrbanizationLayerConfig,
) {
	use crate::generation::{UrbanizationGenerationSystems, UrbanizationStoreSystems};
	use durham::terrain_streaming_enabled;
	use durham::TerrainColliderSystems;
	use lod::LodGenerateSystems;
	use lod::LodPresentSystems;
	use layer_stack::GenerationModeSystems;

	register_urbanization_lod_generate(app, config.generate_budget);
	// Mob readers of UrbanizationIndex select cells before terrain streaming starts.
	app.add_systems(
		Update,
		sync_urbanization_pin::<Mode>
			.in_set(GenerationModeSystems::<Mode>::default())
			.in_set(UrbanizationGenerationSystems)
			.before(LodGenerateSystems::Produce)
			.before(UrbanizationStoreSystems)
			.before(LodPresentSystems::Produce)
			.before(TerrainColliderSystems::QueueMeshes),
	);
	app.add_systems(
		Update,
		(
			stream_urbanization::<Mode>
				.before(LodGenerateSystems::Produce)
				.before(UrbanizationStoreSystems),
			generate_urbanization_developments::<Mode>
				.after(LodGenerateSystems::Drain)
				.before(UrbanizationStoreSystems),
			write_urbanization_host_region
				.in_set(UrbanizationStoreSystems),
		)
			.in_set(GenerationModeSystems::<Mode>::default())
			.in_set(UrbanizationGenerationSystems)
			.run_if(terrain_streaming_enabled)
			.before(LodPresentSystems::Produce)
			.before(TerrainColliderSystems::QueueMeshes),
	);
}

/// Union of selected urbanization cells overlapping the present keep.
pub fn write_urbanization_host_region(
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	index: Res<UrbanizationIndex>,
	mut layer: ResMut<UrbanizationLayerRegion>,
) {
	let Some(keep) = keep.region else {
		layer.region = None;
		return;
	};
	let mut region: Option<Aabb3d> = None;
	for tracked in SpatialIndex::<SelectedUrbanization>::tracked_ids_for(&*index, keep) {
		let Some(selected) = index.get(tracked.0) else {
			continue;
		};
		let cell = selected.extent.aabb();
		region = Some(match region {
			None => cell,
			Some(acc) => Aabb3d::from_min_max(acc.min.min(cell.min), acc.max.max(cell.max)),
		});
	}
	layer.region = region.or(Some(keep));
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

pub fn sync_urbanization_pin<Mode: GenerationMode>(
	config: Res<UrbanizationModeConfig<Mode>>,
	mut urbanization: ResMut<UrbanizationIndex>,
	mut development: ResMut<DevelopmentConfig>,
) {
	if let Some(spec) = focused_spec(&config.config) {
		urbanization.kind = spec.kind;
		urbanization.noise = spec.noise;
		development.use_urbanization = true;
		development.seed = spec.noise.seed.max(0) as u32;
	} else if let Some(kind) = config.config.focus_urbanization {
		urbanization.kind = Some(kind);
		development.use_urbanization = true;
	}
}

/// Drive urbanization bullseyes from the mode's hopscotch spec.
pub fn stream_urbanization<Mode: GenerationMode>(
	config: Res<UrbanizationModeConfig<Mode>>,
	camera: Query<&Transform, With<Camera3d>>,
	mut lod: UrbanizationStreamLod,
	mut last_key: ResMut<UrbanizationStreamKey>,
) {
	let cam = camera.single().ok().map(|t| t.translation);
	lod.apply_spec(focused_spec(&config.config).as_ref(), cam, &mut last_key.0);
}

/// Tear stream LOD and the spec key down so the next mode can refill.
pub fn clear_urbanization_stream(
	index: Option<ResMut<UrbanizationIndex>>,
	key: Option<ResMut<UrbanizationStreamKey>>,
	lod: Option<UrbanizationStreamLod>,
) {
	if let Some(mut key) = key {
		key.0 = None;
	}
	if let Some(mut lod) = lod {
		let mut last = None;
		lod.apply_spec(None, None, &mut last);
	} else if let Some(mut index) = index {
		index.clear();
	}
}

/// Bounded leaf generate on the 1 km urbanization keep. Height GET miss
/// leaves the leaf `NotTracked` so the next frame retries.
#[allow(clippy::collapsible_if)]
pub fn generate_urbanization_developments<Mode: GenerationMode>(
	config: Res<UrbanizationModeConfig<Mode>>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	mut development: DevelopmentIndex,
	budget: Res<LodGenerateBudget<UrbanizationLodChan>>,
) {
	if config.config.urbanization.is_none() {
		return;
	}
	let Some(region) = keep.region else {
		return;
	};

	let noise = development.config().urbanization_noise();
	development.urbanization.noise = noise;
	if let Some(spec) = focused_spec(&config.config) {
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

/// Visual region for padding. Streamed layouts use Durham's presentation
/// ring; a pinned patch uses the scheme region or the layout.
pub fn urbanization_visual_region(
	layout: &TerrainCellLayout,
	layer_region: Option<Aabb3d>,
) -> Option<Aabb3d> {
	if layout.is_streamed() {
		Some(layout.presentation_region())
	} else {
		Some(layer_region.unwrap_or_else(|| layout.presentation_region()))
	}
}

/// Host region: the scheme's write, or the layout on a pinned patch.
pub fn urbanization_host_region(
	layout: &TerrainCellLayout,
	layer_region: Option<Aabb3d>,
) -> Option<Aabb3d> {
	layer_region.or_else(|| {
		if layout.is_streamed() {
			None
		} else {
			Some(layout.presentation_region())
		}
	})
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PaddedTerrainTickKey {
	pub(crate) region: Aabb3d,
	pub(crate) store_rev: u64,
	pub(crate) terrain_rev: u64,
	pub(crate) viewer: Option<(i32, i32)>,
}

/// Compose pads only for Durham cells that are already stored.
///
/// Pads sample the inner terrain store (`M`), never `Urbanization<M>`.
#[allow(private_interfaces)]
pub fn generate_urbanization_padded_terrain(
	layer: Res<UrbanizationLayerRegion>,
	layout: Res<TerrainCellLayout>,
	mut development: DevelopmentIndex,
	mut last: Local<Option<PaddedTerrainTickKey>>,
) {
	let Some(region) = urbanization_visual_region(&layout, layer.region) else {
		*last = None;
		return;
	};
	let removed = development.store.invalidate_dirty_padded();
	let key = PaddedTerrainTickKey {
		region,
		store_rev: development.store.membership_revision(),
		terrain_rev: development.terrain_store().membership_revision(),
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
