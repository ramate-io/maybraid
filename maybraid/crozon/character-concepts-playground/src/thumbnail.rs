use std::collections::HashMap;

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use crozon_characters::species::braidman::BraidmanColor;

use crate::ui::{thumbnail_image, UiAssetTarget};

const THUMBNAIL_SPACING: f32 = 8.0;

#[derive(Resource, Default)]
pub struct ThumbnailCache {
	entries: HashMap<ThumbnailKey, ThumbnailEntry>,
	next_slot: u32,
	revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ThumbnailKey {
	target: UiAssetTarget,
	color: BraidmanColor,
}

#[derive(Clone, Copy)]
struct ThumbnailEntry {
	camera: Entity,
	last_seen_revision: u64,
}

#[derive(Component)]
pub struct ThumbnailPreview {
	pub color: BraidmanColor,
}

impl ThumbnailCache {
	pub fn begin_ui_rebuild(&mut self) {
		self.revision += 1;
	}
}

pub fn camera_for_target(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	asset_server: &AssetServer,
	cache: &mut ThumbnailCache,
	target: UiAssetTarget,
	path: &'static str,
	color: BraidmanColor,
) -> Entity {
	let key = ThumbnailKey { target, color };
	if let Some(entry) = cache.entries.get_mut(&key) {
		entry.last_seen_revision = cache.revision;
		return entry.camera;
	}

	let slot = cache.next_slot;
	cache.next_slot += 1;
	let base = Vec3::new(4000.0 + slot as f32 * THUMBNAIL_SPACING, 0.0, 0.0);
	let image = images.add(thumbnail_image());
	let camera_transform =
		Transform::from_translation(base + Vec3::new(0.0, 0.45, 1.55)).looking_at(base, Vec3::Y);
	let camera = commands
		.spawn((
			Camera3d::default(),
			Camera { order: -1, ..default() },
			RenderTarget::Image(image.into()),
			Projection::Perspective(PerspectiveProjection {
				fov: 0.75,
				near: 0.05,
				far: 10.0,
				..default()
			}),
			camera_transform,
			Name::new(format!("thumbnail_camera_{}", target.label())),
		))
		.id();

	commands.spawn((
		SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path))),
		Transform::from_translation(base),
		ThumbnailPreview { color },
		Name::new(format!("thumbnail_asset_{}", target.label())),
	));

	commands.spawn((
		PointLight { intensity: 450.0, range: 6.0, shadows_enabled: false, ..default() },
		Transform::from_translation(base + Vec3::new(0.3, 1.8, 1.6)),
		Name::new(format!("thumbnail_light_{}", target.label())),
	));

	cache
		.entries
		.insert(key, ThumbnailEntry { camera, last_seen_revision: cache.revision });
	camera
}

pub fn sync_thumbnail_camera_activity(cache: Res<ThumbnailCache>, mut cameras: Query<&mut Camera>) {
	for entry in cache.entries.values() {
		let Ok(mut camera) = cameras.get_mut(entry.camera) else {
			continue;
		};
		camera.is_active = entry.last_seen_revision == cache.revision;
	}
}
