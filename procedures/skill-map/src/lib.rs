use bevy::asset::RenderAssetUsages;
use bevy::camera::ImageRenderTarget;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{
	Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

#[derive(Component, Clone)]
pub struct SkillMapRenderTarget(pub Handle<Image>);

#[derive(Resource, Clone)]
pub struct SkillMapPlugin {
	skill_map_label: String,
}

impl SkillMapPlugin {
	fn create_skillmap_render_target(mut images: ResMut<Assets<Image>>, mut commands: Commands) {
		let size = Extent3d { width: 512, height: 512, depth_or_array_layers: 1 };

		let mut image = Image {
			texture_descriptor: TextureDescriptor {
				label: Some("skillmap_render_target"),
				size,
				dimension: TextureDimension::D2,
				format: TextureFormat::Bgra8UnormSrgb,
				mip_level_count: 1,
				sample_count: 1,
				usage: TextureUsages::TEXTURE_BINDING
					| TextureUsages::COPY_DST
					| TextureUsages::RENDER_ATTACHMENT,
				view_formats: &[TextureFormat::Bgra8UnormSrgb].as_slice(),
			},
			..default()
		};

		image.resize(size);
		let handle = images.add(image);

		commands.spawn(SkillMapRenderTarget(handle));
	}

	fn create_skillmap_camera(
		mut commands: Commands,
		mut images: ResMut<Assets<Image>>,
		query: Query<&SkillMapRenderTarget>,
	) {
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

		commands.spawn((
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
	}

	fn build_skillmap_subapp(app: &mut App) {
		let mut sub_app = SubApp::new();

		sub_app
			.add_plugins(MinimalPlugins)
			.add_plugins(AssetPlugin::default()) // needed for sprite textures
			.add_systems(Startup, skillmap_setup)
			.add_systems(Update, skillmap_update);

		app.add_sub_app("skillmap", sub_app);
	}
}

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_systems(Startup, create_skillmap_render_target)
		.add_systems(Startup, build_skillmap_subapp)
		.add_systems(Startup, setup_ui)
		.run();
}
