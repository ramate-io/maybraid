//! Corner viewports, map cameras, and the debraid overlay.

use bevy::asset::RenderAssetUsages;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, TextureFormat, TextureUsages};
use bevy::text::FontSize;
use bevy::ui::widget::ViewportNode;

use crate::cursor::SkillMapCursor;
use crate::map::{authored_map_from_spec, render_layer, AuthoredMap, SkillMapId};
use crate::tile_material::SkillMapTileAssets;
use crate::tiles::spawn_map_tiles;
use crate::user::{SkillMapEquip, SkillMapHeld, SkillMapMember, SkillMapSession, SkillMapUser};
use crate::SkillMapEnabled;

pub(crate) const VIEWPORT_PX: f32 = 228.0;
/// `WindowSize` area is viewport pixels times this. 1.0 showed almost the whole 256 map.
pub(crate) const MAP_CAMERA_SCALE: f32 = 0.4;

pub(crate) fn map_view_extent() -> Vec2 {
	Vec2::splat(VIEWPORT_PX * MAP_CAMERA_SCALE)
}
const VIEWPORT_GAP: f32 = 12.0;
const VIEWPORT_INSET: f32 = 16.0;
const LIVE_BORDER: Color = Color::srgb(1.0, 0.48, 0.08);
const IDLE_BORDER: Color = Color::srgba(1.0, 0.86, 0.22, 0.42);
const LABEL_YELLOW: Color = Color::srgb(1.0, 0.86, 0.22);

#[derive(Component)]
pub struct SkillMapViewport;

#[derive(Component)]
pub struct SkillMapViewportCamera;

#[derive(Component)]
pub struct SkillMapLabel;

#[derive(Component)]
pub struct Debraid {
	pub remaining: f32,
}

#[derive(Component)]
pub(crate) struct DebraidOverlay;

/// Present the equipped map. Rebuild when [`SkillMapEquip`] changes.
pub fn present_skill_maps(
	mut commands: Commands,
	mut images: ResMut<Assets<Image>>,
	tiles: Res<SkillMapTileAssets>,
	users: Query<(Entity, &SkillMapUser, &SkillMapEquip)>,
	mut sessions: Query<&mut SkillMapSession>,
	members: Query<(Entity, &SkillMapMember)>,
) {
	for (user, mapping, equip) in &users {
		let Ok(mut session) = sessions.get_mut(mapping.maps) else {
			continue;
		};
		if session.presented == equip.spec && (equip.spec.is_none() || !session.cameras.is_empty())
		{
			continue;
		}
		clear_presented(&mut commands, mapping.maps, &mut session, &members);
		session.presented = equip.spec;
		let Some(spec) = equip.spec else {
			continue;
		};
		spawn_one_map(
			&mut commands,
			&mut images,
			&tiles,
			user,
			mapping.maps,
			&mut session,
			authored_map_from_spec(spec),
			0,
		);
	}
}

fn clear_presented(
	commands: &mut Commands,
	session: Entity,
	viewports: &mut SkillMapSession,
	members: &Query<(Entity, &SkillMapMember)>,
) {
	for (entity, member) in members {
		if member.session == session {
			commands.entity(entity).try_despawn();
		}
	}
	viewports.cameras.clear();
	viewports.nodes.clear();
	viewports.presented = None;
}

fn spawn_one_map(
	commands: &mut Commands,
	images: &mut Assets<Image>,
	tiles: &SkillMapTileAssets,
	user: Entity,
	session: Entity,
	viewports: &mut SkillMapSession,
	spec: AuthoredMap,
	stack_index: usize,
) {
	let layer = render_layer(spec.id);
	let member = SkillMapMember { user, session };
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
	projection.scale = MAP_CAMERA_SCALE;
	let camera = commands
		.spawn((
			Name::new(format!("skill-map-camera-{}", spec.label)),
			Camera2d,
			Camera {
				order: -2 - stack_index as isize,
				clear_color: ClearColorConfig::Custom(spec.kind.viewport_clear()),
				..default()
			},
			RenderTarget::Image(image_handle.into()),
			Projection::Orthographic(projection),
			Transform::from_xyz(0.0, 0.0, 1.0),
			SkillMapViewportCamera,
			spec.id,
			member,
			layer.clone(),
		))
		.id();

	let bottom = VIEWPORT_INSET + stack_index as f32 * (VIEWPORT_PX + VIEWPORT_GAP);
	let node = commands
		.spawn((
			Name::new(format!("skill-map-viewport-{}", spec.label)),
			SkillMapViewport,
			spec.id,
			member,
			Node {
				position_type: PositionType::Absolute,
				bottom: Val::Px(bottom),
				right: Val::Px(VIEWPORT_INSET),
				width: Val::Px(VIEWPORT_PX),
				height: Val::Px(VIEWPORT_PX),
				border: UiRect::all(Val::Px(2.0)),
				padding: UiRect::all(Val::Px(8.0)),
				border_radius: BorderRadius::all(Val::Px(14.0)),
				flex_direction: FlexDirection::Column,
				justify_content: JustifyContent::FlexStart,
				align_items: AlignItems::FlexStart,
				..default()
			},
			BorderColor::all(IDLE_BORDER),
			BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.12)),
			ViewportNode::new(camera),
			Visibility::Hidden,
			Pickable::IGNORE,
		))
		.with_children(|parent| {
			parent.spawn((
				SkillMapLabel,
				Text::new(format!("{} {:04X}", spec.label, spec.seed as u16)),
				TextFont { font_size: FontSize::Px(16.0), ..default() },
				TextColor(LABEL_YELLOW),
				Pickable::IGNORE,
			));
		})
		.id();

	commands.spawn((
		Name::new(format!("skill-map-cursor-{}", spec.label)),
		SkillMapCursor,
		spec.id,
		member,
		Mesh2d(tiles.cursor_mesh.clone()),
		MeshMaterial2d(tiles.cursor.clone()),
		Transform::from_xyz(0.0, 0.0, 1.0),
		layer,
	));

	spawn_map_tiles(commands, spec, member, tiles);
	viewports.cameras.insert(spec.id, camera);
	viewports.nodes.insert(spec.id, node);
}

pub fn sync_viewport_chrome(
	enabled: Res<SkillMapEnabled>,
	users: Query<(&SkillMapUser, &SkillMapHeld)>,
	mut nodes: Query<
		(&SkillMapMember, &mut Visibility, &mut BorderColor, &mut BackgroundColor),
		With<SkillMapViewport>,
	>,
) {
	for (member, mut visibility, mut border, mut background) in &mut nodes {
		if !enabled.0 {
			*visibility = Visibility::Hidden;
			continue;
		}
		*visibility = Visibility::Inherited;
		let held = users.iter().any(|(user, held)| user.maps == member.session && held.0);
		if held {
			*border = BorderColor::all(LIVE_BORDER);
			background.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
		} else {
			*border = BorderColor::all(IDLE_BORDER);
			background.0 = Color::srgba(0.06, 0.05, 0.04, 0.45);
		}
	}
}

type TrackedCameras<'w, 's> = Query<
	'w,
	's,
	(&'static SkillMapId, &'static SkillMapMember, &'static Transform),
	(With<SkillMapViewportCamera>, Without<SkillMapCursor>, Changed<Transform>),
>;

type TrackedCursors<'w, 's> = Query<
	'w,
	's,
	(&'static SkillMapId, &'static SkillMapMember, &'static mut Transform),
	(With<SkillMapCursor>, Without<SkillMapViewportCamera>),
>;

pub fn track_cursors(cameras: TrackedCameras, mut cursors: TrackedCursors) {
	for (map, camera_member, camera) in &cameras {
		for (cursor_map, cursor_member, mut cursor) in &mut cursors {
			if cursor_map.0 == map.0 && cursor_member.session == camera_member.session {
				cursor.translation.x = camera.translation.x;
				cursor.translation.y = camera.translation.y;
			}
		}
	}
}

pub fn spawn_debraid(
	commands: &mut Commands,
	session: &SkillMapSession,
	id: SkillMapId,
	secs: f32,
) {
	let Some(node) = session.nodes.get(&id).copied() else {
		return;
	};
	commands.entity(node).with_children(|parent| {
		parent
			.spawn((
				DebraidOverlay,
				Debraid { remaining: secs },
				id,
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(0.0),
					right: Val::Px(0.0),
					top: Val::Px(0.0),
					bottom: Val::Px(0.0),
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
					..default()
				},
				BackgroundColor(Color::srgba(0.85, 0.08, 0.06, 0.55)),
				Pickable::IGNORE,
			))
			.with_children(|overlay| {
				overlay.spawn((
					Text::new("DEBRAID"),
					TextFont { font_size: FontSize::Px(22.0), ..default() },
					TextColor(Color::WHITE),
					Pickable::IGNORE,
				));
			});
	});
}

pub fn tick_debraid(
	time: Res<Time>,
	mut commands: Commands,
	mut overlays: Query<(Entity, &mut Debraid), With<DebraidOverlay>>,
) {
	let dt = time.delta_secs();
	for (entity, mut debraid) in &mut overlays {
		debraid.remaining -= dt;
		if debraid.remaining <= 0.0 {
			commands.entity(entity).despawn();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::SkillMapEnabled;

	#[test]
	fn chrome_and_track_queries_are_disjoint() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<SkillMapEnabled>()
			.add_systems(Update, (sync_viewport_chrome, track_cursors, tick_debraid));
		app.update();
	}
}
