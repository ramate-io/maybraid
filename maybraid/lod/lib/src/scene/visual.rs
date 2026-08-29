//! Per-view visual LOD: persistent data + [`VisualLodPolicy`] + [`VisualLodRenderer`].
//!
//! [`SceneChunk`] schedules semantic / world realization. This module describes
//! visual alternatives whose representation is selected per view and submitted
//! by a renderer. Camera motion must not mutate semantic roots or enqueue
//! [`SceneChunk`] work.

use std::marker::PhantomData;
use std::sync::Arc;

use bevy::math::Affine3A;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::sync_world::RenderEntity;
use bevy::render::{Extract, ExtractSchedule, RenderApp};
use material_ref::MaterialRef;
use scene_ref::SceneRef;

use crate::scene::refresh::{LodHostBounds, LodViewer};

/// Persistent visual sibling of a [`crate::LodSceneHost`].
///
/// Lives until host cull. Camera-driven representation changes do not write
/// this entity's main-world components.
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct VisualLodRoot;

/// Tile host whose drawable UltraLow/Low/Medium kits are packed visuals.
///
/// Stamp only when packed representations exist. Semantic refresh must not
/// spawn visual kits on this host. Nested hosts under [`VisualLodRoot`] are
/// frozen against camera-driven level writes.
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct VisualOwnsAppearance;

/// One persistent visual band under [`VisualLodRoot`]. Policy shows one band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component, ExtractComponent)]
pub struct VisualLodBand(pub NamedVisualLevel);

/// Named visual band. Policy selection for projected-bounds hosts.
///
/// Ordered coarse → fine so several views can take [`Ord::max`] until the
/// renderer queues a distinct phase item per view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NamedVisualLevel {
	UltraLow,
	Low,
	Medium,
	High,
}

impl NamedVisualLevel {
	pub const ALL: [Self; 4] = [Self::UltraLow, Self::Low, Self::Medium, Self::High];

	/// Packed forest draws High as Medium (High kits stay on the semantic drain).
	pub fn clamp_to_packed(self) -> Self {
		match self {
			Self::High => Self::Medium,
			other => other,
		}
	}

	pub fn to_scene_level(self) -> crate::LodSceneLevel {
		match self {
			Self::UltraLow => crate::LodSceneLevel::UltraLow,
			Self::Low => crate::LodSceneLevel::Low,
			Self::Medium => crate::LodSceneLevel::Medium,
			Self::High => crate::LodSceneLevel::High,
		}
	}

	pub fn from_scene_level(level: crate::LodSceneLevel) -> Option<Self> {
		match level {
			crate::LodSceneLevel::UltraLow => Some(Self::UltraLow),
			crate::LodSceneLevel::Low => Some(Self::Low),
			crate::LodSceneLevel::Medium => Some(Self::Medium),
			crate::LodSceneLevel::High => Some(Self::High),
			crate::LodSceneLevel::Distance(_) | crate::LodSceneLevel::Resolution(_) => None,
		}
	}
}

/// One value per named visual band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Banded<T> {
	pub ultra_low: T,
	pub low: T,
	pub medium: T,
	pub high: T,
}

impl<T> Banded<T> {
	pub fn for_level(&self, level: NamedVisualLevel) -> &T {
		match level {
			NamedVisualLevel::UltraLow => &self.ultra_low,
			NamedVisualLevel::Low => &self.low,
			NamedVisualLevel::Medium => &self.medium,
			NamedVisualLevel::High => &self.high,
		}
	}

	pub fn for_level_mut(&mut self, level: NamedVisualLevel) -> &mut T {
		match level {
			NamedVisualLevel::UltraLow => &mut self.ultra_low,
			NamedVisualLevel::Low => &mut self.low,
			NamedVisualLevel::Medium => &mut self.medium,
			NamedVisualLevel::High => &mut self.high,
		}
	}
}

/// One drawable: per-band [`SceneRef`]s, one material, one local pose.
///
/// Geometry is cached by [`SceneRef`]; material by [`MaterialRef`]. The renderer
/// combines them only at submit. Empty bands stay `None` — UltraLow bins and
/// Low sites are different instance sets, not always the same kit at four LODs.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualInstance {
	pub scenes: Banded<Option<SceneRef>>,
	pub material: MaterialRef,
	pub transform: Affine3A,
}

impl VisualInstance {
	pub fn new(material: MaterialRef, transform: Affine3A) -> Self {
		Self { scenes: Banded::default(), material, transform }
	}

	pub fn scene_for(&self, level: NamedVisualLevel) -> Option<&SceneRef> {
		self.scenes.for_level(level).as_ref()
	}
}

/// Shared instance list used as a [`VisualLodScene::Representation`].
#[derive(Debug, Clone, Default)]
pub struct VisualInstanceList {
	pub instances: Arc<[VisualInstance]>,
}

impl VisualInstanceList {
	pub fn new(instances: impl Into<Arc<[VisualInstance]>>) -> Self {
		Self { instances: instances.into() }
	}

	pub fn is_empty(&self) -> bool {
		self.instances.is_empty()
	}
}

/// True when `entity` is [`VisualLodRoot`] or a descendant (visual band / kits).
pub fn under_visual_lod_root(
	entity: Entity,
	child_of: &Query<&ChildOf>,
	visual_roots: &Query<(), With<VisualLodRoot>>,
) -> bool {
	let mut current = entity;
	loop {
		if visual_roots.contains(current) {
			return true;
		}
		let Ok(parent) = child_of.get(current) else {
			return false;
		};
		current = parent.parent();
	}
}

/// Screen-space AABB height (pixels) that enters each named band.
///
/// `error < ultra_low` → UltraLow, `< low` → Low, `< medium` → Medium, else High.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedBoundsThresholds {
	pub ultra_low: f32,
	pub low: f32,
	pub medium: f32,
}

impl Default for ProjectedBoundsThresholds {
	fn default() -> Self {
		Self { ultra_low: 24.0, low: 80.0, medium: 220.0 }
	}
}

impl ProjectedBoundsThresholds {
	pub fn select(self, screen_error_px: f32) -> NamedVisualLevel {
		if screen_error_px < self.ultra_low {
			NamedVisualLevel::UltraLow
		} else if screen_error_px < self.low {
			NamedVisualLevel::Low
		} else if screen_error_px < self.medium {
			NamedVisualLevel::Medium
		} else {
			NamedVisualLevel::High
		}
	}
}

/// Supplies thresholds for [`ProjectedBoundsPolicy`].
pub trait HasVisualLodThresholds {
	fn visual_lod_thresholds(&self) -> ProjectedBoundsThresholds;
}

/// View used by [`VisualLodPolicy::select`]. Stateless; rebuilt each extract.
#[derive(Debug, Clone, Copy)]
pub struct VisualLodView {
	pub translation: Vec3,
	pub view_from_world: Affine3A,
	pub clip_from_view: Mat4,
	pub viewport_size: Vec2,
}

impl VisualLodView {
	pub fn from_camera(camera: &Camera, transform: &GlobalTransform) -> Option<Self> {
		let viewport = camera.physical_viewport_size()?;
		Some(Self {
			translation: transform.translation(),
			view_from_world: transform.affine().inverse(),
			clip_from_view: camera.clip_from_view(),
			viewport_size: viewport.as_vec2(),
		})
	}

	/// Headless / unit-test perspective looking at `look_at`.
	pub fn test_perspective(translation: Vec3, look_at: Vec3, viewport: Vec2, fov: f32) -> Self {
		let transform = GlobalTransform::from(
			Transform::from_translation(translation).looking_at(look_at, Vec3::Y),
		);
		let aspect = viewport.x / viewport.y.max(1.0);
		Self {
			translation,
			view_from_world: transform.affine().inverse(),
			clip_from_view: Mat4::perspective_rh(fov, aspect, 0.1, 8_000.0),
			viewport_size: viewport,
		}
	}
}

/// Pixel height of `bounds` (world AABB) in `view`.
pub fn projected_screen_error(view: &VisualLodView, bounds: &LodHostBounds) -> f32 {
	let aabb = bounds.0;
	let corners = [
		Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z),
		Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z),
		Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z),
		Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z),
		Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z),
		Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z),
		Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z),
		Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z),
	];
	let mut min_y = f32::INFINITY;
	let mut max_y = f32::NEG_INFINITY;
	let mut any = false;
	for corner in corners {
		let view_p = view.view_from_world.transform_point3(corner);
		let clip = view.clip_from_view.project_point3(view_p);
		if !clip.is_finite() {
			continue;
		}
		min_y = min_y.min(clip.y);
		max_y = max_y.max(clip.y);
		any = true;
	}
	if !any {
		return 0.0;
	}
	((max_y - min_y) * 0.5 * view.viewport_size.y).abs()
}

/// Persistent visual alternatives for one host type. Not a [`crate::SceneChunk`].
pub trait VisualLodScene: Component + Sized {
	type Representation: Send + Sync + 'static;
	type Policy: VisualLodPolicy<Self>;
	type Renderer: VisualLodRenderer<Self>;

	fn visual_representations(&self) -> Self::Representation;
}

/// Which representation this view wants. Pure; no ref resolution.
pub trait VisualLodPolicy<T: VisualLodScene>: Send + Sync + 'static {
	type Selection: Copy + Send + Sync + 'static;

	fn select(scene: &T, view: &VisualLodView, bounds: &LodHostBounds) -> Self::Selection;
}

/// How the selected representation is submitted.
pub trait VisualLodRenderer<T: VisualLodScene>: Send + Sync + 'static {
	fn queue(
		scene: &T,
		selection: <T::Policy as VisualLodPolicy<T>>::Selection,
		ctx: &mut VisualLodRenderContext,
	);
}

/// Render-world insert target for [`VisualLodRenderer::queue`].
pub struct VisualLodRenderContext<'w, 's, 'a> {
	commands: &'a mut Commands<'w, 's>,
	render_entity: Entity,
}

impl<'w, 's, 'a> VisualLodRenderContext<'w, 's, 'a> {
	pub fn render_entity(&self) -> Entity {
		self.render_entity
	}

	pub fn insert(&mut self, bundle: impl Bundle) {
		self.commands.entity(self.render_entity).insert(bundle);
	}
}

/// Stateless projected-AABB policy. First concrete [`VisualLodPolicy`]; 668 reuses it.
pub struct ProjectedBoundsPolicy;

impl<T> VisualLodPolicy<T> for ProjectedBoundsPolicy
where
	T: VisualLodScene<Policy = ProjectedBoundsPolicy> + HasVisualLodThresholds,
{
	type Selection = NamedVisualLevel;

	fn select(scene: &T, view: &VisualLodView, bounds: &LodHostBounds) -> Self::Selection {
		scene.visual_lod_thresholds().select(projected_screen_error(view, bounds))
	}
}

/// Extract + per-view `Policy::select` + `Renderer::queue`. Knows no mesh/material types.
pub struct VisualSceneLodPlugin<T>(PhantomData<fn() -> T>);

impl<T> Default for VisualSceneLodPlugin<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T> Plugin for VisualSceneLodPlugin<T>
where
	T: VisualLodScene,
{
	fn build(&self, app: &mut App) {
		let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
			return;
		};
		render_app.add_systems(ExtractSchedule, extract_visual_lod::<T>);
	}
}

fn extract_visual_lod<T: VisualLodScene>(
	mut commands: Commands,
	views: Extract<Query<(&'static Camera, &'static GlobalTransform), With<LodViewer>>>,
	instances: Extract<
		Query<(RenderEntity, &'static T, &'static LodHostBounds), With<VisualLodRoot>>,
	>,
) {
	let views: Vec<VisualLodView> = views
		.iter()
		.filter_map(|(camera, xf)| VisualLodView::from_camera(camera, xf))
		.collect();
	if views.is_empty() {
		return;
	}
	for (render_entity, scene, bounds) in &instances {
		for view in &views {
			let selection = T::Policy::select(scene, view, bounds);
			let mut ctx = VisualLodRenderContext { commands: &mut commands, render_entity };
			T::Renderer::queue(scene, selection, &mut ctx);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::bounding::Aabb3d;

	#[derive(Component)]
	struct Probe {
		thresholds: ProjectedBoundsThresholds,
	}

	impl HasVisualLodThresholds for Probe {
		fn visual_lod_thresholds(&self) -> ProjectedBoundsThresholds {
			self.thresholds
		}
	}

	impl VisualLodScene for Probe {
		type Representation = ();
		type Policy = ProjectedBoundsPolicy;
		type Renderer = NoopRenderer;

		fn visual_representations(&self) -> Self::Representation {}
	}

	struct NoopRenderer;

	impl VisualLodRenderer<Probe> for NoopRenderer {
		fn queue(_scene: &Probe, _selection: NamedVisualLevel, _ctx: &mut VisualLodRenderContext) {}
	}

	fn world_box() -> LodHostBounds {
		LodHostBounds(Aabb3d::from_min_max(Vec3::splat(-50.0), Vec3::splat(50.0)))
	}

	#[test]
	fn projected_policy_is_stateless_and_far_is_coarser() {
		let probe = Probe { thresholds: ProjectedBoundsThresholds::default() };
		let bounds = world_box();
		let near = VisualLodView::test_perspective(
			Vec3::new(0.0, 20.0, 40.0),
			Vec3::ZERO,
			Vec2::new(1280.0, 720.0),
			std::f32::consts::FRAC_PI_3,
		);
		let far = VisualLodView::test_perspective(
			Vec3::new(0.0, 1_500.0, 3_000.0),
			Vec3::ZERO,
			Vec2::new(1280.0, 720.0),
			std::f32::consts::FRAC_PI_3,
		);
		let near_sel = ProjectedBoundsPolicy::select(&probe, &near, &bounds);
		let far_sel = ProjectedBoundsPolicy::select(&probe, &far, &bounds);
		assert_ne!(near_sel, far_sel);
		assert!(matches!(far_sel, NamedVisualLevel::UltraLow | NamedVisualLevel::Low));
		assert!(matches!(near_sel, NamedVisualLevel::Medium | NamedVisualLevel::High));
	}

	#[test]
	fn thresholds_partition_error() {
		let t = ProjectedBoundsThresholds { ultra_low: 10.0, low: 20.0, medium: 30.0 };
		assert_eq!(t.select(5.0), NamedVisualLevel::UltraLow);
		assert_eq!(t.select(15.0), NamedVisualLevel::Low);
		assert_eq!(t.select(25.0), NamedVisualLevel::Medium);
		assert_eq!(t.select(40.0), NamedVisualLevel::High);
	}

	#[test]
	fn named_bands_map_to_scene_levels() {
		assert_eq!(NamedVisualLevel::High.to_scene_level(), crate::LodSceneLevel::High);
		assert_eq!(NamedVisualLevel::Medium.to_scene_level(), crate::LodSceneLevel::Medium);
		assert_eq!(NamedVisualLevel::Low.to_scene_level(), crate::LodSceneLevel::Low);
		assert_eq!(NamedVisualLevel::UltraLow.to_scene_level(), crate::LodSceneLevel::UltraLow);
	}

	#[test]
	fn packed_clamp_and_finest_view() {
		assert_eq!(NamedVisualLevel::High.clamp_to_packed(), NamedVisualLevel::Medium);
		assert_eq!(
			NamedVisualLevel::UltraLow.max(NamedVisualLevel::Medium),
			NamedVisualLevel::Medium
		);
	}

	#[test]
	fn banded_indexes_named_levels() {
		let mut bands = Banded { ultra_low: 0, low: 1, medium: 2, high: 3 };
		assert_eq!(*bands.for_level(NamedVisualLevel::Low), 1);
		*bands.for_level_mut(NamedVisualLevel::High) = 9;
		assert_eq!(bands.high, 9);
	}

	#[test]
	fn visual_instance_scene_for_skips_empty_band() {
		let mut instance = VisualInstance::new(MaterialRef::default(), Affine3A::IDENTITY);
		instance.scenes.low = Some(SceneRef::glb("vegetation/test.glb"));
		assert!(instance.scene_for(NamedVisualLevel::UltraLow).is_none());
		assert_eq!(
			instance.scene_for(NamedVisualLevel::Low).map(|s| s.path.as_str()),
			Some("vegetation/test.glb")
		);
	}
}
