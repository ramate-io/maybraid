//! Bounded once-per-second policy inputs; no admission decisions or hot text.
use bevy::{camera::visibility::RenderLayers, prelude::*};
use chico::packed::PackedViewPolicy;
const MAX_CAMERAS: usize = 8;
const MAX_LIGHTS: usize = 16;
const MAX_LAYER_BLOCKS: usize = 4;

pub(crate) fn record(world: &mut World, policy: &PackedViewPolicy) {
	bevy::log::tracing::trace!(target:"maybraid.packed.metrics",
        supported=policy.supported(),reason=policy.reason(),cameras_3d=policy.cameras_3d,
        active_cameras_3d=policy.active_cameras_3d,scene_cameras=policy.scene_cameras,
        scene_camera=?policy.scene_camera.map(Entity::to_bits),scene_layers=?policy.scene_layers.as_ref().map(|layers|&layers.bits()[..layers.bits().len().min(MAX_LAYER_BLOCKS)]),
        needs_prepass=policy.needs_prepass,depth_prepass=policy.depth_prepass,normal_prepass=policy.normal_prepass,
        shadow_directional=?policy.shadow_directional.map(Entity::to_bits),shadow_directionals_total=policy.shadow_directionals_total,
        shadow_directionals_relevant=policy.shadow_directionals_relevant,
        shadow_points_total=policy.shadow_points_total,shadow_points_relevant=policy.shadow_points_relevant,
        shadow_spots_total=policy.shadow_spots_total,shadow_spots_relevant=policy.shadow_spots_relevant,
        scope="active grove-layer Camera3d; shadow-light relevance uses sole scene camera full layers; no energy/visibility relaxation",
        "packed_view_policy");
	let mut cameras = Vec::new();
	let mut camera_count = 0;
	for (entity,camera,layers,target) in world.query_filtered::<(Entity,&Camera,Option<&RenderLayers>,Option<&bevy::camera::RenderTarget>),With<Camera3d>>().iter(world) {
        camera_count+=1;
        let bits=entity.to_bits();
        if cameras.len()==MAX_CAMERAS && cameras.last().is_some_and(|row: &(u64,bool,bool,RenderLayers,String,Option<UVec2>,Option<UVec2>)|row.0<=bits) {continue;}
        let row=(bits,camera.is_active,PackedViewPolicy::is_scene_camera(camera,layers),layers.cloned().unwrap_or_default(),format!("{target:?}").chars().take(160).collect::<String>(),camera.physical_viewport_size(),camera.physical_target_size());
        let index=cameras.partition_point(|row|row.0<bits);cameras.insert(index,row);cameras.truncate(MAX_CAMERAS);
    }
	for (entity, active, relevant, layers, target, viewport, size) in &cameras {
		bevy::log::tracing::trace!(target:"maybraid.packed.metrics",entity=*entity,active=*active,scene_camera_candidate=*relevant,
            layers_bits=?&layers.bits()[..layers.bits().len().min(MAX_LAYER_BLOCKS)],layers_blocks=layers.bits().len(),layers_truncated=layers.bits().len()>MAX_LAYER_BLOCKS,
            render_target=target.as_str(),physical_viewport=?viewport,physical_target=?size,"packed_policy_camera");
	}
	let mut lights = Vec::new();
	let mut light_count = 0;
	for (entity, directional, point, spot, layers, view_visible, inherited, name) in world
		.query_filtered::<(
			Entity,
			Option<&DirectionalLight>,
			Option<&PointLight>,
			Option<&SpotLight>,
			Option<&RenderLayers>,
			Option<&ViewVisibility>,
			Option<&InheritedVisibility>,
			Option<&Name>,
		), Or<(With<DirectionalLight>, With<PointLight>, With<SpotLight>)>>()
		.iter(world)
	{
		light_count += 1;
		let bits = entity.to_bits();
		if lights.len() == MAX_LIGHTS
			&& lights.last().is_some_and(
				|row: &(
					u64,
					&str,
					bool,
					f32,
					bool,
					RenderLayers,
					Option<bool>,
					Option<bool>,
					String,
				)| row.0 <= bits,
			) {
			continue;
		}
		let (kind, shadows, energy) = if let Some(light) = directional {
			("directional", light.shadow_maps_enabled, light.illuminance)
		} else if let Some(light) = point {
			("point", light.shadow_maps_enabled, light.intensity)
		} else if let Some(light) = spot {
			("spot", light.shadow_maps_enabled, light.intensity)
		} else {
			continue;
		};
		let row = (
			bits,
			kind,
			shadows,
			energy,
			policy.light_is_relevant(layers),
			layers.cloned().unwrap_or_default(),
			view_visible.map(|value| value.get()),
			inherited.map(|value| value.get()),
			name.map(|name| name.as_str().chars().take(72).collect::<String>())
				.unwrap_or_default(),
		);
		let index = lights.partition_point(|row| row.0 < bits);
		lights.insert(index, row);
		lights.truncate(MAX_LIGHTS);
	}
	for (entity, kind, shadows, energy, relevant, layers, view_visible, inherited, name) in &lights
	{
		bevy::log::tracing::trace!(target:"maybraid.packed.metrics",entity=*entity,kind=*kind,name=name.as_str(),shadow_maps_enabled=*shadows,energy=*energy,
            relevant_to_sole_scene_camera=*relevant,layers_bits=?&layers.bits()[..layers.bits().len().min(MAX_LAYER_BLOCKS)],layers_blocks=layers.bits().len(),layers_truncated=layers.bits().len()>MAX_LAYER_BLOCKS,
            view_visible=?view_visible,inherited_visible=?inherited,"packed_policy_light");
	}
	bevy::log::tracing::trace!(target:"maybraid.packed.metrics",cameras_total=camera_count,cameras_emitted=cameras.len(),cameras_truncated=camera_count>MAX_CAMERAS,
        lights_total=light_count,lights_emitted=lights.len(),lights_truncated=light_count>MAX_LIGHTS,"packed_policy_complete");
}
