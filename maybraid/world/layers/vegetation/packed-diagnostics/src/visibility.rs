//! Bounded diagnostics only; never changes rendering or host visibility.
use bevy::camera::{
	primitives::{Aabb, Frustum, Sphere},
	visibility::{RenderLayers, VisibilityClass, VisibleEntities},
};
use bevy::ecs::{entity_disabling::Disabled, query::Allow};
use bevy::prelude::*;
use bevy::render::{extract_resource::ExtractResource, RenderApp};
use chico::packed::PackedBatchInstance;
use std::{
	any::TypeId,
	sync::atomic::Ordering,
	time::{Duration, Instant},
};

#[derive(Clone, Debug, Default)]
pub(super) struct PhaseCounts {
	pub total: u64,
	pub disabled: u64,
	pub own_hidden: u64,
	pub inherited_visible: u64,
	pub view_visible: u64,
	pub mesh_class: u64,
}
#[derive(Clone, Debug)]
#[allow(dead_code, reason = "Fields are emitted as bounded diagnostic evidence via Debug.")]
pub(super) struct NodeEvidence {
	pub entity: u64,
	pub disabled: bool,
	pub visibility: Option<Visibility>,
	pub inherited: Option<bool>,
	pub view_visible: Option<bool>,
	pub lod_host: bool,
	pub lod_roots: bool,
	pub lod_level_root: bool,
	pub retired_hcsg: bool,
	pub chico_source: bool,
}
impl NodeEvidence {
	fn read(world: &World, entity: Entity) -> Self {
		Self {
			entity: entity.to_bits(),
			disabled: world.get::<Disabled>(entity).is_some(),
			visibility: world.get::<Visibility>(entity).copied(),
			inherited: world.get::<InheritedVisibility>(entity).map(|v| v.get()),
			view_visible: world.get::<ViewVisibility>(entity).map(|v| v.get()),
			lod_host: world.get::<lod::LodSceneHost>(entity).is_some(),
			lod_roots: world.get::<lod::LodLevelRoots>(entity).is_some(),
			lod_level_root: world.get::<lod::LodLevelRoot>(entity).is_some(),
			retired_hcsg: world.get::<lod::hcsg::RetiredHost>(entity).is_some(),
			chico_source: world.get::<chico::ChicoGroveHost>(entity).is_some(),
		}
	}
}
#[derive(Clone, Debug)]
#[allow(dead_code, reason = "Fields are emitted as bounded diagnostic evidence via Debug.")]
pub(super) struct ProxyEvidence {
	pub entity: u64,
	pub cell: u64,
	pub ordinal: usize,
	pub phase: u8,
	pub lod: String,
	pub shadow_only: bool,
	pub layers: Vec<u64>,
	pub layer_match: bool,
	pub geometric_candidate: bool,
	pub native_sphere: Option<bool>,
	pub native_obb: Option<bool>,
	pub mesh_class: bool,
	pub main_view_visible: Option<bool>,
	pub world_translation: Option<[f32; 3]>,
	pub geometry_min: [f32; 3],
	pub geometry_max: [f32; 3],
	pub chain: Vec<NodeEvidence>,
	pub chain_truncated: bool,
}
#[derive(Clone, Debug, Default, Resource, ExtractResource)]
#[extract_app(RenderApp)]
pub(super) struct VisibilitySample {
	pub sequence: u64,
	pub main_frame: u32,
	pub phases: [PhaseCounts; 4],
	pub scene_cameras: usize,
	pub camera: Option<u64>,
	pub eligible: u64,
	pub eligible_disabled: u64,
	pub eligible_hidden: u64,
	pub eligible_inherited_hidden: u64,
	pub eligible_view_hidden: u64,
	pub selection_reason: &'static str,
	pub proxy: Option<ProxyEvidence>,
}
impl VisibilitySample {
	fn chain(world: &World, entity: Entity) -> (Vec<NodeEvidence>, bool) {
		let mut chain = Vec::new();
		let mut current = Some(entity);
		while let Some(entity) = current {
			if chain.len() == 12 {
				return (chain, true);
			}
			chain.push(NodeEvidence::read(world, entity));
			current = world.get::<ChildOf>(entity).map(ChildOf::parent);
		}
		(chain, false)
	}
	fn read(world: &mut World, sequence: u64) -> Self {
		let mut result = Self {
			sequence,
			main_frame: world
				.get_resource::<bevy::diagnostic::FrameCount>()
				.map_or(0, |frame| frame.0),
			..Default::default()
		};
		let mut cameras=world.query_filtered::<(Entity,&Camera,&GlobalTransform,Option<&Frustum>,Option<&RenderLayers>),With<Camera3d>>().iter(world).filter(|(_,camera,_,_,layers)|camera.is_active&&layers.is_none_or(|layers|layers.intersects(&RenderLayers::default()))).map(|(entity,_,pose,frustum,layers)|(entity,pose.translation(),frustum.cloned(),layers.cloned().unwrap_or_default())).collect::<Vec<_>>();
		cameras.sort_by_key(|(entity, _, _, _)| entity.to_bits());
		result.scene_cameras = cameras.len();
		let camera = cameras.first();
		result.camera = camera.map(|(entity, _, _, _)| entity.to_bits());
		let mut best: Option<((bool, f32, u64, usize, u64), Entity)> = None;
		let mut query = world.query_filtered::<(
			Entity,
			&PackedBatchInstance,
			Has<Disabled>,
			Option<&Visibility>,
			Option<&InheritedVisibility>,
			Option<&ViewVisibility>,
			Option<&VisibilityClass>,
			Option<&RenderLayers>,
		), Allow<Disabled>>();
		for (entity, batch, disabled, visibility, inherited, view, classes, layers) in
			query.iter(world)
		{
			let phase = batch.signal.phase.load(Ordering::Acquire);
			let counts = &mut result.phases[usize::from(phase).min(3)];
			counts.total += 1;
			counts.disabled += u64::from(disabled);
			counts.own_hidden += u64::from(visibility == Some(&Visibility::Hidden));
			counts.inherited_visible += u64::from(inherited.is_some_and(|v| v.get()));
			counts.view_visible += u64::from(view.is_some_and(|v| v.get()));
			counts.mesh_class += u64::from(
				classes.is_some_and(|classes| classes.0.contains(&TypeId::of::<Mesh3d>())),
			);
			let Some((_, eye, frustum, camera_layers)) = camera else { continue };
			if phase != 1
				|| batch.shadow_only
				|| !layers.is_none_or(|layers| layers.intersects(camera_layers))
			{
				continue;
			}
			let b = batch.payload.bounds;
			let aabb = Aabb { center: (b.min + b.max) * 0.5, half_extents: (b.max - b.min) * 0.5 };
			let candidate = frustum.as_ref().is_some_and(|frustum| {
				frustum.intersects_obb(&aabb, &bevy::math::Affine3A::IDENTITY, true, false)
			});
			if candidate {
				result.eligible += 1;
				result.eligible_disabled += u64::from(disabled);
				result.eligible_hidden += u64::from(visibility == Some(&Visibility::Hidden));
				result.eligible_inherited_hidden += u64::from(!inherited.is_some_and(|v| v.get()));
				result.eligible_view_hidden += u64::from(!view.is_some_and(|v| v.get()));
			}
			let distance = eye.distance_squared(Vec3::from(aabb.center));
			let key = (!candidate, distance, batch.cell.to_bits(), batch.ordinal, entity.to_bits());
			if best.as_ref().is_none_or(|(old, _)| {
				key.0
					.cmp(&old.0)
					.then(key.1.total_cmp(&old.1))
					.then(key.2.cmp(&old.2))
					.then(key.3.cmp(&old.3))
					.then(key.4.cmp(&old.4))
					.is_lt()
			}) {
				best = Some((key, entity));
			}
		}
		result.selection_reason = if camera.is_none() {
			"no_active_scene_camera"
		} else if result.eligible > 0 {
			"active_nonshadow_layer_match_geometric_frustum_candidate"
		} else {
			"no_geometric_candidate; active_nonshadow_layer_match_fallback"
		};
		if let Some((_, entity)) = best {
			let batch = world.get::<PackedBatchInstance>(entity);
			if let (Some(batch), Some((camera, _, frustum, camera_layers))) = (batch, camera) {
				let (chain, chain_truncated) = Self::chain(world, entity);
				let layers = world.get::<RenderLayers>(entity).cloned().unwrap_or_default();
				let geometry = batch.payload.bounds;
				let geometric_aabb = Aabb {
					center: (geometry.min + geometry.max) * 0.5,
					half_extents: (geometry.max - geometry.min) * 0.5,
				};
				let global = world.get::<GlobalTransform>(entity);
				let native = world.get::<Aabb>(entity);
				let native_sphere = frustum.as_ref().zip(global).zip(native).map(|((f, t), a)| {
					f.intersects_sphere(
						&Sphere {
							center: t.affine().transform_point3a(a.center),
							radius: t.radius_vec3a(a.half_extents),
						},
						false,
					)
				});
				let native_obb = frustum
					.as_ref()
					.zip(global)
					.zip(native)
					.map(|((f, t), a)| f.intersects_obb(a, &t.affine(), true, false));
				result.proxy = Some(ProxyEvidence {
					entity: entity.to_bits(),
					cell: batch.cell.to_bits(),
					ordinal: batch.ordinal,
					phase: batch.signal.phase.load(Ordering::Acquire),
					lod: format!("{:?}", batch.level),
					shadow_only: batch.shadow_only,
					layers: layers.bits().iter().copied().collect(),
					layer_match: layers.intersects(camera_layers),
					geometric_candidate: frustum.as_ref().is_some_and(|f| {
						f.intersects_obb(
							&geometric_aabb,
							&bevy::math::Affine3A::IDENTITY,
							true,
							false,
						)
					}),
					native_sphere,
					native_obb,
					mesh_class: world
						.get::<VisibilityClass>(entity)
						.is_some_and(|classes| classes.0.contains(&TypeId::of::<Mesh3d>())),
					main_view_visible: world
						.get::<VisibleEntities>(*camera)
						.map(|visible| visible.get(TypeId::of::<Mesh3d>()).contains(&entity)),
					world_translation: global.map(|p| p.translation().to_array()),
					geometry_min: Vec3::from(geometry.min).to_array(),
					geometry_max: Vec3::from(geometry.max).to_array(),
					chain,
					chain_truncated,
				});
			}
		}
		result
	}
}
pub(super) fn record(world: &mut World, mut previous: Local<Option<Instant>>) {
	let now = Instant::now();
	if previous.is_some_and(|last| now.duration_since(last) < Duration::from_secs(1)) {
		return;
	}
	*previous = Some(now);
	let sequence = world.resource::<VisibilitySample>().sequence + 1;
	let sample = VisibilitySample::read(world, sequence);
	bevy::log::tracing::trace!(target:"maybraid.packed.metrics",sample=sample.sequence,main_frame=sample.main_frame,scene_cameras=sample.scene_cameras,camera=?sample.camera,preparing=?sample.phases[0],active=?sample.phases[1],retired=?sample.phases[2],other=?sample.phases[3],eligible=sample.eligible,eligible_disabled=sample.eligible_disabled,eligible_hidden=sample.eligible_hidden,eligible_inherited_hidden=sample.eligible_inherited_hidden,eligible_view_hidden=sample.eligible_view_hidden,selection_reason=sample.selection_reason,proxy=?sample.proxy,scope="Allow<Disabled>; own flags distinct from ancestry; Last after visibility; raw geometric tests are not final engine eligibility", "packed_visibility_sample");
	world.insert_resource(sample);
}

#[cfg(test)]
mod tests {
	use crate::visibility::VisibilitySample;
	use bevy::{ecs::entity_disabling::Disabled, prelude::*};
	#[test]
	fn ancestry_distinguishes_disabled_parent_from_child_and_caps_evidence() -> Result<(), String> {
		let mut world = World::new();
		world.register_disabling_component::<Disabled>();
		let parent = world.spawn((Visibility::Hidden, InheritedVisibility::HIDDEN, Disabled)).id();
		let child = world
			.spawn((Visibility::Inherited, InheritedVisibility::HIDDEN, ChildOf(parent)))
			.id();
		assert_eq!(world.query::<&Visibility>().iter(&world).count(), 1);
		assert_eq!(
			world
				.query_filtered::<&Visibility, bevy::ecs::query::Allow<Disabled>>()
				.iter(&world)
				.count(),
			2
		);
		let (chain, truncated) = VisibilitySample::chain(&world, child);
		assert_eq!(chain.len(), 2);
		assert!(!truncated);
		assert!(!chain[0].disabled);
		assert!(chain[1].disabled);
		assert_eq!(chain[0].inherited, Some(false));
		assert_eq!(chain[1].visibility, Some(Visibility::Hidden));
		world.entity_mut(child).insert(Disabled);
		assert!(VisibilitySample::chain(&world, child).0[0].disabled);
		let mut leaf = child;
		for _ in 0..20 {
			leaf = world.spawn(ChildOf(leaf)).id();
		}
		let (chain, truncated) = VisibilitySample::chain(&world, leaf);
		assert_eq!(chain.len(), 12);
		assert!(truncated);
		Ok(())
	}
}
