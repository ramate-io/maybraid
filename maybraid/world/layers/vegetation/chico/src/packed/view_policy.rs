//! Initial admission follows the sole scene camera's actual light layers.
//! Visibility/energy are intentionally not used to relax the shadow contract:
//! Bevy may still allocate shadow views, and visibility is updated after Update.
use bevy::{
	camera::visibility::RenderLayers,
	core_pipeline::prepass::{DepthPrepass, NormalPrepass},
	prelude::*,
};

#[derive(Clone, Debug, Default)]
pub struct PackedViewPolicy {
	pub cameras_3d: usize,
	pub active_cameras_3d: usize,
	pub scene_cameras: usize,
	pub scene_camera: Option<Entity>,
	pub scene_layers: Option<RenderLayers>,
	pub needs_prepass: bool,
	pub depth_prepass: bool,
	pub normal_prepass: bool,
	pub shadow_directional: Option<Entity>,
	pub shadow_directionals_total: usize,
	pub shadow_points_total: usize,
	pub shadow_spots_total: usize,
	pub shadow_directionals_relevant: usize,
	pub shadow_points_relevant: usize,
	pub shadow_spots_relevant: usize,
}
/// Required-pass ownership is stable across camera movement, not camera/pass changes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ReadinessPolicy {
	camera: Option<Entity>,
	layers: Option<RenderLayers>,
	depth_prepass: bool,
	normal_prepass: bool,
	shadow_directional: Option<Entity>,
}
impl PackedViewPolicy {
	pub(crate) fn readiness_key(&self) -> ReadinessPolicy {
		ReadinessPolicy {
			camera: self.scene_camera,
			layers: self.scene_layers.clone(),
			depth_prepass: self.depth_prepass,
			normal_prepass: self.normal_prepass,
			shadow_directional: self.shadow_directional,
		}
	}

	pub fn is_scene_camera(camera: &Camera, layers: Option<&RenderLayers>) -> bool {
		camera.is_active && layers.unwrap_or_default().intersects(&RenderLayers::default())
	}
	pub fn supported(&self) -> bool {
		self.scene_cameras == 1
			&& self.shadow_directionals_relevant <= 1
			&& self.shadow_points_relevant == 0
			&& self.shadow_spots_relevant == 0
	}
	pub fn reason(&self) -> &'static str {
		if self.scene_cameras != 1 {
			"scene_camera_count"
		} else if self.shadow_directionals_relevant > 1 {
			"multiple_relevant_shadow_directionals"
		} else if self.shadow_points_relevant > 0 {
			"relevant_shadow_points"
		} else if self.shadow_spots_relevant > 0 {
			"relevant_shadow_spots"
		} else {
			"supported"
		}
	}
	pub fn light_is_relevant(&self, layers: Option<&RenderLayers>) -> bool {
		self.scene_layers
			.as_ref()
			.is_some_and(|view| layers.unwrap_or_default().intersects(view))
	}
	pub fn read(world: &mut World) -> Self {
		let mut result = Self::default();
		for (entity,camera,layers,depth,normal) in world.query_filtered::<(Entity,&Camera,Option<&RenderLayers>,Has<DepthPrepass>,Has<NormalPrepass>),With<Camera3d>>().iter(world) {
            result.cameras_3d+=1;
            result.active_cameras_3d+=usize::from(camera.is_active);
            if !Self::is_scene_camera(camera,layers) { continue; }
            result.scene_cameras+=1;
            result.scene_camera=Some(entity);
            result.scene_layers=Some(layers.cloned().unwrap_or_default());
            result.needs_prepass=depth||normal;
            result.depth_prepass=depth;result.normal_prepass=normal;
        }
		if result.scene_cameras != 1 {
			result.scene_camera = None;
			result.scene_layers = None;
			result.needs_prepass = false;
			result.depth_prepass = false;
			result.normal_prepass = false;
		}
		for (entity, light, layers) in
			world.query::<(Entity, &DirectionalLight, Option<&RenderLayers>)>().iter(world)
		{
			if !light.shadow_maps_enabled {
				continue;
			}
			result.shadow_directionals_total += 1;
			if result.light_is_relevant(layers) {
				result.shadow_directionals_relevant += 1;
				result.shadow_directional = Some(entity);
			}
		}
		if result.shadow_directionals_relevant != 1 {
			result.shadow_directional = None;
		}
		for (light, layers) in world.query::<(&PointLight, Option<&RenderLayers>)>().iter(world) {
			if !light.shadow_maps_enabled {
				continue;
			}
			result.shadow_points_total += 1;
			result.shadow_points_relevant += usize::from(result.light_is_relevant(layers));
		}
		for (light, layers) in world.query::<(&SpotLight, Option<&RenderLayers>)>().iter(world) {
			if !light.shadow_maps_enabled {
				continue;
			}
			result.shadow_spots_total += 1;
			result.shadow_spots_relevant += usize::from(result.light_is_relevant(layers));
		}
		result
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn preview_key_with_proxy_layer_is_disjoint_from_world_but_not_camera_proxy_layer() {
		let mut world = World::new();
		let camera = world.spawn((Camera3d::default(), Camera::default())).id();
		world.spawn(DirectionalLight { shadow_maps_enabled: true, ..default() });
		let preview = world
			.spawn((
				DirectionalLight { shadow_maps_enabled: true, illuminance: 10000.0, ..default() },
				RenderLayers::layer(1).with(30),
			))
			.id();
		let policy = PackedViewPolicy::read(&mut world);
		assert_eq!(policy.shadow_directionals_total, 2);
		assert_eq!(policy.shadow_directionals_relevant, 1);
		assert!(policy.supported());
		world.entity_mut(preview).insert(RenderLayers::default());
		assert_eq!(
			PackedViewPolicy::read(&mut world).reason(),
			"multiple_relevant_shadow_directionals"
		);
		world.entity_mut(preview).insert(RenderLayers::layer(1).with(30));
		world.entity_mut(camera).insert(RenderLayers::default().with(30));
		assert_eq!(
			PackedViewPolicy::read(&mut world).reason(),
			"multiple_relevant_shadow_directionals"
		);
	}
	#[test]
	fn relevant_point_spot_and_zero_energy_shadow_lights_remain_conservative() {
		let mut world = World::new();
		world.spawn((Camera3d::default(), Camera::default()));
		let point = world
			.spawn((
				PointLight { shadow_maps_enabled: true, intensity: 0.0, ..default() },
				RenderLayers::layer(1),
			))
			.id();
		let spot = world
			.spawn((SpotLight { shadow_maps_enabled: true, ..default() }, RenderLayers::layer(1)))
			.id();
		assert!(PackedViewPolicy::read(&mut world).supported());
		world.entity_mut(point).insert(RenderLayers::default());
		assert_eq!(PackedViewPolicy::read(&mut world).reason(), "relevant_shadow_points");
		world.entity_mut(point).insert(RenderLayers::layer(1));
		world.entity_mut(spot).insert(RenderLayers::default());
		assert_eq!(PackedViewPolicy::read(&mut world).reason(), "relevant_shadow_spots");
		world.despawn(spot);
		world.spawn(DirectionalLight { shadow_maps_enabled: true, illuminance: 0.0, ..default() });
		world.spawn(DirectionalLight { shadow_maps_enabled: true, ..default() });
		assert_eq!(
			PackedViewPolicy::read(&mut world).reason(),
			"multiple_relevant_shadow_directionals"
		);
	}
	#[test]
	fn inactive_and_disjoint_camera_prepass_does_not_change_required_scene_passes() {
		let mut world = World::new();
		world.spawn((Camera3d::default(), Camera::default()));
		world.spawn((Camera3d::default(), Camera::default(), RenderLayers::layer(1), DepthPrepass));
		world.spawn((Camera3d::default(), Camera { is_active: false, ..default() }, DepthPrepass));
		let policy = PackedViewPolicy::read(&mut world);
		assert!(policy.supported());
		assert!(!policy.needs_prepass);
		assert_eq!(policy.cameras_3d, 3);
		assert_eq!(policy.active_cameras_3d, 2);
		assert_eq!(policy.scene_cameras, 1);
	}
}
