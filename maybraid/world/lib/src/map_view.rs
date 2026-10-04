//! Overhead map camera and place-name pins. One `Camera3d`, not a minimap.

use bevy::prelude::*;
use bevy::text::FontSize;
use durham::Durham;
use game_commands::command::TextEntryFocus;
use geneva::{LanguageOverlay, NameKey, NamedOverlay};
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use menu_components::{
	BARLOW_BLACK, ITEM_FONT_SIZE, NOTO_SANS_REGULAR, TEXT_YELLOW, TEXT_YELLOW_FAINT,
};
use player::CameraFollow;
use player_camera::{
	CameraController, CameraLookSuppressed, CameraPovLocked, FollowCamera, PlayerCameraSystems,
};
use poi_intelligence::{PoiId, PoiRecord, PoiRegistry};
use richmond::Richmond;
use terrain_layer_model::{OnTerrain, TerrainView};
use urbanization_layer_model::Urbanization;
use world_player::{Player as VegetationPlayer, PlayerLifeSet, PlaygroundMode};

use crate::control::{InventoryEditCameraFollow, WorldGameplayEnabled};
use crate::player_lifecycle::WorldPlayerRespawnState;
use crate::ui::project_mob_pin;

pub const DEFAULT_MAP_HEIGHT: f32 = 420.0;
const MIN_MAP_HEIGHT: f32 = 80.0;
const MAX_MAP_HEIGHT: f32 = 2_400.0;
const MAP_PIN_LIMIT: usize = 48;
const MAX_REGION_LABELS: usize = 2;
const MAX_FEATURE_LABELS: usize = 6;
const MAX_POI_LABELS: usize = 10;
const REGION_LABEL_PX: f32 = 28.0;
const FEATURE_LABEL_PX: f32 = 14.0;
const POI_LABEL_PX: f32 = 7.0;
const PLAYER_MARKER_PX: f32 = 14.0;
const SELECTED_POI_LABEL_PX: f32 = 16.0;
const SELECTION_RING_PX: f32 = 46.0;
const SELECTION_DOT_PX: f32 = 10.0;
const PICKER_TITLE: &str = "Pick Respawn Point";

/// Overhead view of the current location. Focus moves with spawn-location picks only.
#[derive(Resource, Debug, PartialEq)]
pub struct WorldMapView {
	pub open: bool,
	pub focus: Vec2,
	pub height: f32,
	pub close_locked: bool,
	/// Death picker / new body: stamp third person instead of restoring the last POV.
	pub begin_life: bool,
}

impl Default for WorldMapView {
	fn default() -> Self {
		Self {
			open: false,
			focus: Vec2::ZERO,
			height: DEFAULT_MAP_HEIGHT,
			close_locked: false,
			begin_life: false,
		}
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
		if self.close_locked {
			self.begin_life = true;
		}
		self.open = false;
		self.close_locked = false;
	}

	pub fn request_begin_life(&mut self) {
		self.begin_life = true;
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
struct RespawnPickerTitle;

#[derive(Component)]
struct MapRespawnSelection;

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
	text: Text,
	font: TextFont,
	color: TextColor,
	shadow: TextShadow,
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
				(toggle_map_view, sync_map_camera_locks, pan_map_view)
					.chain()
					.in_set(WorldMapSet::Toggle),
			)
			.add_systems(
				Update,
				stamp_map_camera
					.after(WorldMapSet::Toggle)
					.after(PlayerLifeSet::Resolve)
					.before(PlayerCameraSystems::Look),
			)
			.add_systems(
				Update,
				(
					sync_map_name_pins,
					sync_map_player_marker,
					sync_respawn_picker_title,
					sync_respawn_selection_marker,
					draw_highlighted_poi,
				)
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
	commands.spawn((
		Name::new("respawn-picker-title"),
		RespawnPickerTitle,
		Node {
			position_type: PositionType::Absolute,
			top: Val::Px(28.0),
			width: Val::Percent(100.0),
			justify_content: JustifyContent::Center,
			..default()
		},
		Text::new(PICKER_TITLE),
		TextFont {
			font: assets.load(BARLOW_BLACK).into(),
			font_size: FontSize::Px(ITEM_FONT_SIZE),
			..default()
		},
		TextColor(TEXT_YELLOW),
		TextShadow { offset: Vec2::new(1.5, 1.5), color: Color::srgba(0.06, 0.05, 0.04, 0.72) },
		TextLayout::new(Justify::Center, bevy::text::LineBreak::NoWrap),
		Pickable::IGNORE,
		Visibility::Hidden,
		GlobalZIndex(i32::MAX - 2),
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
	mut map: ResMut<WorldMapView>,
	surface: TerrainView<Urbanization<Richmond<OnTerrain<Durham>>>>,
	mut cameras: Query<&mut CameraController, With<FollowCamera>>,
) {
	let ground = surface.height_or_fallback(map.focus);
	let begin_life = !map.open && map.begin_life;
	if begin_life {
		map.begin_life = false;
	}
	for mut controller in &mut cameras {
		if map.open {
			controller.enter_map(map.focus, map.height, ground);
		} else if begin_life {
			controller.begin_life();
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
		&mut Text,
		&mut TextFont,
		&mut TextColor,
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
	for (pin_entity, pin, mut node, mut text, mut font, mut color, mut visibility) in &mut pins {
		let Some(target) = wanted.iter().find(|target| target.id == pin.target) else {
			commands.entity(pin_entity).despawn();
			continue;
		};
		let Some((screen, _)) =
			project_mob_pin(camera, camera_transform, pin_world(&surface, target.xz))
		else {
			*visibility = Visibility::Hidden;
			continue;
		};
		place_map_pin(&mut node, screen, target.size, &target.label);
		text.0 = target.label.clone();
		*font = map_label_text_font(&fonts, target.size);
		color.0 = label_ink(target.id, highlighted);
		*visibility = Visibility::Visible;
		assigned.push(target.id);
	}
	for target in wanted {
		if assigned.contains(&target.id) {
			continue;
		}
		let Some((screen, _)) =
			project_mob_pin(camera, camera_transform, pin_world(&surface, target.xz))
		else {
			continue;
		};
		commands.entity(hud).with_children(|root| {
			root.spawn(MapNamePinBundle {
				name: Name::new("map-name-pin"),
				pin: MapNamePin { target: target.id },
				node: map_pin_node(screen, target.size, &target.label),
				text: Text::new(target.label.clone()),
				font: map_label_text_font(&fonts, target.size),
				color: TextColor(label_ink(target.id, highlighted)),
				shadow: map_label_shadow(),
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
	extent: Rect,
	label: String,
	size: f32,
}

fn map_pin_targets(
	map: &WorldMapView,
	overlay: &LanguageOverlay,
	registry: Option<&PoiRegistry>,
	pending: Option<&WorldPlayerRespawnState>,
) -> Vec<MapPinWanted> {
	let view = map_view_rect(map);
	let mut wanted = Vec::new();
	let highlighted = pending.and_then(|state| state.pending.as_ref()?.highlighted);
	if let Some(pending) = pending.and_then(|state| state.pending.as_ref()) {
		if pending.map_opened {
			if let Some(registry) = registry {
				for id in &pending.candidates {
					let Some(record) = registry.get(*id) else {
						continue;
					};
					let xz = record.position.xz();
					let selected = Some(*id) == highlighted;
					wanted.push(MapPinWanted {
						id: MapPinTarget::Poi(*id),
						xz,
						extent: Rect::from_center_size(xz, Vec2::splat(12.0)),
						label: label_for_poi(record, overlay),
						size: if selected {
							SELECTED_POI_LABEL_PX
						} else {
							map_label_size(MapLabelKind::Poi)
						},
					});
				}
			}
		}
	}
	let mut names: Vec<_> = overlay
		.names
		.iter()
		.filter(|name| name_visible(name.key, map.height))
		.filter_map(|name| {
			let kind = map_label_kind(MapPinTarget::Name(name.key));
			let xz = label_anchor(name, view, kind)?;
			Some((name, kind, xz))
		})
		.collect();
	names.sort_by(|(a, _, a_xz), (b, _, b_xz)| {
		name_rank(a.key)
			.cmp(&name_rank(b.key))
			.then_with(|| a_xz.distance(map.focus).total_cmp(&b_xz.distance(map.focus)))
	});
	let mut regions = 0;
	let mut features = 0;
	let mut pois = 0;
	for (name, kind, xz) in names {
		let at_cap = match kind {
			MapLabelKind::Region => regions >= MAX_REGION_LABELS,
			MapLabelKind::Feature => features >= MAX_FEATURE_LABELS,
			MapLabelKind::Poi => pois >= MAX_POI_LABELS,
		};
		if at_cap {
			continue;
		}
		if matches!(kind, MapLabelKind::Poi) && wanted.iter().any(|pin| pin.xz.distance(xz) < 8.0) {
			continue;
		}
		match kind {
			MapLabelKind::Region => regions += 1,
			MapLabelKind::Feature => features += 1,
			MapLabelKind::Poi => pois += 1,
		}
		wanted.push(MapPinWanted {
			id: MapPinTarget::Name(name.key),
			xz,
			extent: name.extent,
			label: bilingual(&name.surface, &name.english),
			size: map_label_size(kind),
		});
	}
	wanted.truncate(MAP_PIN_LIMIT);
	resolve_label_collisions(wanted, view, highlighted.map(MapPinTarget::Poi))
}

fn map_view_rect(map: &WorldMapView) -> Rect {
	let half = (map.height * 1.05).clamp(90.0, 2_200.0);
	Rect::from_center_size(map.focus, Vec2::splat(half * 2.0))
}

fn label_anchor(name: &NamedOverlay, view: Rect, kind: MapLabelKind) -> Option<Vec2> {
	match kind {
		MapLabelKind::Poi => view.contains(name.xz).then_some(name.xz),
		MapLabelKind::Region => {
			let hit = view.intersect(name.extent);
			if rect_empty(hit) {
				return None;
			}
			Some(title_band(hit))
		}
		MapLabelKind::Feature => {
			let hit = view.intersect(name.extent);
			if rect_empty(hit) {
				return None;
			}
			if swallows_view(hit, view) {
				return None;
			}
			if hit.contains(name.xz) {
				Some(name.xz)
			} else {
				Some(clamp_into_rect(name.xz, inset_rect(hit, 0.12)))
			}
		}
	}
}

fn title_band(hit: Rect) -> Vec2 {
	let pad_y = (hit.height() * 0.16).max(8.0);
	Vec2::new(hit.center().x, (hit.max.y - pad_y).clamp(hit.min.y + 4.0, hit.max.y - 4.0))
}

fn swallows_view(hit: Rect, view: Rect) -> bool {
	hit.width() * hit.height() >= view.width() * view.height() * 0.45
}

fn inset_rect(rect: Rect, fraction: f32) -> Rect {
	let pad = rect.size() * fraction.clamp(0.0, 0.4);
	let min = rect.min + pad;
	let max = rect.max - pad;
	if min.x < max.x && min.y < max.y {
		Rect { min, max }
	} else {
		rect
	}
}

fn rect_empty(rect: Rect) -> bool {
	rect.width() <= 0.0 || rect.height() <= 0.0
}

fn clamp_into_rect(point: Vec2, rect: Rect) -> Vec2 {
	Vec2::new(point.x.clamp(rect.min.x, rect.max.x), point.y.clamp(rect.min.y, rect.max.y))
}

fn resolve_label_collisions(
	mut wanted: Vec<MapPinWanted>,
	view: Rect,
	keep: Option<MapPinTarget>,
) -> Vec<MapPinWanted> {
	if let Some(keep) = keep {
		wanted.sort_by_key(|pin| if pin.id == keep { 0u8 } else { 1 });
	}
	let mut kept = Vec::with_capacity(wanted.len());
	for pin in wanted {
		let keep_pin = keep == Some(pin.id);
		let sep = label_separation(&pin, view);
		if keep_pin
			|| kept.iter().all(|other: &MapPinWanted| {
				pin.xz.distance(other.xz) >= sep.max(label_separation(other, view))
			}) {
			kept.push(pin);
			continue;
		}
		if let Some(xz) = nudge_label(&pin, &kept, view, sep) {
			kept.push(MapPinWanted { xz, ..pin });
		}
	}
	kept
}

fn label_separation(pin: &MapPinWanted, view: Rect) -> f32 {
	let side = view.width().min(view.height());
	match map_label_kind(pin.id) {
		MapLabelKind::Region => (side * 0.28).max(48.0),
		MapLabelKind::Feature => (side * 0.16).max(28.0),
		MapLabelKind::Poi => (side * 0.12).max(20.0),
	}
}

fn nudge_label(pin: &MapPinWanted, kept: &[MapPinWanted], view: Rect, sep: f32) -> Option<Vec2> {
	let room = allowed_label_rect(pin, view, sep);
	if rect_empty(room) {
		return None;
	}
	let dirs = [
		Vec2::X,
		Vec2::NEG_X,
		Vec2::Y,
		Vec2::NEG_Y,
		Vec2::new(1.0, 1.0),
		Vec2::new(-1.0, 1.0),
		Vec2::new(1.0, -1.0),
		Vec2::new(-1.0, -1.0),
	];
	for dir in dirs {
		let candidate = clamp_into_rect(pin.xz + dir.normalize() * sep, room);
		if kept.iter().all(|other: &MapPinWanted| {
			candidate.distance(other.xz) >= sep.max(label_separation(other, view))
		}) {
			return Some(candidate);
		}
	}
	None
}

fn allowed_label_rect(pin: &MapPinWanted, view: Rect, sep: f32) -> Rect {
	match map_label_kind(pin.id) {
		MapLabelKind::Poi => {
			view.intersect(Rect::from_center_size(pin.xz, Vec2::splat((sep * 2.0).max(24.0))))
		}
		MapLabelKind::Region | MapLabelKind::Feature => {
			let hit = view.intersect(pin.extent);
			if rect_empty(hit) {
				view
			} else {
				hit
			}
		}
	}
}

fn bilingual(surface: &str, english: &[String]) -> String {
	let gloss: Vec<_> =
		english.iter().map(|word| word.trim()).filter(|word| !word.is_empty()).collect();
	if gloss.is_empty() {
		surface.to_string()
	} else {
		format!("{surface} ({})", gloss.join(" "))
	}
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
		.or_else(|| {
			overlay
				.names
				.iter()
				.filter(|name| name.xz.distance(poi.position.xz()) <= 24.0)
				.min_by(|a, b| {
					a.xz.distance(poi.position.xz()).total_cmp(&b.xz.distance(poi.position.xz()))
				})
		})
		.map(|name| bilingual(&name.surface, &name.english))
		.unwrap_or_else(|| format!("{:?}", poi.kind))
}

fn pin_world(surface: &TerrainView<Urbanization<Richmond<OnTerrain<Durham>>>>, xz: Vec2) -> Vec3 {
	Vec3::new(xz.x, surface.height_or_fallback(xz) + 2.0, xz.y)
}

fn label_ink(target: MapPinTarget, highlighted: Option<PoiId>) -> Color {
	if matches!(target, MapPinTarget::Poi(id) if Some(id) == highlighted) {
		return TEXT_YELLOW;
	}
	match map_label_kind(target) {
		MapLabelKind::Region => Color::srgba(0.10, 0.08, 0.06, 0.94),
		MapLabelKind::Feature => Color::srgba(0.14, 0.12, 0.09, 0.90),
		MapLabelKind::Poi => Color::srgba(0.16, 0.14, 0.11, 0.86),
	}
}

fn map_label_shadow() -> TextShadow {
	TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.98, 0.94, 0.86, 0.82) }
}

fn pin_width(size: f32, label: &str) -> f32 {
	(size * 0.58 * label.chars().count() as f32 + 12.0).clamp(72.0, 520.0)
}

fn map_pin_node(screen: Vec2, size: f32, label: &str) -> Node {
	let width = pin_width(size, label);
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(screen.x - width * 0.5),
		top: Val::Px(screen.y - size * 0.85),
		width: Val::Px(width),
		justify_content: JustifyContent::Center,
		..default()
	}
}

fn place_map_pin(node: &mut Node, screen: Vec2, size: f32, label: &str) {
	let width = pin_width(size, label);
	node.left = Val::Px(screen.x - width * 0.5);
	node.top = Val::Px(screen.y - size * 0.85);
	node.width = Val::Px(width);
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

fn hide_player_markers(markers: &mut Query<(&mut Node, &mut Visibility), With<MapPlayerMarker>>) {
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

fn picker_prompt_visible(map: &WorldMapView) -> bool {
	map.open && map.close_locked
}

fn sync_respawn_picker_title(
	map: Res<WorldMapView>,
	mut titles: Query<&mut Visibility, With<RespawnPickerTitle>>,
) {
	let visible = picker_prompt_visible(&map);
	for mut visibility in &mut titles {
		*visibility = if visible { Visibility::Visible } else { Visibility::Hidden };
	}
}

fn selected_poi_xz(
	pending: Option<&WorldPlayerRespawnState>,
	registry: Option<&PoiRegistry>,
) -> Option<Vec2> {
	let id = pending?.pending.as_ref()?.highlighted?;
	Some(registry?.get(id)?.position.xz())
}

fn sync_respawn_selection_marker(
	map: Res<WorldMapView>,
	registry: Option<Res<PoiRegistry>>,
	pending: Option<Res<WorldPlayerRespawnState>>,
	camera: Query<(&Camera, &GlobalTransform), (With<Camera3d>, With<FollowCamera>)>,
	surface: TerrainView<Urbanization<Richmond<OnTerrain<Durham>>>>,
	mut markers: Query<(&mut Node, &mut Visibility), With<MapRespawnSelection>>,
	mut commands: Commands,
) {
	let Some(xz) = selected_poi_xz(pending.as_deref(), registry.as_deref())
		.filter(|_| picker_prompt_visible(&map))
	else {
		hide_selection_markers(&mut markers);
		return;
	};
	let Ok((camera, camera_transform)) = camera.single() else {
		hide_selection_markers(&mut markers);
		return;
	};
	let Some((screen, _)) = project_mob_pin(camera, camera_transform, pin_world(&surface, xz))
	else {
		hide_selection_markers(&mut markers);
		return;
	};
	if let Some((mut node, mut visibility)) = markers.iter_mut().next() {
		place_selection_marker(&mut node, screen);
		*visibility = Visibility::Visible;
		return;
	}
	let marker = commands
		.spawn((
			Name::new("map-respawn-selection"),
			MapRespawnSelection,
			selection_marker_node(screen),
			BackgroundColor(Color::srgba(1.0, 0.86, 0.22, 0.12)),
			BorderColor::all(TEXT_YELLOW),
			Pickable::IGNORE,
			Visibility::Visible,
			GlobalZIndex(i32::MAX - 8),
		))
		.id();
	commands.entity(marker).with_children(|root| {
		root.spawn((
			Node {
				width: Val::Px(SELECTION_DOT_PX),
				height: Val::Px(SELECTION_DOT_PX),
				border_radius: BorderRadius::all(Val::Px(SELECTION_DOT_PX * 0.5)),
				..default()
			},
			BackgroundColor(TEXT_YELLOW),
			Pickable::IGNORE,
		));
	});
}

fn hide_selection_markers(
	markers: &mut Query<(&mut Node, &mut Visibility), With<MapRespawnSelection>>,
) {
	for (_, mut visibility) in markers.iter_mut() {
		*visibility = Visibility::Hidden;
	}
}

fn selection_marker_node(screen: Vec2) -> Node {
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(screen.x - SELECTION_RING_PX * 0.5),
		top: Val::Px(screen.y - SELECTION_RING_PX * 0.5),
		width: Val::Px(SELECTION_RING_PX),
		height: Val::Px(SELECTION_RING_PX),
		border: UiRect::all(Val::Px(3.0)),
		border_radius: BorderRadius::all(Val::Px(SELECTION_RING_PX * 0.5)),
		justify_content: JustifyContent::Center,
		align_items: AlignItems::Center,
		..default()
	}
}

fn place_selection_marker(node: &mut Node, screen: Vec2) {
	node.left = Val::Px(screen.x - SELECTION_RING_PX * 0.5);
	node.top = Val::Px(screen.y - SELECTION_RING_PX * 0.5);
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
	let Some(id) = pending.as_deref().and_then(|state| state.pending.as_ref()?.highlighted) else {
		return;
	};
	let Some(record) = registry.and_then(|registry| registry.get(id).copied()) else {
		return;
	};
	let radius = (map.height * 0.045).max(record.arrival_radius).clamp(12.0, 48.0);
	let mut points = Vec::with_capacity(33);
	for index in 0..=32 {
		let angle = index as f32 / 32.0 * std::f32::consts::TAU;
		points.push(Vec3::new(
			record.position.x + angle.cos() * radius,
			record.position.y + 0.8,
			record.position.z + angle.sin() * radius,
		));
	}
	gizmos.linestrip(points, TEXT_YELLOW);
	if let Some(death) = death_xz(pending.as_deref()) {
		gizmos.line(
			pin_world(&surface, death),
			pin_world(&surface, record.position.xz()),
			TEXT_YELLOW_FAINT,
		);
	}
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
			begin_life: false,
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
			begin_life: false,
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
			begin_life: false,
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
	fn bilingual_label_appends_english() {
		assert_eq!(bilingual("ʃin", &["ridge".into()]), "ʃin (ridge)");
		assert_eq!(bilingual("ʃin", &[String::new()]), "ʃin");
	}

	#[test]
	fn region_label_sits_in_the_view_not_at_the_centroid() {
		let map = WorldMapView {
			open: true,
			focus: Vec2::new(100.0, 80.0),
			height: DEFAULT_MAP_HEIGHT,
			close_locked: false,
			begin_life: false,
		};
		let overlay = LanguageOverlay {
			names: vec![NamedOverlay {
				key: NameKey::Region { ix: 0, iz: 0 },
				surface: "ʃin".into(),
				english: vec!["ridge".into()],
				provisional: true,
				xz: Vec2::splat(12_500.0),
				extent: Rect::from_corners(Vec2::ZERO, Vec2::splat(25_000.0)),
			}],
			..Default::default()
		};
		let wanted = map_pin_targets(&map, &overlay, None, None);
		assert_eq!(wanted.len(), 1);
		let view = map_view_rect(&map);
		assert!(view.contains(wanted[0].xz));
		assert!((wanted[0].xz.x - view.center().x).abs() < view.width() * 0.2);
		assert!(wanted[0].xz.y > view.center().y);
		assert_eq!(wanted[0].label, "ʃin (ridge)");
	}

	#[test]
	fn overlapping_region_titles_do_not_stack() {
		let map = WorldMapView {
			open: true,
			focus: Vec2::ZERO,
			height: DEFAULT_MAP_HEIGHT,
			close_locked: false,
			begin_life: false,
		};
		let overlay = LanguageOverlay {
			names: vec![
				NamedOverlay {
					key: NameKey::Region { ix: 0, iz: 0 },
					surface: "east".into(),
					english: Vec::new(),
					provisional: true,
					xz: Vec2::new(5_000.0, 0.0),
					extent: Rect::from_corners(
						Vec2::new(0.0, -10_000.0),
						Vec2::new(10_000.0, 10_000.0),
					),
				},
				NamedOverlay {
					key: NameKey::Region { ix: -1, iz: 0 },
					surface: "west".into(),
					english: Vec::new(),
					provisional: true,
					xz: Vec2::new(-5_000.0, 0.0),
					extent: Rect::from_corners(
						Vec2::new(-10_000.0, -10_000.0),
						Vec2::new(0.0, 10_000.0),
					),
				},
			],
			..Default::default()
		};
		let wanted = map_pin_targets(&map, &overlay, None, None);
		assert_eq!(wanted.len(), 2);
		assert!(wanted[0].xz.distance(wanted[1].xz) > 40.0);
	}

	#[test]
	fn closing_a_locked_map_requests_a_new_life_camera() {
		let mut map = WorldMapView { open: true, close_locked: true, ..default() };
		map.close();
		assert!(!map.open);
		assert!(!map.close_locked);
		assert!(map.begin_life);
	}

	#[test]
	fn selected_respawn_labels_are_larger_than_idle_pois() {
		assert!(SELECTED_POI_LABEL_PX > map_label_size(MapLabelKind::Poi));
	}

	#[test]
	fn picker_title_only_shows_on_a_locked_map() {
		assert!(picker_prompt_visible(&WorldMapView {
			open: true,
			close_locked: true,
			..default()
		}));
		assert!(!picker_prompt_visible(&WorldMapView {
			open: true,
			close_locked: false,
			..default()
		}));
		assert!(!picker_prompt_visible(&WorldMapView::default()));
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
