//! Magenta occupancy overlay for `/parting debug mask`.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::light::NotShadowCaster;
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use vegetation_grass_field::{
	ExtractedPartingOccluder, GrassFieldSettings, GrassPartingDebug, GrassPartingOccluder,
	PartingMask, PARTING_MASK_EXTENT_M, PARTING_MASK_HEIGHT, PARTING_MASK_WIDTH,
};

/// Above the 1.8 m capsule so a top-down still sees the disc, not a punched hole.
const OVERLAY_Y_M: f32 = 1.95;

#[derive(Component)]
struct MaskOverlay;

#[derive(Resource)]
struct MaskOverlayHandles {
	image: Handle<Image>,
}

pub struct MaskOverlayPlugin;

impl Plugin for MaskOverlayPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, spawn_mask_overlay)
			.add_systems(Update, sync_mask_overlay);
	}
}

fn spawn_mask_overlay(
	mut commands: Commands,
	mut images: ResMut<Assets<Image>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let handle = images.add(occupancy_image(&PartingMask::default_size()));
	let mesh =
		meshes.add(Plane3d::default().mesh().size(PARTING_MASK_EXTENT_M, PARTING_MASK_EXTENT_M));
	let material = materials.add(StandardMaterial {
		base_color: Color::WHITE,
		base_color_texture: Some(handle.clone()),
		alpha_mode: AlphaMode::Blend,
		unlit: true,
		double_sided: true,
		cull_mode: None,
		..default()
	});
	commands.spawn((
		Name::new("parting-mask-overlay"),
		MaskOverlay,
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_xyz(0.0, OVERLAY_Y_M, 0.0),
		Visibility::Hidden,
		NotShadowCaster,
	));
	commands.insert_resource(MaskOverlayHandles { image: handle });
}

fn sync_mask_overlay(
	debug: Res<GrassPartingDebug>,
	settings: Res<GrassFieldSettings>,
	handles: Option<Res<MaskOverlayHandles>>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
	occluders: Query<(&GrassPartingOccluder, &GlobalTransform)>,
	mut overlay: Query<(&mut Transform, &mut Visibility), With<MaskOverlay>>,
	mut images: ResMut<Assets<Image>>,
) {
	let Ok((mut transform, mut visibility)) = overlay.single_mut() else {
		return;
	};
	if !debug.mask_overlay {
		*visibility = Visibility::Hidden;
		return;
	}
	let Some(handles) = handles else {
		return;
	};
	let Ok(camera) = cameras.single() else {
		*visibility = Visibility::Hidden;
		return;
	};
	let origin = camera.translation().xz();
	let extracted: Vec<_> = occluders
		.iter()
		.map(|(occluder, transform)| {
			ExtractedPartingOccluder::from_transform(*occluder, *transform)
		})
		.collect();
	let mut mask = PartingMask::default_size();
	mask.splat(&extracted, origin, settings.parting);
	if let Some(mut image) = images.get_mut(&handles.image) {
		*image = occupancy_image(&mask);
	}
	*transform = Transform::from_xyz(origin.x, OVERLAY_Y_M, origin.y);
	*visibility = Visibility::Visible;
}

fn occupancy_image(mask: &PartingMask) -> Image {
	let data = mask.occupancy_preview_rgba8();
	debug_assert_eq!(data.len(), (PARTING_MASK_WIDTH * PARTING_MASK_HEIGHT * 4) as usize);
	let mut image = Image::new(
		Extent3d {
			width: PARTING_MASK_WIDTH,
			height: PARTING_MASK_HEIGHT,
			depth_or_array_layers: 1,
		},
		TextureDimension::D2,
		data,
		TextureFormat::Rgba8UnormSrgb,
		RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
	);
	image.sampler = ImageSampler::nearest();
	image
}
