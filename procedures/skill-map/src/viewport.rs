use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, TextureFormat, TextureUsages};
use std::collections::HashMap;

use crate::{ReifiedSkillMapId, SkillMapRenderLayer};

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SkillMapViewportId(u32);

#[derive(Component)]
pub struct SkillMapViewport;

#[derive(Component)]
pub struct SkillMapViewportCamera;

#[derive(Resource, Default)]
pub struct SkillMapViewports {
	/// We store the SkillMap to viewport entity mapping.
	viewport_id_to_entities: HashMap<SkillMapViewportId, (Entity, Entity)>,
}

pub struct SkillMapViewportPlugin;

impl SkillMapViewportPlugin {
	/// When spawned, this function is called to register a skillmap to a viewport.
	///
	/// It can also be called to update the viewport to point to a new skillmap.
	pub fn register_skillmap_to_viewport(
		mut commands: Commands,
		mut images: ResMut<Assets<Image>>,
		mut skillmap_viewports: ResMut<SkillMapViewports>,
		dispatch_query: Query<
			(
				Entity,
				&SkillMapViewportId,
				&ReifiedSkillMapId,
				&SkillMapRenderLayer,
				Option<&Transform>,
				Option<&Node>,
				Option<&BorderColor>,
			),
			Added<ReifiedSkillMapId>,
		>,
	) {
		for (_entity, viewport_id, _skillmap_id, render_layer, transform, node, border_color) in
			&dispatch_query
		{
			// Get or create the camera and viewport entities.
			let (camera, viewport) =
				match skillmap_viewports.viewport_id_to_entities.get(viewport_id) {
					Some((camera_entity, viewport_entity)) => {
						(camera_entity.clone(), viewport_entity.clone())
					}
					None => {
						let mut image = Image::new_uninit(
							default(),
							TextureDimension::D2,
							TextureFormat::Bgra8UnormSrgb,
							RenderAssetUsages::all(),
						);
						image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
							| TextureUsages::COPY_DST
							| TextureUsages::RENDER_ATTACHMENT;
						let image_handle = images.add(image);
						let camera = commands
							.spawn((
								Camera2d::default(),
								Camera {
									// Render this camera before our UI camera
									order: -1,
									target: RenderTarget::Image(image_handle.clone().into()),
									..default()
								},
							))
							.id();

						let viewport = commands.spawn((
							Node {
								position_type: PositionType::Absolute,
								top: px(50),
								left: px(50),
								width: px(200),
								height: px(200),
								border: UiRect::all(px(5)),
								..default()
							},
							BorderColor::all(Color::WHITE),
							ViewportNode::new(camera),
						));

						skillmap_viewports
							.viewport_id_to_entities
							.insert(viewport_id.clone(), (camera, viewport.id()));

						(camera, viewport.id())
					}
				};

			// Update the camera render layer
			commands.entity(camera).insert(render_layer.0.clone());

			// Update the camera transform if there is one
			if let Some(transform) = transform {
				commands.entity(camera).insert(*transform);
			}

			// Update the the viewport node if there is one
			if let Some(node) = node {
				commands.entity(viewport).insert(node.clone());
			}

			// Update the the viewport border color if there is one
			if let Some(border_color) = border_color {
				commands.entity(viewport).insert(*border_color);
			}
		}
	}
}

impl Plugin for SkillMapViewportPlugin {
	fn build(&self, app: &mut App) {
		app.world_mut().commands().insert_resource(SkillMapViewports::default());
		app.add_systems(Update, Self::register_skillmap_to_viewport);
	}
}
