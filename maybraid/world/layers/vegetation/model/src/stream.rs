//! Forest and canopy bump-out streaming. Selection only: no presenters.

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico::{
	BumpOutGenerateBullseye, BumpOutLodChan, BumpOutPresentBullseye, CanopyBumpOut, ChicoGrove,
	ForestExtent, ForestGenerateBullseye, ForestIndex, ForestLodChan, ForestPresentBullseye,
	MediumBumpOutLodChan, MediumCanopyBumpOut, BUMP_OUT_OUTER_RADIUS_M,
	DEFAULT_FOREST_GROVE_TILE_XZ, GROVE_GENERATE_RADIUS_M, GROVE_PRESENT_RADIUS_M,
	MEDIUM_BUMP_OUT_ANCHOR_STEP_M, MEDIUM_BUMP_OUT_OUTER_RADIUS_M,
};
use durham::terrain_streaming_enabled;
use lod::gen::{LodGenerateBudget, LodGenerateKeepRegion, LodGenerateQueue, LodGenerateRegion};
use lod::presentation::{LodPresentKeepRegion, LodPresentQueue, LodPresentRegion};
use lod::{
	LodGeneratePlugin, LodGenerateRegionPlugin, LodGenerateSystems, LodPresentRegionPlugin,
	LodPresentSystems, LodViewer,
};

use crate::config::ForestStreamSpec;
use crate::generation::{VegetationGenerationSystems, VegetationModeConfig};
use layer_stack::{GenerationMode, GenerationModeSystems};

/// Spec fingerprint. A resource so leaving a mode can clear it.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct VegetationStreamKey(pub Option<String>);

/// Default present ring multiplier (`1` → 1 km grove present / 3 km generate).
pub const DEFAULT_FOREST_STREAM_RADIUS: u32 = 1;

/// Hopscotch default so neighboring 1600 m cells stay related.
pub const DEFAULT_FOREST_NOISE: &str = "1337,0.0005,1,1";

/// Present / generate metric radii for a stream-radius multiplier.
pub fn stream_radii_m(stream_radius: u32) -> (f32, f32) {
	if stream_radius == 0 {
		return (DEFAULT_FOREST_GROVE_TILE_XZ, DEFAULT_FOREST_GROVE_TILE_XZ * 2.0);
	}
	let present = GROVE_PRESENT_RADIUS_M * stream_radius as f32;
	(present, present + (GROVE_GENERATE_RADIUS_M - GROVE_PRESENT_RADIUS_M))
}

/// Generate half of the old `register_forest_lod`. Present plugins stay in
/// vegetation presentation. Each mode writes [`LodGenerateBudget`] on enter.
pub fn register_forest_generate(app: &mut App) {
	app.init_resource::<ForestIndex>()
		.init_resource::<LodGenerateBudget<ForestLodChan>>()
		.init_resource::<LodPresentQueue<ChicoGrove>>()
		.add_plugins(LodGenerateRegionPlugin::<
			ForestGenerateBullseye,
			With<LodViewer>,
			ForestLodChan,
		>::default())
		.add_plugins(LodGeneratePlugin::<
			ChicoGrove,
			ForestIndex,
			ForestLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentRegionPlugin::<
			ForestPresentBullseye,
			With<LodViewer>,
			ForestLodChan,
		>::default())
		.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain));
}

/// Generate half of the old `register_bump_out_lod`.
pub fn register_bump_out_generate(app: &mut App) {
	app.init_resource::<LodGenerateBudget<BumpOutLodChan>>()
		.init_resource::<LodGenerateBudget<MediumBumpOutLodChan>>()
		.init_resource::<LodPresentQueue<CanopyBumpOut>>()
		.init_resource::<LodPresentQueue<MediumCanopyBumpOut>>()
		.init_resource::<LodPresentKeepRegion<MediumBumpOutLodChan>>()
		.add_plugins(LodGenerateRegionPlugin::<
			BumpOutGenerateBullseye,
			With<LodViewer>,
			BumpOutLodChan,
		>::default())
		.add_plugins(LodGeneratePlugin::<
			CanopyBumpOut,
			ForestIndex,
			BumpOutLodChan,
			With<LodViewer>,
		>::default())
		.add_plugins(LodPresentRegionPlugin::<
			BumpOutPresentBullseye,
			With<LodViewer>,
			BumpOutLodChan,
		>::default())
		.add_plugins(LodGeneratePlugin::<
			MediumCanopyBumpOut,
			ForestIndex,
			MediumBumpOutLodChan,
			With<LodViewer>,
		>::default())
		.configure_sets(Update, LodPresentSystems::Produce.after(LodGenerateSystems::Drain));
}

/// Keep / queue / bullseye resources the forest stream drives.
///
/// Presenter teardown lives in vegetation presentation so this crate does not
/// despawn hosts.
#[derive(SystemParam)]
pub struct ForestStreamLod<'w> {
	index: ResMut<'w, ForestIndex>,
	generate: ResMut<'w, ForestGenerateBullseye>,
	present: ResMut<'w, ForestPresentBullseye>,
	generate_queue: ResMut<'w, LodGenerateQueue<ChicoGrove>>,
	present_queue: ResMut<'w, LodPresentQueue<ChicoGrove>>,
	generate_regions: MessageWriter<'w, LodGenerateRegion<ForestLodChan>>,
	present_regions: MessageWriter<'w, LodPresentRegion<ForestLodChan>>,
	generate_keep: ResMut<'w, LodGenerateKeepRegion<ForestLodChan>>,
	keep: ResMut<'w, LodPresentKeepRegion<ForestLodChan>>,
}

impl ForestStreamLod<'_> {
	/// Enable or tear down the forest stream from an optional spec and camera.
	pub fn apply_spec(
		&mut self,
		spec: Option<&ForestStreamSpec>,
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
			self.present_queue.clear();
			last_key.take();
			return;
		};

		let key = spec.key();
		let key_changed = last_key.as_ref() != Some(&key);
		if key_changed {
			self.index.clear();
			self.generate_queue.clear();
			self.present_queue.clear();
			*last_key = Some(key);
		}

		self.index.noise = spec.noise;
		self.index.layering = spec.layering;
		let (present_m, generate_m) = stream_radii_m(spec.stream_radius);
		self.generate.radius_m = generate_m;
		self.generate.enabled = true;
		self.present.radius_m = present_m;
		self.present.enabled = true;

		let Some(cam) = camera else {
			return;
		};
		let generate_aabb = ForestExtent::xz_radius_aabb(cam, generate_m);
		let present_aabb = ForestExtent::xz_radius_aabb(cam, present_m);
		self.generate_keep.region = Some(generate_aabb);
		self.keep.region = Some(present_aabb);
		if key_changed {
			self.generate_regions.write(LodGenerateRegion::new(generate_aabb));
			self.present_regions.write(LodPresentRegion::new(present_aabb));
		}
	}
}

/// Keep / queue / bullseye resources the forest stream drives for bump-outs.
#[derive(SystemParam)]
pub struct BumpOutStreamLod<'w> {
	generate: ResMut<'w, BumpOutGenerateBullseye>,
	present: ResMut<'w, BumpOutPresentBullseye>,
	generate_queue: ResMut<'w, LodGenerateQueue<CanopyBumpOut>>,
	present_queue: ResMut<'w, LodPresentQueue<CanopyBumpOut>>,
	medium_generate_queue: ResMut<'w, LodGenerateQueue<MediumCanopyBumpOut>>,
	medium_present_queue: ResMut<'w, LodPresentQueue<MediumCanopyBumpOut>>,
	generate_regions: MessageWriter<'w, LodGenerateRegion<BumpOutLodChan>>,
	present_regions: MessageWriter<'w, LodPresentRegion<BumpOutLodChan>>,
	medium_generate_regions: MessageWriter<'w, LodGenerateRegion<MediumBumpOutLodChan>>,
	medium_present_regions: MessageWriter<'w, LodPresentRegion<MediumBumpOutLodChan>>,
	generate_keep: ResMut<'w, LodGenerateKeepRegion<BumpOutLodChan>>,
	keep: ResMut<'w, LodPresentKeepRegion<BumpOutLodChan>>,
	medium_generate_keep: ResMut<'w, LodGenerateKeepRegion<MediumBumpOutLodChan>>,
	medium_keep: ResMut<'w, LodPresentKeepRegion<MediumBumpOutLodChan>>,
}

impl BumpOutStreamLod<'_> {
	pub fn apply_spec(
		&mut self,
		spec: Option<&ForestStreamSpec>,
		camera: Option<Vec3>,
		last_key: &mut Option<String>,
		last_medium_region: &mut Option<Aabb3d>,
	) {
		let Some(spec) = spec else {
			self.generate.enabled = false;
			self.present.enabled = false;
			self.generate_keep.region = None;
			self.keep.region = None;
			self.generate_queue.clear();
			self.present_queue.clear();
			self.medium_generate_keep.region = None;
			self.medium_keep.region = None;
			self.medium_generate_queue.clear();
			self.medium_present_queue.clear();
			last_key.take();
			last_medium_region.take();
			return;
		};

		let key = spec.key();
		let key_changed = last_key.as_ref() != Some(&key);
		if key_changed {
			self.generate_queue.clear();
			self.present_queue.clear();
			self.medium_generate_queue.clear();
			self.medium_present_queue.clear();
			*last_key = Some(key);
		}

		self.generate.radius_m = BUMP_OUT_OUTER_RADIUS_M;
		self.generate.enabled = true;
		self.present.radius_m = BUMP_OUT_OUTER_RADIUS_M;
		self.present.enabled = true;

		let Some(cam) = camera else {
			return;
		};
		let aabb = ForestExtent::xz_radius_aabb(cam, BUMP_OUT_OUTER_RADIUS_M);
		self.generate_keep.region = Some(aabb);
		self.keep.region = Some(aabb);
		if key_changed {
			self.generate_regions.write(LodGenerateRegion::new(aabb));
			self.present_regions.write(LodPresentRegion::new(aabb));
		}

		let step = MEDIUM_BUMP_OUT_ANCHOR_STEP_M;
		let anchor = Vec3::new((cam.x / step).round() * step, 0.0, (cam.z / step).round() * step);
		let medium_region = ForestExtent::xz_radius_aabb(anchor, MEDIUM_BUMP_OUT_OUTER_RADIUS_M);
		let medium_region_changed = *last_medium_region != Some(medium_region);
		self.medium_generate_keep.region = Some(medium_region);
		self.medium_keep.region = Some(medium_region);
		if key_changed || medium_region_changed {
			self.medium_generate_regions.write(LodGenerateRegion::new(medium_region));
			self.medium_present_regions.write(LodPresentRegion::new(medium_region));
			*last_medium_region = Some(medium_region);
		}
	}
}

pub fn stream_forest<Mode: GenerationMode>(
	config: Res<VegetationModeConfig<Mode>>,
	camera: Query<&Transform, With<Camera3d>>,
	mut lod: ForestStreamLod,
	mut last_key: ResMut<VegetationStreamKey>,
) {
	let cam = camera.single().ok().map(|t| t.translation);
	lod.apply_spec(config.config.forest.as_ref(), cam, &mut last_key.0);
}

pub fn stream_canopy_bump_outs<Mode: GenerationMode>(
	config: Res<VegetationModeConfig<Mode>>,
	camera: Query<&Transform, With<Camera3d>>,
	mut lod: BumpOutStreamLod,
	mut last_key: ResMut<VegetationStreamKey>,
	mut last_medium_region: Local<Option<Aabb3d>>,
) {
	let cam = camera.single().ok().map(|t| t.translation);
	lod.apply_spec(
		config.config.forest.as_ref(),
		cam,
		&mut last_key.0,
		&mut last_medium_region,
	);
}

/// Forest and bump-out streams share one key. Snapshot it once so the second
/// apply still sees the cleared value after a hop.
pub fn stream_vegetation<Mode: GenerationMode>(
	config: Res<VegetationModeConfig<Mode>>,
	camera: Query<&Transform, With<Camera3d>>,
	mut forest: ForestStreamLod,
	mut bump_outs: BumpOutStreamLod,
	mut last_key: ResMut<VegetationStreamKey>,
	mut last_medium_region: Local<Option<Aabb3d>>,
) {
	let cam = camera.single().ok().map(|t| t.translation);
	let spec = config.config.forest.as_ref();
	let mut forest_key = last_key.0.clone();
	let mut bump_key = last_key.0.clone();
	forest.apply_spec(spec, cam, &mut forest_key);
	bump_outs.apply_spec(spec, cam, &mut bump_key, &mut last_medium_region);
	last_key.0 = forest_key;
}

/// Tear stream LOD and the spec key down so the next mode can refill.
pub fn clear_vegetation_stream(
	index: Option<ResMut<ForestIndex>>,
	key: Option<ResMut<VegetationStreamKey>>,
	forest: Option<ForestStreamLod>,
	bump_outs: Option<BumpOutStreamLod>,
) {
	if let Some(mut key) = key {
		key.0 = None;
	}
	if let Some(mut lod) = forest {
		let mut last = None;
		lod.apply_spec(None, None, &mut last);
	} else if let Some(mut index) = index {
		index.clear();
	}
	if let Some(mut lod) = bump_outs {
		let mut last = None;
		let mut last_medium_region = None;
		lod.apply_spec(None, None, &mut last, &mut last_medium_region);
	}
}

/// Forest and bump-out streams for `Mode`, reading [`VegetationModeConfig`].
pub fn install_vegetation_stream<Mode: GenerationMode>(app: &mut App) {
	app.add_systems(
		Update,
		stream_vegetation::<Mode>
			.in_set(GenerationModeSystems::<Mode>::default())
			.in_set(VegetationGenerationSystems)
			.run_if(terrain_streaming_enabled)
			.before(LodGenerateSystems::Produce)
			.before(LodPresentSystems::Produce),
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use chico::{DEFAULT_FOREST_GROVE_TILE_XZ, GROVE_GENERATE_RADIUS_M, GROVE_PRESENT_RADIUS_M};

	#[test]
	fn default_stream_radii_are_one_and_three_kilometres() -> anyhow::Result<()> {
		let (present, generate) = stream_radii_m(DEFAULT_FOREST_STREAM_RADIUS);
		assert!((present - GROVE_PRESENT_RADIUS_M).abs() < 1e-3);
		assert!((generate - GROVE_GENERATE_RADIUS_M).abs() < 1e-3);
		let (tight_present, tight_generate) = stream_radii_m(0);
		assert!((tight_present - DEFAULT_FOREST_GROVE_TILE_XZ).abs() < 1e-3);
		assert!(tight_generate > tight_present);
		Ok(())
	}
}
