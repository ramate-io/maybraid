//! Bevy presenter: furniture shaders as a blended backdrop, HUD star, menu label.

use std::f32::consts::PI;

use bevy::asset::RenderAssetUsages;
use bevy::audio::AudioPlugin;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget, ScalingMode};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::time::Virtual;
use bevy::window::{PresentMode, PrimaryWindow};
use furniture_assemblies::palette::{cosmos, rockadder};
use furniture_shaders::{FurnitureShadersPlugin, FurnitureSurfaceMaterial};
use menu_components::icons::maybraid::Icon;
use menu_components::theme::{
	BARLOW_BLACK, COLUMN_INSET, HEADER_FONT_SIZE, HEADER_MARGIN_BOTTOM, MAYBRAID_LOGO, TEXT_YELLOW,
};

use crate::timeline::{TitleSequence, STAR_SIZE_PX, TRAIL_BEADS};

const ROCK_LAYER: usize = 1;
const COSMOS_LAYER: usize = 2;

/// Window + capture configuration for [`TitleSequencePlugin`].
#[derive(Resource, Clone, Debug)]
pub struct TitleSequenceConfig {
	pub sequence: TitleSequence,
	pub width: u32,
	pub height: u32,
	pub fps: u32,
	pub capture: bool,
}

impl TitleSequenceConfig {
	pub fn live(sequence: TitleSequence, width: u32, height: u32, fps: u32) -> Self {
		Self { sequence, width, height, fps, capture: false }
	}
}

/// Sequence clock. Capture writes this explicitly; live preview follows [`Time`].
#[derive(Resource, Clone, Copy, Debug)]
pub struct SequenceClock {
	pub time: f32,
}

#[derive(Component)]
struct CosmosLayer;

#[derive(Component)]
struct TitleStar;

#[derive(Component)]
struct TitleTrail {
	index: usize,
}

#[derive(Component)]
struct TitleLabel;

/// Present the sampled timeline after the clock is written.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TitleSequenceSystems {
	Clock,
	ShaderTime,
	Present,
}

/// Spawns the title scene and presents [`TitleSequence`] each frame.
pub struct TitleSequencePlugin {
	pub config: TitleSequenceConfig,
}

impl Plugin for TitleSequencePlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(self.config.clone())
			.insert_resource(SequenceClock { time: 0.0 })
			.add_plugins(FurnitureShadersPlugin)
			.configure_sets(
				Update,
				(
					TitleSequenceSystems::Clock,
					TitleSequenceSystems::ShaderTime,
					TitleSequenceSystems::Present,
				)
					.chain(),
			)
			.add_systems(Startup, setup_title_scene)
			.add_systems(Update, sync_clock.in_set(TitleSequenceSystems::Clock))
			.add_systems(Update, sync_shader_time.in_set(TitleSequenceSystems::ShaderTime))
			.add_systems(Update, present_title.in_set(TitleSequenceSystems::Present));
	}
}

/// DefaultPlugins tuned for a fixed-size title window.
pub fn window_plugins(config: &TitleSequenceConfig, assets: impl Into<String>) -> impl PluginGroup {
	DefaultPlugins
		.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Maybraid title sequence".into(),
				resolution: (config.width, config.height).into(),
				resizable: false,
				decorations: !config.capture,
				present_mode: PresentMode::AutoNoVsync,
				..default()
			}),
			..default()
		})
		.set(AssetPlugin { file_path: assets.into(), ..default() })
		.disable::<AudioPlugin>()
}

fn setup_title_scene(
	mut commands: Commands,
	mut images: ResMut<Assets<Image>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<FurnitureSurfaceMaterial>>,
	config: Res<TitleSequenceConfig>,
	asset_server: Res<AssetServer>,
) {
	let rock_image = render_target(&mut images, config.width, config.height);
	let cosmos_image = render_target(&mut images, config.width, config.height);
	let rock_layer = RenderLayers::layer(ROCK_LAYER);
	let cosmos_layer = RenderLayers::layer(COSMOS_LAYER);

	let rock_mat = materials.add(FurnitureSurfaceMaterial::from_material_ref(&rockadder(3, 1)));
	let cosmos_mat = materials.add(FurnitureSurfaceMaterial::from_material_ref(&cosmos(3, 1)));
	let plane = meshes.add(Rectangle::from_size(Vec2::splat(40.0)));

	commands.insert_resource(GlobalAmbientLight {
		brightness: 620.0,
		color: Color::srgb(1.0, 0.92, 0.82),
		..default()
	});
	spawn_shader_booth(
		&mut commands,
		plane.clone(),
		rock_mat,
		rock_image.clone(),
		rock_layer,
		-2,
		"rockadder",
	);
	spawn_shader_booth(
		&mut commands,
		plane,
		cosmos_mat,
		cosmos_image.clone(),
		cosmos_layer,
		-1,
		"cosmos",
	);

	commands.spawn((
		Camera2d,
		Camera { order: 0, clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
		IsDefaultUiCamera,
	));

	let logo = asset_server.load(MAYBRAID_LOGO);
	let font = asset_server.load(BARLOW_BLACK);
	commands
		.spawn((
			Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
			Pickable::IGNORE,
		))
		.with_children(|root| {
			root.spawn((
				ImageNode { image: rock_image, ..default() },
				fullscreen_abs(),
				Pickable::IGNORE,
			));
			root.spawn((
				CosmosLayer,
				ImageNode { image: cosmos_image, color: Color::WHITE.with_alpha(0.5), ..default() },
				fullscreen_abs(),
				Pickable::IGNORE,
			));
			for index in 0..TRAIL_BEADS {
				root.spawn((
					TitleTrail { index },
					ImageNode {
						image: logo.clone(),
						color: TEXT_YELLOW.with_alpha(0.0),
						..default()
					},
					mark_node(-STAR_SIZE_PX, -STAR_SIZE_PX, STAR_SIZE_PX),
					UiTransform::default(),
					Visibility::Hidden,
					Pickable::IGNORE,
				));
			}
			root.spawn((
				TitleStar,
				mark_node(-STAR_SIZE_PX, -STAR_SIZE_PX, STAR_SIZE_PX),
				UiTransform::default(),
				Visibility::Hidden,
				Pickable::IGNORE,
			))
			.with_children(|star| {
				Icon::maybraid(STAR_SIZE_PX, TEXT_YELLOW).spawn(
					star,
					logo.clone(),
					Visibility::Inherited,
				);
			});
			// Same face as `TextMenuHeader`: Barlow Black, header size, yellow.
			root.spawn((
				TitleLabel,
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(COLUMN_INSET),
					bottom: Val::Px(HEADER_MARGIN_BOTTOM + 72.0),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|label| {
				label.spawn((
					Text::new(config.sequence.label.clone()),
					TextFont {
						font: font.clone().into(),
						font_size: FontSize::Px(HEADER_FONT_SIZE),
						..default()
					},
					TextColor(TEXT_YELLOW.with_alpha(0.0)),
					Pickable::IGNORE,
				));
			});
		});
}

fn fullscreen_abs() -> Node {
	Node {
		position_type: PositionType::Absolute,
		width: Val::Percent(100.0),
		height: Val::Percent(100.0),
		left: Val::Px(0.0),
		top: Val::Px(0.0),
		..default()
	}
}

fn mark_node(left: f32, top: f32, size: f32) -> Node {
	Node {
		position_type: PositionType::Absolute,
		width: Val::Px(size),
		height: Val::Px(size),
		left: Val::Px(left),
		top: Val::Px(top),
		flex_shrink: 0.0,
		..default()
	}
}

fn spawn_shader_booth(
	commands: &mut Commands,
	mesh: Handle<Mesh>,
	material: Handle<FurnitureSurfaceMaterial>,
	target: Handle<Image>,
	layer: RenderLayers,
	order: isize,
	name: &'static str,
) {
	commands.spawn((
		Name::new(format!("title-{name}-mesh")),
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_xyz(0.0, 0.0, 0.0),
		layer.clone(),
	));
	commands.spawn((
		Name::new(format!("title-{name}-camera")),
		Camera3d::default(),
		Camera { order, clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
		RenderTarget::Image(target.into()),
		Projection::Orthographic(ortho_fill()),
		Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
		Msaa::Sample4,
		layer.clone(),
	));
	commands.spawn((
		DirectionalLight {
			illuminance: 12_800.0,
			color: Color::srgb(1.0, 0.88, 0.70),
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
		layer.clone(),
	));
	commands.spawn((
		DirectionalLight {
			illuminance: 4_200.0,
			color: Color::srgb(1.0, 0.72, 0.48),
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 5.0, -PI / 4.0, 0.0)),
		layer,
	));
}

fn ortho_fill() -> OrthographicProjection {
	let mut projection = OrthographicProjection::default_3d();
	projection.scaling_mode = ScalingMode::FixedVertical { viewport_height: 10.0 };
	projection
}

fn render_target(images: &mut Assets<Image>, width: u32, height: u32) -> Handle<Image> {
	let size = Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 };
	let mut image = Image::new_uninit(
		size,
		TextureDimension::D2,
		TextureFormat::Bgra8UnormSrgb,
		RenderAssetUsages::all(),
	);
	image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
		| TextureUsages::COPY_SRC
		| TextureUsages::COPY_DST
		| TextureUsages::RENDER_ATTACHMENT;
	images.add(image)
}

fn sync_clock(config: Res<TitleSequenceConfig>, time: Res<Time>, mut clock: ResMut<SequenceClock>) {
	if config.capture {
		return;
	}
	let duration = config.sequence.duration;
	if duration <= 0.0 {
		clock.time = 0.0;
		return;
	}
	clock.time = time.elapsed_secs().rem_euclid(duration);
}

fn sync_shader_time(clock: Res<SequenceClock>, mut virtual_time: ResMut<Time<Virtual>>) {
	let target = clock.time.max(0.0);
	let current = virtual_time.elapsed_secs();
	let delta = target - current;
	if delta > 1e-5 {
		virtual_time.advance_by(std::time::Duration::from_secs_f32(delta));
	}
}

#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
struct TitlePresent<'w, 's> {
	windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
	cosmos: Query<'w, 's, &'static mut ImageNode, (With<CosmosLayer>, Without<TitleTrail>)>,
	star: Query<
		'w,
		's,
		(&'static mut Node, &'static mut UiTransform, &'static mut Visibility),
		(With<TitleStar>, Without<TitleTrail>),
	>,
	trails: Query<
		'w,
		's,
		(&'static TitleTrail, &'static mut Node, &'static mut ImageNode, &'static mut Visibility),
		(Without<TitleStar>, Without<CosmosLayer>),
	>,
	labels: Query<'w, 's, Entity, With<TitleLabel>>,
	children: Query<'w, 's, &'static Children>,
	texts: Query<'w, 's, &'static mut TextColor>,
}

fn present_title(
	clock: Res<SequenceClock>,
	config: Res<TitleSequenceConfig>,
	mut present: TitlePresent,
) {
	let Ok(window) = present.windows.single() else {
		return;
	};
	let width = window.width().max(1.0);
	let height = window.height().max(1.0);
	let frame = config.sequence.sample(clock.time);

	if let Ok(mut image) = present.cosmos.single_mut() {
		image.color = Color::WHITE.with_alpha(frame.blend);
	}

	if let Ok((mut node, mut transform, mut visibility)) = present.star.single_mut() {
		place_mark(
			&mut node,
			frame.star.x,
			frame.star.y,
			STAR_SIZE_PX * frame.star.scale,
			width,
			height,
		);
		transform.rotation = Rot2::radians(frame.star.rotation);
		*visibility = if frame.star.visible { Visibility::Inherited } else { Visibility::Hidden };
	}

	for (trail, mut node, mut image, mut visibility) in &mut present.trails {
		match frame.star.trail.get(trail.index).copied() {
			Some(bead) => {
				place_mark(&mut node, bead.x, bead.y, STAR_SIZE_PX * bead.scale, width, height);
				image.color = TEXT_YELLOW.with_alpha(bead.alpha);
				*visibility = Visibility::Inherited;
			}
			None => {
				*visibility = Visibility::Hidden;
				image.color = TEXT_YELLOW.with_alpha(0.0);
			}
		}
	}

	let alpha = frame.label.alpha;
	for root in present.labels.iter() {
		set_label_alpha(root, alpha, &present.children, &mut present.texts);
	}
}

fn place_mark(node: &mut Node, x: f32, y: f32, size: f32, width: f32, height: f32) {
	node.width = Val::Px(size);
	node.height = Val::Px(size);
	node.left = Val::Px(x * width - size * 0.5);
	node.top = Val::Px(y * height - size * 0.5);
}

fn set_label_alpha(
	entity: Entity,
	alpha: f32,
	children: &Query<&Children>,
	texts: &mut Query<&mut TextColor>,
) {
	if let Ok(mut color) = texts.get_mut(entity) {
		*color = TextColor(TEXT_YELLOW.with_alpha(alpha));
	}
	let Ok(kids) = children.get(entity) else {
		return;
	};
	for child in kids {
		set_label_alpha(*child, alpha, children, texts);
	}
}
