//! Richmond's generation windows and the hopscotch stream.
//!
//! Every Richmond node is a [`lod::gen::GenerationScheme`] over
//! [`HcsgStorage`]. Two windows subscribe the leaves people see:
//! [`DevelopmentWindow`] composes [`PaddedTerrain`] over the visual region and
//! [`HostWindow`] fits [`Built`] hosts over the host region. Everything
//! between them (sites, developments, urbanization, ground) is generated as
//! their dependencies.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use layer_stack::{GenerationMode, GenerationModeSystems, LayerModeConfig, LayerSystems};
use lod::gen::{LodGenerateBudget, LodGenerated};
use lod::hcsg::{
	CurrentBounds, GenerateOn, GenerateQueue, GenerationProducer, HcsgStorage, LodGenerateSystems,
	Seed,
};
use lod::LodJobCounter;
use terrain_layer_model::{terrain_streaming, TerrainExtent, TerrainLayerSystems};
use urbanization_cells::{
	register_urbanization_nodes, SelectedUrbanization, UrbanizationExtent, UrbanizationKind,
	UrbanizationNodes, UrbanizationSelection, UrbanizationWindow, DEFAULT_URBANIZATION_EXTENT_XZ,
	DEVELOPMENT_GENERATE_RADIUS_M, DEVELOPMENT_PRESENT_RADIUS_M,
};
use urbanization_layer_model::{
	urbanization_host_region, urbanization_visual_region, Urbanization,
	UrbanizationGenerationSystems, UrbanizationLayerRegion,
};

use crate::built::Built;
use crate::config::DevelopmentConfig;
use crate::developments::site::AuthoredDevelopments;
use crate::ground::RichmondGround;
use crate::layer::Richmond;
use crate::layer_config::{focused_spec, RichmondConfig};
use crate::padded::PaddedTerrain;
use crate::storage::{register_richmond_nodes, RichmondNodes};

/// Producer for [`PaddedTerrain`]: the urbanization layer's visual region.
pub struct DevelopmentWindow;

/// Producer for [`Built`] hosts: the urbanization layer's host region.
pub struct HostWindow;

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

/// Hopscotch stream for a mode that owns a spec.
pub fn install_urbanization_stream<Mode, G>(app: &mut App)
where
	Mode: GenerationMode,
	G: RichmondGround,
{
	app.add_systems(
		Update,
		stream_urbanization::<Mode, G>
			.in_set(GenerationModeSystems::<Mode>::default())
			.in_set(UrbanizationGenerationSystems)
			.run_if(terrain_streaming::<G>)
			.before(LayerSystems::<Urbanization<Richmond<G>>>::default())
			.before(LodGenerateSystems::Produce)
			.before(TerrainLayerSystems::<G::Base>::QueueColliders),
	);
}

/// Spec mode: the layer region is every urbanization cell the present
/// ring touches, and [`UrbanizationWindow`] keeps selections in the
/// generate ring. Without a spec the mode presents no urbanization.
pub fn stream_urbanization<Mode: GenerationMode, G: RichmondGround>(
	config: Res<LayerModeConfig<Mode, Urbanization<Richmond<G>>>>,
	camera: Query<&Transform, With<Camera3d>>,
	mut layer: ResMut<UrbanizationLayerRegion>,
	mut urbanization: GenerationProducer<UrbanizationWindow>,
) {
	let Some(spec) = focused_spec(&config.config) else {
		layer.set_if_neq(UrbanizationLayerRegion { region: None });
		return;
	};
	let Ok(camera) = camera.single() else {
		return;
	};
	let cam = camera.translation;
	let (present_m, generate_m) = stream_radii_m(spec.stream_radius);
	urbanization.publish(UrbanizationExtent::xz_radius_aabb(cam, generate_m), Some(cam.xz()));
	let present = UrbanizationExtent::xz_radius_aabb(cam, present_m);
	let region = UrbanizationExtent::cells_overlapping(present)
		.into_iter()
		.map(|extent| extent.aabb())
		.reduce(union);
	layer.set_if_neq(UrbanizationLayerRegion { region: region.or(Some(present)) });
}

/// Publishes both Richmond windows from the layer region. Ground cells
/// generated inside a window are rescanned, so pads and hosts follow the
/// ground they sit on, including after a ground rebuild.
pub fn produce_richmond_windows<G: RichmondGround>(
	layer: Res<UrbanizationLayerRegion>,
	extent: Res<TerrainExtent<G::Base>>,
	storage: Res<HcsgStorage>,
	mut ground: MessageReader<LodGenerated<G::Cell>>,
	mut developments: GenerationProducer<DevelopmentWindow>,
	mut hosts: GenerationProducer<HostWindow>,
) {
	if let Some(keep) = urbanization_visual_region(&*extent, layer.region) {
		developments.publish(keep, None);
	}
	if let Some(keep) = urbanization_host_region(&*extent, layer.region) {
		hosts.publish(keep, None);
	}
	let grown: Vec<Aabb3d> = ground
		.read()
		.filter_map(|generated| storage.entry::<G::Cell>(generated.id).map(|entry| entry.bounds))
		.collect();
	if grown.is_empty() {
		return;
	}
	rescan_within(&mut developments, &grown);
	rescan_within(&mut hosts, &grown);
}

fn rescan_within<P: Send + Sync + 'static>(
	producer: &mut GenerationProducer<P>,
	regions: &[Aabb3d],
) {
	let Some(keep) = producer.current().map(|bounds| bounds.keep) else {
		return;
	};
	producer.rescan(regions.iter().filter_map(|region| clip_xz(*region, keep)));
}

fn clip_xz(region: Aabb3d, keep: Aabb3d) -> Option<Aabb3d> {
	let min = region.min.max(Vec3A::new(keep.min.x, region.min.y, keep.min.z));
	let max = region.max.min(Vec3A::new(keep.max.x, region.max.y, keep.max.z));
	(min.x < max.x && min.z < max.z).then_some(Aabb3d { min, max })
}

fn union(a: Aabb3d, b: Aabb3d) -> Aabb3d {
	Aabb3d { min: a.min.min(b.min), max: a.max.max(b.max) }
}

impl<G: RichmondGround> urbanization_layer_model::UrbanizationGeneration for Richmond<G> {
	const LABEL: &'static str = "richmond";
	type Config = RichmondConfig;

	fn install_generation(app: &mut App) {
		crate::plugin::register_richmond_plugin(app);
		app.init_resource::<HcsgStorage>()
			.init_resource::<DevelopmentConfig>()
			.init_resource::<AuthoredDevelopments>()
			.init_resource::<UrbanizationSelection>()
			.init_resource::<UrbanizationLayerRegion>()
			.add_message::<LodGenerated<G::Cell>>()
			.add_plugins((
				Seed::<DevelopmentConfig>::default()
					.invalidates::<RichmondNodes>()
					.restarts::<DevelopmentWindow>()
					.restarts::<HostWindow>(),
				Seed::<AuthoredDevelopments>::default()
					.invalidates::<RichmondNodes>()
					.restarts::<DevelopmentWindow>()
					.restarts::<HostWindow>(),
				Seed::<UrbanizationSelection>::default()
					.invalidates::<UrbanizationNodes>()
					.invalidates::<RichmondNodes>()
					.restarts::<UrbanizationWindow>()
					.restarts::<DevelopmentWindow>()
					.restarts::<HostWindow>(),
				GenerateOn::<UrbanizationWindow, SelectedUrbanization>::default(),
				GenerateOn::<DevelopmentWindow, PaddedTerrain<G>>::default(),
				GenerateOn::<HostWindow, Built<G>>::default(),
			))
			.add_systems(
				Update,
				produce_richmond_windows::<G>
					.in_set(LodGenerateSystems::Produce)
					.in_set(UrbanizationGenerationSystems)
					.after(LayerSystems::<Urbanization<Richmond<G>>>::default())
					.run_if(terrain_streaming::<G>),
			);
		let mut storage = app.world_mut().resource_mut::<HcsgStorage>();
		register_urbanization_nodes(&mut storage);
		register_richmond_nodes::<G>(&mut storage);
	}

	fn apply_generation(world: &mut World, config: &RichmondConfig) {
		world.insert_resource(config.development_config());
		world.insert_resource(config.urbanization_selection());
		let budget = config.generate_budget;
		world.insert_resource(LodGenerateBudget::<UrbanizationWindow>::new(budget));
		world.insert_resource(LodGenerateBudget::<DevelopmentWindow>::new(budget));
		world.insert_resource(LodGenerateBudget::<HostWindow>::new(budget));
	}

	/// Drops every derived node and forgets the windows. The roots stay
	/// seeded; the next mode's [`Self::apply_generation`] reseeds them.
	fn clear_generation(world: &mut World) {
		if let Some(mut storage) = world.get_resource_mut::<HcsgStorage>() {
			storage.clear_group::<UrbanizationNodes>();
			storage.clear_group::<RichmondNodes>();
		}
		if let Some(mut layer) = world.get_resource_mut::<UrbanizationLayerRegion>() {
			layer.region = None;
		}
		let cancelled = forget_window::<UrbanizationWindow, SelectedUrbanization>(world)
			+ forget_window::<DevelopmentWindow, PaddedTerrain<G>>(world)
			+ forget_window::<HostWindow, Built<G>>(world);
		LodJobCounter::end_cleared(world, cancelled);
	}

	fn install_presentation(app: &mut App) {
		crate::layer_present::install_richmond_presentation::<G>(app);
	}
}

/// Resets `P`'s bounds and `T`'s queue so the next publish scans from scratch.
fn forget_window<P: Send + Sync + 'static, T: Send + Sync + 'static>(world: &mut World) -> u64 {
	if let Some(mut current) = world.get_resource_mut::<CurrentBounds<P>>() {
		current.bounds = None;
	}
	world
		.get_resource_mut::<GenerateQueue<P, T>>()
		.map_or(0, |mut queue| queue.reset())
}
