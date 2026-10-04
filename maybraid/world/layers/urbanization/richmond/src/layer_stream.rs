//! Richmond hopscotch stream and development generate.

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use layer_stack::{GenerationMode, GenerationModeSystems};
use lod::gen::{
	GeneratingSpatialIndex, Id, LodGenerateBudget, LodGenerateKeepRegion, LodGenerateQueue,
	LodGenerateRegion, MaterializeStatus, SpatialIndex, StorageStatus,
};
use lod::lod_ref::LodRef;
use lod::presentation::{LodPresentKeepRegion, LodPresentRegion};
use lod::{
	LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems, LodJobCounter,
	LodPresentRegionPlugin, LodPresentSystems, LodViewer,
};
use terrain_layer_model::{terrain_streaming, TerrainExtent, TerrainLayerSystems};
use urbanization_cells::{
	SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent, UrbanizationGenerateBullseye,
	UrbanizationIndex, UrbanizationKind, UrbanizationLodChan, UrbanizationPresentBullseye,
	DEFAULT_URBANIZATION_EXTENT_XZ, DEVELOPMENT_GENERATE_RADIUS_M, DEVELOPMENT_PRESENT_RADIUS_M,
};
use layer_stack::{LayerModeConfig, LayerSystems};
use urbanization_layer_model::{
	urbanization_visual_region, Urbanization, UrbanizationGenerationSystems, UrbanizationLayerRegion,
};

use crate::config::DevelopmentConfig;
use crate::development::DevelopmentCell;
use crate::ground::RichmondGround;
use crate::index::{DevelopmentEntryStore, DevelopmentIndex};
use crate::layer::Richmond;
use crate::layer_config::{focused_spec, RichmondConfig, UrbanizationStreamSpec};
use crate::padded::TerrainWithPads;
use crate::BuiltDevelopment;

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

/// Present / generate metric radii for a stream-radius multiplier.
pub fn stream_radii_m(stream_radius: u32) -> (f32, f32) {
	if stream_radius == 0 {
		return (DEFAULT_URBANIZATION_EXTENT_XZ, DEFAULT_URBANIZATION_EXTENT_XZ * 2.0);
	}
	let present = DEVELOPMENT_PRESENT_RADIUS_M * stream_radius as f32;
	(present, present + (DEVELOPMENT_GENERATE_RADIUS_M - DEVELOPMENT_PRESENT_RADIUS_M))
}

/// Generate + present-keep plugins for [`SelectedUrbanization`].
pub fn register_urbanization_lod_generate(app: &mut App) {
	app.init_resource::<UrbanizationIndex>()
		.init_resource::<UrbanizationGenerateBullseye>()
		.init_resource::<UrbanizationPresentBullseye>()
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
pub fn install_urbanization_stream<Mode, G>(app: &mut App)
where
	Mode: GenerationMode,
	G: RichmondGround,
{
	register_urbanization_lod_generate(app);
	app.add_systems(
		Update,
		sync_urbanization_pin::<Mode, G>
			.in_set(GenerationModeSystems::<Mode>::default())
			.in_set(UrbanizationGenerationSystems)
			.before(LodGenerateSystems::Produce)
			.before(LayerSystems::<Urbanization<Richmond<G>>>::default())
			.before(LodPresentSystems::Produce)
			.before(TerrainLayerSystems::<G::Base>::QueueColliders),
	);
	app.add_systems(
		Update,
		(
			stream_urbanization::<Mode, G>
				.before(LodGenerateSystems::Produce)
				.before(LayerSystems::<Urbanization<Richmond<G>>>::default()),
			generate_urbanization_developments::<Mode, G>
				.after(LodGenerateSystems::Drain)
				.before(LayerSystems::<Urbanization<Richmond<G>>>::default()),
			write_urbanization_host_region.in_set(LayerSystems::<Urbanization<Richmond<G>>>::default()),
		)
			.in_set(GenerationModeSystems::<Mode>::default())
			.in_set(UrbanizationGenerationSystems)
			.run_if(terrain_streaming::<G>)
			.before(LodPresentSystems::Produce)
			.before(TerrainLayerSystems::<G::Base>::QueueColliders),
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
	jobs: Res<'w, LodJobCounter>,
}

impl UrbanizationStreamLod<'_> {
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
			self.jobs.end_n(self.generate_queue.clear());
			last_key.take();
			return;
		};

		let key = spec.key();
		let key_changed = last_key.as_ref() != Some(&key);
		if key_changed {
			self.index.clear();
			self.jobs.end_n(self.generate_queue.clear());
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

pub fn sync_urbanization_pin<Mode: GenerationMode, G: RichmondGround>(
	config: Res<LayerModeConfig<Mode, Urbanization<Richmond<G>>>>,
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

pub fn stream_urbanization<Mode: GenerationMode, G: RichmondGround>(
	config: Res<LayerModeConfig<Mode, Urbanization<Richmond<G>>>>,
	camera: Query<&Transform, With<Camera3d>>,
	mut lod: UrbanizationStreamLod,
	mut last_key: ResMut<UrbanizationStreamKey>,
) {
	let cam = camera.single().ok().map(|t| t.translation);
	lod.apply_spec(focused_spec(&config.config).as_ref(), cam, &mut last_key.0);
}

pub fn clear_urbanization_stream_world(world: &mut World) {
	if let Some(mut key) = world.get_resource_mut::<UrbanizationStreamKey>() {
		key.0 = None;
	}
	if let Some(mut generate) = world.get_resource_mut::<UrbanizationGenerateBullseye>() {
		generate.enabled = false;
	}
	if let Some(mut present) = world.get_resource_mut::<UrbanizationPresentBullseye>() {
		present.enabled = false;
	}
	if let Some(mut keep) = world.get_resource_mut::<LodGenerateKeepRegion<UrbanizationLodChan>>() {
		keep.region = None;
	}
	if let Some(mut keep) = world.get_resource_mut::<LodPresentKeepRegion<UrbanizationLodChan>>() {
		keep.region = None;
	}
	if let Some(mut index) = world.get_resource_mut::<UrbanizationIndex>() {
		index.clear();
	}
	let cancelled = world
		.get_resource_mut::<LodGenerateQueue<SelectedUrbanization>>()
		.map(|mut queue| queue.clear())
		.unwrap_or(0);
	LodJobCounter::end_cleared(world, cancelled);
}

/// Bounded leaf generate on the 1 km urbanization keep.
#[allow(clippy::collapsible_if)]
pub fn generate_urbanization_developments<Mode: GenerationMode, G: RichmondGround>(
	config: Res<LayerModeConfig<Mode, Urbanization<Richmond<G>>>>,
	keep: Res<LodPresentKeepRegion<UrbanizationLodChan>>,
	mut development: DevelopmentIndex<G>,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PaddedTerrainTickKey {
	region: Aabb3d,
	store_rev: u64,
	terrain_rev: u64,
	viewer: Option<(i32, i32)>,
}

/// Compose pads only for stored ground cells.
pub(crate) fn generate_richmond_padded_terrain<G: RichmondGround>(
	layer: Res<UrbanizationLayerRegion>,
	extent: Res<TerrainExtent<G::Base>>,
	mut development: DevelopmentIndex<G>,
	mut last: Local<Option<PaddedTerrainTickKey>>,
) {
	let Some(region) = urbanization_visual_region(&*extent, layer.region) else {
		*last = None;
		return;
	};
	let removed = development.store.invalidate_dirty_padded();
	let key = PaddedTerrainTickKey {
		region,
		store_rev: development.store.membership_revision(),
		terrain_rev: G::membership_revision(&development.ground),
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
	for id in G::terrain_ids_overlapping(&development.ground, region) {
		let _ = GeneratingSpatialIndex::<TerrainWithPads>::get_or_generate(
			&mut development,
			id,
			&lod_ref,
		);
	}
	*last = Some(PaddedTerrainTickKey {
		region,
		store_rev: development.store.membership_revision(),
		terrain_rev: G::membership_revision(&development.ground),
		viewer: None,
	});
}

impl<G: RichmondGround> urbanization_layer_model::UrbanizationGeneration for Richmond<G> {
	const LABEL: &'static str = "richmond";
	type Config = RichmondConfig;

	fn install_generation(app: &mut App) {
		crate::plugin::register_richmond_plugin(app);
		app.init_resource::<DevelopmentConfig>()
			.init_resource::<LodGenerateBudget<UrbanizationLodChan>>()
			.init_resource::<UrbanizationLayerRegion>()
			.init_resource::<UrbanizationStreamKey>();
		app.add_systems(
			Update,
			generate_richmond_padded_terrain::<G>
				.in_set(UrbanizationGenerationSystems)
				.after(LayerSystems::<Urbanization<Richmond<G>>>::default())
				.run_if(terrain_streaming::<G>)
				.before(LodPresentSystems::Produce)
				.before(TerrainLayerSystems::<G::Base>::QueueColliders),
		);
	}

	fn apply_generation(world: &mut World, config: &RichmondConfig) {
		*world.resource_mut::<DevelopmentConfig>() = config.development_config();
		*world.resource_mut::<LodGenerateBudget<UrbanizationLodChan>>() =
			LodGenerateBudget::new(config.generate_budget);
	}

	fn clear_generation(world: &mut World) {
		if let Some(mut store) = world.get_resource_mut::<DevelopmentEntryStore>() {
			store.clear();
		}
		if let Some(mut layer) = world.get_resource_mut::<UrbanizationLayerRegion>() {
			layer.region = None;
		}
		clear_urbanization_stream_world(world);
	}

	fn install_presentation(app: &mut App) {
		crate::layer_present::install_richmond_presentation::<G>(app);
	}
}
