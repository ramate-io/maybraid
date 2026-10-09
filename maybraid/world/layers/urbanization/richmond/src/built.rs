//! [`Built`]: the hosts fitted to a filled development.

use std::marker::PhantomData;

use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{bsn, template_value, Name, Transform, Vec3};
use bevy::scene::Scene;
use building_components::scene_children;
use lod::gen::{
	GenerationScheme, Id, LodScene, LodSceneLevel, LodSceneStatus, OriginalId, Version,
};
use lod::hcsg::shared::{self, GenerationContext};
use lod::hcsg::HcsgStorage;
use lod::lod_ref::LodRef;
use urbanization_layer_model::UrbanSetting;

use crate::artifact::BuiltDevelopment;
use crate::developments::RichmondDevelopment;
use crate::ground::RichmondGround;
use crate::host::DevelopmentHosts;
use crate::storage::{column_bounds, overlaps_xz};

/// Buildings fitted to one filled [`RichmondDevelopment`] over ground `G`.
pub struct Built<G> {
	pub development: BuiltDevelopment,
	/// Where people arrive: over the cell center, at the first pad's height.
	pub setting: UrbanSetting,
	pub setting_at: Vec3,
	_ground: PhantomData<fn() -> G>,
}

impl<G> Built<G> {
	pub fn new(development: BuiltDevelopment, setting: UrbanSetting, setting_at: Vec3) -> Self {
		Self { development, setting, setting_at, _ground: PhantomData }
	}

	/// The buildings fitted to development `id`; `None` for an empty one.
	pub fn fit(id: Id, development: &RichmondDevelopment<G>) -> Option<Self> {
		let built = development.built()?;
		let cell = development.cell();
		let center = (Vec3::from(cell.min) + Vec3::from(cell.max)) * 0.5;
		let elevation = development.pads().first().map_or(center.y, |pad| pad.height);
		let arrival_radius =
			((cell.max.x - cell.min.x).min(cell.max.z - cell.min.z) * 0.25).clamp(8.0, 128.0);
		let setting = UrbanSetting { id, arrival_radius };
		Some(Self::new(built, setting, Vec3::new(center.x, elevation, center.z)))
	}
}

impl<G: RichmondGround> Built<G> {
	/// Stored built developments overlapping `region` on XZ, with versions.
	pub fn overlapping(
		storage: &HcsgStorage,
		region: Aabb3d,
	) -> Vec<(Id, Version, &BuiltDevelopment)> {
		storage
			.overlapping::<Self>(column_bounds(region))
			.into_iter()
			.filter_map(|id| {
				let entry = storage.entry::<Self>(id)?;
				overlaps_xz(region, entry.bounds).then_some((
					id,
					entry.version,
					&entry.value.development,
				))
			})
			.collect()
	}
}

impl<G: RichmondGround> GenerationScheme<HcsgStorage> for Built<G> {
	fn original_ids_for(storage: &mut HcsgStorage, region: Aabb3d) -> Vec<OriginalId> {
		storage
			.original_ids_for::<RichmondDevelopment<G>>(region)
			.into_iter()
			.filter(|OriginalId(id)| {
				storage
					.get_one_or_generate::<RichmondDevelopment<G>>(*id)
					.is_some_and(RichmondDevelopment::is_filled)
			})
			.collect()
	}

	fn build_with_id(storage: &mut HcsgStorage, id: Id) -> Option<(Self, Aabb3d)> {
		let development = storage.get_one_or_generate::<RichmondDevelopment<G>>(id)?;
		Some((Self::fit(id, development)?, column_bounds(development.cell())))
	}
}

impl<G: RichmondGround> shared::GenerationScheme for Built<G> {
	/// The filled developments originating in `region`.
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<RichmondDevelopment<G>>(region)
			.into_iter()
			.filter(|OriginalId(id)| {
				cx.get_or_generate::<RichmondDevelopment<G>>(*id)
					.is_some_and(|development| development.is_filled())
			})
			.collect()
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let development = cx.get_or_generate::<RichmondDevelopment<G>>(id)?;
		Some((Self::fit(id, &development)?, column_bounds(development.cell())))
	}
}

/// One level: the [`UrbanSetting`] and a nested host per building, each
/// streaming its own levels.
impl<G: RichmondGround> LodScene for Built<G> {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		let setting = self.setting;
		let at = Transform::from_translation(self.setting_at);
		let mut children: Vec<Box<dyn Scene>> = vec![Box::new(bsn! {
			template_value(Name::new("urban-setting"))
			template(move |_ctx| Ok(setting))
			template_value(at)
		})];
		children.extend(self.development.hosts().iter().map(|host| host.scene(Some(setting.id))));
		scene_children(children)
	}
}
