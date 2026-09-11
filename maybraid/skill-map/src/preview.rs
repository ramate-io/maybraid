//! Off-screen catalog render of an authored map. Same tiles and shade as play.

use bevy::asset::RenderAssetUsages;
use bevy::camera::{ClearColorConfig, RenderTarget, ScalingMode};
use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, TextureFormat, TextureUsages};
use crozon_character_items::SkillMapSpec;

use crate::map::{authored_map, render_layer, MapExtents, SkillKind, SkillMapId};
use crate::tile_material::SkillMapTileAssets;
use crate::tiles::spawn_map_tiles_at;

/// Square starting target; [`bevy::ui::widget::ViewportNode`] resizes it to the cell.
pub const CATALOG_PREVIEW_PX: u32 = 160;
/// First [`SkillMapId`] reserved for menu previews (live play uses `0`).
pub const MENU_PREVIEW_ID_BASE: u32 = 16;

#[derive(Component, Clone, Copy, Debug)]
pub struct SkillMapMenuPreview {
	pub spec: SkillMapSpec,
}

/// Camera + image a catalog [`ViewportNode`] can bind.
#[derive(Clone, Debug)]
pub struct SkillMapCatalogPreview {
	pub host: Entity,
	pub camera: Entity,
	pub image: Handle<Image>,
}

/// World units visible along one catalog edge. Matches [`MapExtents`] (256).
fn catalog_world_span() -> f32 {
	let extents = MapExtents::default();
	extents.tile_size().x * extents.steps as f32
}

/// Camera + tiles into an image. Caller owns the handle and camera for UI.
pub fn spawn_skill_map_catalog_preview(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	tiles: &SkillMapTileAssets,
	spec: SkillMapSpec,
	slot: u32,
) -> SkillMapCatalogPreview {
	let id = SkillMapId(MENU_PREVIEW_ID_BASE + slot);
	let mut authored = authored_map(SkillKind::from_item(spec.kind), spec.seed);
	authored.id = id;
	let layer = render_layer(id);
	let host = commands
		.spawn((
			Name::new(format!("skill-map-catalog-{}", spec.kind.label())),
			SkillMapMenuPreview { spec },
		))
		.id();

	let mut image = Image::new_uninit(
		default(),
		TextureDimension::D2,
		TextureFormat::Bgra8UnormSrgb,
		RenderAssetUsages::all(),
	);
	image.texture_descriptor.usage =
		TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
	let image_handle = images.add(image);

	let mut projection = OrthographicProjection::default_2d();
	let span = catalog_world_span();
	projection.scaling_mode = ScalingMode::Fixed { width: span, height: span };
	projection.scale = 1.0;
	let camera = commands
		.spawn((
			Name::new(format!("skill-map-catalog-camera-{}", spec.kind.label())),
			Camera2d,
			Camera {
				order: -40 - slot as isize,
				clear_color: ClearColorConfig::Custom(authored.kind.viewport_clear()),
				..default()
			},
			RenderTarget::Image(image_handle.clone().into()),
			Projection::Orthographic(projection),
			Transform::from_xyz(0.0, 0.0, 1.0),
			id,
			layer,
			ChildOf(host),
		))
		.id();
	spawn_map_tiles_at(commands, authored, None, tiles, Vec2::ZERO, Some(host));
	SkillMapCatalogPreview { host, camera, image: image_handle }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn catalog_camera_fits_the_authored_extents() {
		let extents = MapExtents::default();
		let world = extents.tile_size().x * extents.steps as f32;
		assert!((catalog_world_span() - world).abs() < f32::EPSILON);
	}
}
