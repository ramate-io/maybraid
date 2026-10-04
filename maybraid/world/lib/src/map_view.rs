//! Overhead map camera and place-name pins. One `Camera3d`, not a minimap.

use bevy::prelude::*;
use bevy::text::FontSize;
use durham::Durham;
use game_commands::command::TextEntryFocus;
use geneva::{LanguageOverlay, NameKey};
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use menu_components::{NOTO_SANS_REGULAR, TEXT_YELLOW};
use player::CameraFollow;
use player_camera::{
	CameraController, CameraLookSuppressed, CameraPovLocked, FollowCamera, PlayerCameraSystems,
};
use poi_intelligence::{PoiId, PoiRecord, PoiRegistry};
use richmond::Richmond;
use terrain_layer_model::{OnTerrain, TerrainView};
use urbanization_layer_model::Urbanization;
use world_player::{Player as VegetationPlayer, PlaygroundMode};

use crate::control::{InventoryEditCameraFollow, WorldGameplayEnabled};
use crate::player_lifecycle::WorldPlayerRespawnState;
use crate::ui::project_mob_pin;

pub const DEFAULT_MAP_HEIGHT: f32 = 420.0;
const MIN_MAP_HEIGHT: f32 = 80.0;
const MAX_MAP_HEIGHT: f32 = 2_400.0;
const MAP_PIN_LIMIT: usize = 48;
const REGION_LABEL_PX: f32 = 28.0;
const FEATURE_LABEL_PX: f32 = 14.0;
const POI_LABEL_PX: f32 = 7.0;
const PLAYER_MARKER_PX: f32 = 14.0;

/// Overhead view of the current location. Focus moves with spawn-location picks only.
#[derive(Resource, Debug, PartialEq)]
pub struct WorldMapView {
	pub open: bool,
	pub focus: Vec2,
	pub height: f32,
	pub close_locked: bool,
}

impl Default for WorldMapView {
	fn default() -> Self {
		Self { open: false, focus: Vec2::ZERO, height: DEFAULT_MAP_HEIGHT, close_locked: false }
	}
}

impl WorldMapView {
	pub fn open_at(&mut self, focus: Vec2, close_locked: bool) {
		self.open = true;
		self.focus = focus;
		self.height = self.height.clamp(MIN_MAP_HEIGHT, MAX_MAP_HEIGHT);
		if self.height < MIN_MAP_HEIGHT + 1.0 {
			self.height = DEFAULT_MAP_HEIGHT;
		}
		self.close_locked = close_locked;
	}

	pub fn close(&mut self) {
		self.open = false;
		self.close_locked = false;
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum WorldMapSet {
	Toggle,
}

#[derive(Resource, Clone)]
struct MapLabelFont(Handle<Font>);

#[derive(Component)]
struct MapNameHud;

#[derive(Component)]
struct MapNamePin {
	target: MapPinTarget,
}

#[derive(Component)]
struct MapPlayerMarker;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MapPinTarget {
	Name(NameKey),
	Poi(PoiId),
}

#[derive(Bundle)]
struct MapNamePinBundle {
	name: Name,
	pin: MapNamePin,
	node: Node,
	background: BackgroundColor,
	text: Text,
	font: TextFont,
	color: TextColor,
	pickable: Pickable,
	visibility: Visibility,
}

pub struct WorldMapViewPlugin;

impl Plugin for WorldMapViewPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<WorldMapView>()
			.init_resource::<LanguageOverlay>()
			.configure_sets(
				Update,
				WorldMapSet::Toggle
					.after(CharacterControlSystems)
					.before(PlayerCameraSystems::Look),
			)
			.add_systems(Startup, spawn_map_name_hud)
			.add_systems(
				Update,
				(toggle_map_view, sync_map_camera_locks, pan_map_view, stamp_map_camera)
					.chain()
					.in_set(WorldMapSet::Toggle),
			)
			.add_systems(
				Update,
				(sync_map_name_pins, sync_map_player_marker, draw_highlighted_poi)
					.after(PlayerCameraSystems::Apply),
			);
	}
}

fn spawn_map_name_hud(mut commands: Commands, assets: Res<AssetServer>) {
	commands.insert_resource(MapLabelFont(assets.load(NOTO_SANS_REGULAR)));
	commands.spawn((
		Name::new("map-name-hud"),
		MapNameHud,
		Node {
			position_type: PositionType::Absolute,
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			..default()
		},
		Pickable::IGNORE,
		Visibility::Hidden,
	));
}

pub(crate) fn toggle_map_view(
	mode: Res<PlaygroundMode>,
	gameplay: Res<WorldGameplayEnabled>,
	text_focus: Res<TextEntryFocus>,
	mut map: ResMut<WorldMapView>,
	mut intents: MessageReader<CharacterIntent>,
	players: Query<&Transform, With<VegetationPlayer>>,
	followers: Query<&Transform, (With<CameraFollow>, Without<Camera3d>)>,
) {
	if text_focus.0 || *mode != PlaygroundMode::Character {
		return;
	}
	let mut open = false;
	let mut close = false;
	for intent in intents.read() {
		match intent {
			CharacterIntent::ToggleMap if map.open => close = true,
			CharacterIntent::ToggleMap | CharacterIntent::OpenMap => open = true,
			CharacterIntent::CloseMap => close = true,
			_ => {}
		}
	}
	if map.close_locked {
		return;
	}
	if close && map.open {
		map.close();
		return;
	}
	if !open || map.open || !gameplay.0 {
		return;
	}
	let Some(xz) = players
		.iter()
		.next()
		.or_else(|| followers.iter().next())
		.map(|transform| transform.translation.xz())
	else {
		return;
	};
	map.open_at(xz, false);
}

fn sync_map_camera_locks(
	map: Res<WorldMapView>,
	edit: Option<Res<InventoryEditCameraFollow>>,
	mut locked: Option<ResMut<CameraPovLocked>>,
	mut suppressed: Option<ResMut<CameraLookSuppressed>>,
) {
	let hold = map.open || edit.is_some_and(|edit| edit.0);
	if let Some(locked) = locked.as_deref_mut() {
		locked.0 = hold;
	}
	if let Some(suppressed) = suppressed.as_deref_mut() {
		suppressed.0 = map.open;
	}
}

fn pan_map_view(
	time: Res<Time>,
	mut map: ResMut<WorldMapView>,
	mut intents: MessageReader<CharacterIntent>,
) {
	if !map.open {
		return;
	}
	let mut zoom_in = 0.0;
	let mut zoom_out = 0.0;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::Focus(value) => zoom_in += value,
			CharacterIntent::Ads(value) => zoom_out += value,
			_ => {}
		}
	}
	let zoom = (zoom_in - zoom_out).clamp(-1.0, 1.0);
	map.height =
		(map.height * (1.0 - zoom * 0.6 * time.delta_secs())).clamp(MIN_MAP_HEIGHT, MAX_MAP_HEIGHT);
}

fn stamp_map_camera(
	map: Res<WorldMapView>,
	surface: TerrainView<Urbanization<Richmond<OnTerrain<Durham>>>>,
	mut cameras: Query<&mut CameraController, With<FollowCamera>>,
) {
	let ground = surface.height_or_fallback(map.focus);
	for mut controller in &mut cameras {
		if map.open {
			controller.enter_map(map.focus, map.height, ground);
		} else {
			controller.exit_map();
		}
	}
}

#[allow(clippy::too_many_arguments)]
fn sync_map_name_pins(
	map: Res<WorldMapView>,
	overlay: Res<LanguageOverlay>,
	fonts: Res<MapLabelFont>,
	registry: Option<Res<PoiRegistry>>,
	pending: Option<Res<WorldPlayerRespawnState>>,
	camera: Query<(&Camera, &GlobalTransform), (With<Camera3d>, With<FollowCamera>)>,
	surface: TerrainView<Urbanization<Richmond<OnTerrain<Durham>>>>,
	hud: Query<Entity, With<MapNameHud>>,
	mut pins: Query<(
		Entity,
		&MapNamePin,
		&mut Node,
		&mut BackgroundColor,
		&mut Text,
		&mut TextFont,
		&mut Visibility,
	)>,
	mut commands: Commands,
	mut root: Query<&mut Visibility, (With<MapNameHud>, Without<MapNamePin>)>,
) {
	let Ok(mut hud_vis) = root.single_mut() else {
		return;
	};
	if !map.open {
		*hud_vis = Visibility::Hidden;
		for (_, _, _, _, _, _, mut visibility) in &mut pins {
			*visibility = Visibility::Hidden;
		}
		return;
	}
	*hud_vis = Visibility::Inherited;
	let Ok(hud) = hud.single() else {
		return;
	};
	let Ok((camera, camera_transform)) = camera.single() else {
		for (_, _, _, _, _, _, mut visibility) in &mut pins {
			*visibility = Visibility::Hidden;
		}
		return;
	};

	let highlighted = pending.as_deref().and_then(|state| state.pending.as_ref()?.highlighted);
	let wanted = map_pin_targets(&map, &overlay, registry.as_deref(), pending.as_deref());
	let mut assigned = Vec::new();
	for (pin_entity, pin, mut node, mut background, mut text, mut font, mut visibility) in &mut pins
	{
		let Some(target) = wanted.iter().find(|target| target.id == pin.target) else {
			commands.entity(pin_entity).despawn();
			continue;
		};
		let Some((screen, on_screen)) =
			project_mob_pin(camera, camera_transform, pin_world(&surface, target.xz))
		else {
			*visibility = Visibility::Hidden;
			continue;
		};
		place_map_pin(&mut node, screen, target.size);
		background.0 = pin_color(target.id, highlighted, on_screen);
		text.0 = target.label.clone();
		*font = map_label_text_font(&fonts, target.size);
		*visibility = Visibility::Visible;
		assigned.push(target.id);
	}
	for target in wanted {
		if assigned.contains(&target.id) {
			continue;
		}
		let Some((screen, on_screen)) =
			project_mob_pin(camera, camera_transform, pin_world(&surface, target.xz))
		else {
			continue;
		};
		commands.entity(hud).with_children(|root| {
			root.spawn(MapNamePinBundle {
				name: Name::new("map-name-pin"),
				pin: MapNamePin { target: target.id },
				node: map_pin_node(screen, target.size),
				background: BackgroundColor(pin_color(target.id, highlighted, on_screen)),
				text: Text::new(target.label.clone()),
				font: map_label_text_font(&fonts, target.size),
				color: TextColor(Color::WHITE),
				pickable: Pickable::IGNORE,
				visibility: Visibility::Visible,
			});
		});
	}
}

fn map_label_text_font(font: &MapLabelFont, size: f32) -> TextFont {
	TextFont { font: font.0.clone().into(), font_size: FontSize::Px(size), ..default() }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MapLabelKind {
	Region,
	Feature,
	Poi,
}

fn map_label_kind(target: MapPinTarget) -> MapLabelKind {
	match target {
		MapPinTarget::Poi(_) => MapLabelKind::Poi,
		MapPinTarget::Name(NameKey::Region { .. }) => MapLabelKind::Region,
		MapPinTarget::Name(NameKey::Place { .. } | NameKey::ProvisionalPlace { .. }) => {
			MapLabelKind::Poi
		}
		MapPinTarget::Name(_) => MapLabelKind::Feature,
	}
}

fn map_label_size(kind: MapLabelKind) -> f32 {
	match kind {
		MapLabelKind::Region => REGION_LABEL_PX,
		MapLabelKind::Feature => FEATURE_LABEL_PX,
		MapLabelKind::Poi => POI_LABEL_PX,
	}
}

struct MapPinWanted {
	id: MapPinTarget,
	xz: Vec2,
	label: String,
	size: f32,
}

fn map_pin_targets(
	map: &WorldMapView,
	overlay: &LanguageOverlay,
	registry: Option<&PoiRegistry>,
	pending: Option<&WorldPlayerRespawnState>,
) -> Vec<MapPinWanted> {
	let radius = (map.height * 1.5).clamp(120.0, 3_000.0);
	let mut wanted = Vec::new();
	if let Some(pending) = pending.and_then(|state| state.pending.as_ref()) {
		if pending.map_opened {
			if let Some(registry) = registry {
				for id in &pending.candidates {
					let Some(record) = registry.get(*id) else {
						continue;
					};
					wanted.push(MapPinWanted {
						id: MapPinTarget::Poi(*id),
						xz: record.position.xz(),
						label: label_for_poi(record, overlay),
						size: map_label_size(MapLabelKind::Poi),
					});
				}
			}
		}
	}
	let mut names: Vec<_> = overlay
		.names
		.iter()
		.filter(|name| name.xz.distance(map.focus) <= radius)
		.filter(|name| name_visible(name.key, map.height))
		.collect();
	names.sort_by(|a, b| {
		name_rank(a.key)
			.cmp(&name_rank(b.key))
			.then_with(|| a.xz.distance(map.focus).total_cmp(&b.xz.distance(map.focus)))
	});
	for name in names.into_iter().take(MAP_PIN_LIMIT) {
		if wanted.iter().any(|pin| pin.xz.distance(name.xz) < 8.0) {
			continue;
		}
		wanted.push(MapPinWanted {
			id: MapPinTarget::Name(name.key),
			xz: name.xz,
			label: name.surface.clone(),
			size: map_label_size(map_label_kind(MapPinTarget::Name(name.key))),
		});
	}
	wanted.truncate(MAP_PIN_LIMIT);
	wanted
}

fn name_visible(key: NameKey, height: f32) -> bool {
	match map_label_kind(MapPinTarget::Name(key)) {
		MapLabelKind::Region => true,
		MapLabelKind::Feature => height >= 160.0,
		MapLabelKind::Poi => height <= 1_200.0,
	}
}

fn name_rank(key: NameKey) -> u8 {
	match map_label_kind(MapPinTarget::Name(key)) {
		MapLabelKind::Region => 0,
		MapLabelKind::Feature => 1,
		MapLabelKind::Poi => 2,
	}
}

pub(crate) fn label_for_poi(poi: &PoiRecord, overlay: &LanguageOverlay) -> String {
	overlay
		.names
		.iter()
		.filter(|name| {
			matches!(name.key, NameKey::Place { .. } | NameKey::ProvisionalPlace { .. })
				&& name.xz.distance(poi.position.xz()) <= poi.arrival_radius.max(8.0)
		})
		.min_by(|a, b| {
			a.xz.distance(poi.position.xz()).total_cmp(&b.xz.distance(poi.position.xz()))
		})
		.map(|name| name.surface.clone())
		.or_else(|| {
			overlay
				.names
				.iter()
				.filter(|name| name.xz.distance(poi.position.xz()) <= 24.0)
				.min_by(|a, b| {
					a.xz.distance(poi.position.xz()).total_cmp(&b.xz.distance(poi.position.xz()))
				})
				.map(|name| name.surface.clone())
		})
		.unwrap_or_else(|| format!("{:?}", poi.kind))
}

fn pin_world(surface: &TerrainView<Urbanization<Richmond<OnTerrain<Durham>>>>, xz: Vec2) -> Vec3 {
	Vec3::new(xz.x, surface.height_or_fallback(xz) + 2.0, xz.y)
}

fn pin_color(target: MapPinTarget, highlighted: Option<PoiId>, on_screen: bool) -> Color {
	let selected = matches!(target, MapPinTarget::Poi(id) if Some(id) == highlighted);
	let color = if selected {
		Color::srgba(0.95, 0.72, 0.18, 0.92)
	} else if matches!(target, MapPinTarget::Poi(_)) {
		Color::srgba(0.18, 0.42, 0.62, 0.82)
	} else {
		Color::srgba(0.12, 0.16, 0.22, 0.78)
	};
	color.with_alpha(if on_screen { color.alpha() } else { 0.94 })
}

fn pin_width(size: f32) -> f32 {
	(size * 8.5).clamp(72.0, 260.0)
}

fn map_pin_node(screen: Vec2, size: f32) -> Node {
	let width = pin_width(size);
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(screen.x - width * 0.5),
		top: Val::Px(screen.y - size * 0.85),
		width: Val::Px(width),
		padding: UiRect::axes(Val::Px((size * 0.35).max(3.0)), Val::Px((size * 0.18).max(2.0))),
		justify_content: JustifyContent::Center,
		..default()
	}
}

fn place_map_pin(node: &mut Node, screen: Vec2, size: f32) {
	let width = pin_width(size);
	node.left = Val::Px(screen.x - width * 0.5);
	node.top = Val::Px(screen.y - size * 0.85);
	node.width = Val::Px(width);
	node.padding = UiRect::axes(Val::Px((size * 0.35).max(3.0)), Val::Px((size * 0.18).max(2.0)));
}

fn sync_map_player_marker(
	map: Res<WorldMapView>,
	pending: Option<Res<WorldPlayerRespawnState>>,
	players: Query<&Transform, With<VegetationPlayer>>,
	camera: Query<(&Camera, &GlobalTransform), (With<Camera3d>, With<FollowCamera>)>,
	surface: TerrainView<Urbanization<Richmond<OnTerrain<Durham>>>>,
	hud: Query<Entity, With<MapNameHud>>,
	mut markers: Query<(&mut Node, &mut Visibility), With<MapPlayerMarker>>,
	mut commands: Commands,
) {
	if !map.open {
		hide_player_markers(&mut markers);
		return;
	}
	let Some(xz) = player_map_xz(players.iter().next(), death_xz(pending.as_deref())) else {
		hide_player_markers(&mut markers);
		return;
	};
	let Ok((camera, camera_transform)) = camera.single() else {
		hide_player_markers(&mut markers);
		return;
	};
	let Some((screen, _)) = project_mob_pin(camera, camera_transform, pin_world(&surface, xz))
	else {
		hide_player_markers(&mut markers);
		return;
	};
	if let Some((mut node, mut visibility)) = markers.iter_mut().next() {
		place_player_marker(&mut node, screen);
		*visibility = Visibility::Visible;
		return;
	}
	let Ok(hud) = hud.single() else {
		return;
	};
	commands.entity(hud).with_children(|root| {
		root.spawn((
			Name::new("map-player-marker"),
			MapPlayerMarker,
			player_marker_node(screen),
			BackgroundColor(TEXT_YELLOW),
			BorderColor::all(Color::srgba(0.08, 0.10, 0.14, 0.92)),
			Pickable::IGNORE,
			Visibility::Visible,
			GlobalZIndex(i32::MAX - 10),
		));
	});
}

fn hide_player_markers(
	markers: &mut Query<(&mut Node, &mut Visibility), With<MapPlayerMarker>>,
) {
	for (_, mut visibility) in markers.iter_mut() {
		*visibility = Visibility::Hidden;
	}
}

fn death_xz(pending: Option<&WorldPlayerRespawnState>) -> Option<Vec2> {
	pending
		.and_then(|state| state.pending.as_ref())
		.map(|pending| pending.death_at.xz())
}

fn player_map_xz(live: Option<&Transform>, death: Option<Vec2>) -> Option<Vec2> {
	live.map(|transform| transform.translation.xz()).or(death)
}

fn player_marker_node(screen: Vec2) -> Node {
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(screen.x - PLAYER_MARKER_PX * 0.5),
		top: Val::Px(screen.y - PLAYER_MARKER_PX * 0.5),
		width: Val::Px(PLAYER_MARKER_PX),
		height: Val::Px(PLAYER_MARKER_PX),
		border: UiRect::all(Val::Px(2.0)),
		border_radius: BorderRadius::all(Val::Px(PLAYER_MARKER_PX * 0.5)),
		..default()
	}
}

fn place_player_marker(node: &mut Node, screen: Vec2) {
	node.left = Val::Px(screen.x - PLAYER_MARKER_PX * 0.5);
	node.top = Val::Px(screen.y - PLAYER_MARKER_PX * 0.5);
}

fn draw_highlighted_poi(
	map: Res<WorldMapView>,
	registry: Option<Res<PoiRegistry>>,
	pending: Option<Res<WorldPlayerRespawnState>>,
	players: Query<&Transform, With<VegetationPlayer>>,
	surface: TerrainView<Urbanization<Richmond<OnTerrain<Durham>>>>,
	mut gizmos: Gizmos,
) {
	if !map.open {
		return;
	}
	if let Some(xz) = player_map_xz(players.iter().next(), death_xz(pending.as_deref())) {
		let at = pin_world(&surface, xz);
		gizmos.sphere(Isometry3d::from_translation(at), 2.2, TEXT_YELLOW);
	}
	let Some(id) = pending.and_then(|state| state.pending.as_ref()?.highlighted) else {
		return;
	};
	let Some(record) = registry.and_then(|registry| registry.get(id).copied()) else {
		return;
	};
	let radius = record.arrival_radius.max(4.0);
	let mut points = Vec::with_capacity(33);
	for index in 0..=32 {
		let angle = index as f32 / 32.0 * std::f32::consts::TAU;
		points.push(Vec3::new(
			record.position.x + angle.cos() * radius,
			record.position.y + 0.6,
			record.position.z + angle.sin() * radius,
		));
	}
	gizmos.linestrip(points, Color::srgb(0.95, 0.72, 0.18));
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use game_commands::command::TextEntryFocus;
	use player::CameraFollow;

	fn write_toggle(world: &mut World) {
		world.write_message(CharacterIntent::ToggleMap);
	}

	#[test]
	fn opening_the_map_keeps_camera_follow() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(TextEntryFocus(false));
		world.init_resource::<WorldMapView>();
		world.init_resource::<Messages<CharacterIntent>>();
		let player = world
			.spawn((VegetationPlayer, CameraFollow, Transform::from_xyz(12.0, 3.0, -8.0)))
			.id();

		write_toggle(&mut world);
		world
			.run_system_once(toggle_map_view)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(crate::camera::sync_camera_mode)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let map = world.resource::<WorldMapView>();
		assert!(map.open);
		assert_eq!(map.focus, Vec2::new(12.0, -8.0));
		assert!(!map.close_locked);
		assert!(world.get::<CameraFollow>(player).is_some());
		Ok(())
	}

	#[test]
	fn locked_map_ignores_toggle() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldMapView {
			open: true,
			focus: Vec2::ZERO,
			height: DEFAULT_MAP_HEIGHT,
			close_locked: true,
		});
		world.init_resource::<Messages<CharacterIntent>>();
		write_toggle(&mut world);
		world
			.run_system_once(toggle_map_view)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<WorldMapView>().open);
		assert!(world.resource::<WorldMapView>().close_locked);
		Ok(())
	}

	#[test]
	fn stick_intents_do_not_pan_the_map() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.insert_resource(WorldMapView {
			open: true,
			focus: Vec2::new(10.0, 20.0),
			height: DEFAULT_MAP_HEIGHT,
			close_locked: false,
		});
		world.init_resource::<Messages<CharacterIntent>>();
		world.write_message(CharacterIntent::Move(Vec2::X));
		world.write_message(CharacterIntent::Look(Vec2::Y));
		world
			.run_system_once(pan_map_view)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.resource::<WorldMapView>().focus, Vec2::new(10.0, 20.0));
		Ok(())
	}

	#[test]
	fn dpad_down_closes_an_unlocked_map() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldMapView {
			open: true,
			focus: Vec2::ZERO,
			height: DEFAULT_MAP_HEIGHT,
			close_locked: false,
		});
		world.init_resource::<Messages<CharacterIntent>>();
		world.write_message(CharacterIntent::CloseMap);
		world
			.run_system_once(toggle_map_view)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<WorldMapView>().open);
		Ok(())
	}

	#[test]
	fn label_sizes_halve_by_layer() {
		assert_eq!(
			map_label_size(MapLabelKind::Feature),
			map_label_size(MapLabelKind::Region) * 0.5
		);
		assert_eq!(map_label_size(MapLabelKind::Poi), map_label_size(MapLabelKind::Feature) * 0.5);
		assert_eq!(
			map_label_kind(MapPinTarget::Name(NameKey::Region { ix: 0, iz: 0 })),
			MapLabelKind::Region
		);
	}

	#[test]
	fn player_marker_prefers_the_live_body() {
		let live = Transform::from_xyz(4.0, 1.0, -3.0);
		assert_eq!(
			player_map_xz(Some(&live), Some(Vec2::new(90.0, 80.0))),
			Some(Vec2::new(4.0, -3.0))
		);
		assert_eq!(player_map_xz(None, Some(Vec2::new(90.0, 80.0))), Some(Vec2::new(90.0, 80.0)));
		assert_eq!(player_map_xz(None, None), None);
	}
}
