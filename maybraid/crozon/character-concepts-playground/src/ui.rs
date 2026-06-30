use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, TextureFormat, TextureUsages};
use crozon_characters::{
	presets::{BuildPreset, GenderPreset},
	species::braidman::{
		assets::{
			BodyMesh, ClothingMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh,
		},
		BraidmanColor, BraidmanConfig,
	},
};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText, GameCommandUiConfig};

use crate::{
	animation::ConceptAnimation,
	camera_focus::CameraSuggestion,
	preview::ConceptPreviewConfig,
	thumbnail::{self, ThumbnailCache},
};

const PANEL_WIDTH: f32 = 360.0;
const BUTTON_HEIGHT: f32 = 22.0;
const SLIDER_STEP: f32 = 0.05;
const THUMBNAIL_SIZE: f32 = 54.0;

const GENDERS: &[GenderPreset] =
	&[GenderPreset::Neutral, GenderPreset::Male, GenderPreset::Female, GenderPreset::NonBinary];
const BUILDS: &[BuildPreset] = &[
	BuildPreset::Average,
	BuildPreset::Slender,
	BuildPreset::Athletic,
	BuildPreset::Heavy,
	BuildPreset::Stocky,
	BuildPreset::Lanky,
];
const BODIES: &[BodyMesh] = &[BodyMesh::Standard, BodyMesh::Full];
const HEADS: &[HeadMesh] = &[HeadMesh::Standard, HeadMesh::Gaunt, HeadMesh::Full];
const EYES: &[EyeMesh] = &[EyeMesh::Standard, EyeMesh::Falcon];
const NOSES: &[NoseMesh] =
	&[NoseMesh::Standard, NoseMesh::Broad, NoseMesh::Loaf, NoseMesh::Balloon];
const MOUTHS: &[MouthMesh] = &[MouthMesh::Standard];
const EARS: &[EarMesh] = &[EarMesh::Standard, EarMesh::Round, EarMesh::Flank];
const HAIRS: &[HairMesh] = &[
	HairMesh::None,
	HairMesh::ThickBraids,
	HairMesh::FlowingCurls,
	HairMesh::WrappingBraids,
	HairMesh::WrappingBraidsHangingLocks,
	HairMesh::BraidHawk,
	HairMesh::FeatherHawk,
	HairMesh::FlowingEdgyCurls,
	HairMesh::PermBraid,
	HairMesh::TechnoEdge,
];
const CLOTHING: &[ClothingMesh] = &[
	ClothingMesh::BasketballCutShirt,
	ClothingMesh::Tunic,
	ClothingMesh::LongDress,
	ClothingMesh::ShortDress,
	ClothingMesh::FittedCoat,
	ClothingMesh::QuarterCoat,
	ClothingMesh::RobeCoat,
	ClothingMesh::ShortSleevedRobeCoat,
	ClothingMesh::TailoredCoat,
	ClothingMesh::Hood,
];
const ANIMATIONS: &[ConceptAnimation] = &[
	ConceptAnimation::Still,
	ConceptAnimation::Walk,
	ConceptAnimation::Run,
	ConceptAnimation::Jump,
	ConceptAnimation::Tuck,
	ConceptAnimation::TuckedFlip,
	ConceptAnimation::TwoFootedTuckedFlip,
];
const COLORS: &[BraidmanColor] = &[
	BraidmanColor::Natural,
	BraidmanColor::Warm,
	BraidmanColor::Cool,
	BraidmanColor::Dark,
	BraidmanColor::Light,
	BraidmanColor::Red,
	BraidmanColor::Blue,
	BraidmanColor::Green,
	BraidmanColor::Gold,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UiSection {
	#[default]
	Presets,
	Body,
	HeadFeatures,
	Hair,
	Clothing,
	Animation,
}

impl UiSection {
	const fn label(self) -> &'static str {
		match self {
			Self::Presets => "Presets",
			Self::Body => "Body",
			Self::HeadFeatures => "Head & Features",
			Self::Hair => "Hair",
			Self::Clothing => "Clothing",
			Self::Animation => "Animation",
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UiAssetTarget {
	Body(BodyMesh),
	Head(HeadMesh),
	Eye(EyeMesh),
	Nose(NoseMesh),
	Mouth(MouthMesh),
	Ear(EarMesh),
	Hair(HairMesh),
	Clothing(ClothingMesh),
	Animation(ConceptAnimation),
}

impl UiAssetTarget {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Body(value) => value.label(),
			Self::Head(value) => value.label(),
			Self::Eye(value) => value.label(),
			Self::Nose(value) => value.label(),
			Self::Mouth(value) => value.label(),
			Self::Ear(value) => value.label(),
			Self::Hair(value) => value.label(),
			Self::Clothing(value) => value.label(),
			Self::Animation(value) => value.label(),
		}
	}

	pub const fn camera_suggestion(self) -> CameraSuggestion {
		match self {
			Self::Body(_) | Self::Animation(_) => CameraSuggestion::FullBody,
			Self::Clothing(_) => CameraSuggestion::Torso,
			Self::Head(_) | Self::Hair(_) => CameraSuggestion::Head,
			Self::Eye(_) => CameraSuggestion::Eyes,
			Self::Nose(_) | Self::Mouth(_) => CameraSuggestion::Face,
			Self::Ear(_) => CameraSuggestion::Ears,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiColorTarget {
	Body,
	Head,
	Eyes,
	Nose,
	Mouth,
	Ears,
	Hair,
	Clothing(ClothingMesh),
}

#[derive(Resource, Debug)]
pub struct CreatorUiState {
	pub presets_open: bool,
	pub body_open: bool,
	pub head_features_open: bool,
	pub hair_open: bool,
	pub clothing_open: bool,
	pub animation_open: bool,
	pub hovered: Option<UiAssetTarget>,
	pub last_selected: Option<UiAssetTarget>,
	layout_revision: u64,
}

impl Default for CreatorUiState {
	fn default() -> Self {
		Self {
			presets_open: true,
			body_open: true,
			head_features_open: false,
			hair_open: false,
			clothing_open: true,
			animation_open: false,
			hovered: None,
			last_selected: None,
			layout_revision: 0,
		}
	}
}

impl CreatorUiState {
	pub const fn focused_target(&self) -> Option<UiAssetTarget> {
		match self.hovered {
			Some(target) => Some(target),
			None => self.last_selected,
		}
	}

	fn is_open(&self, section: UiSection) -> bool {
		match section {
			UiSection::Presets => self.presets_open,
			UiSection::Body => self.body_open,
			UiSection::HeadFeatures => self.head_features_open,
			UiSection::Hair => self.hair_open,
			UiSection::Clothing => self.clothing_open,
			UiSection::Animation => self.animation_open,
		}
	}

	fn toggle(&mut self, section: UiSection) {
		match section {
			UiSection::Presets => self.presets_open = !self.presets_open,
			UiSection::Body => self.body_open = !self.body_open,
			UiSection::HeadFeatures => self.head_features_open = !self.head_features_open,
			UiSection::Hair => self.hair_open = !self.hair_open,
			UiSection::Clothing => self.clothing_open = !self.clothing_open,
			UiSection::Animation => self.animation_open = !self.animation_open,
		}
		self.layout_revision += 1;
	}
}

#[derive(Resource, Default)]
pub struct CreatorUiSyncState {
	config_key: String,
	layout_revision: u64,
}

#[derive(Component)]
pub struct CreatorUiRoot;

#[derive(Component, Clone, Copy)]
pub enum CreatorUiAction {
	ToggleSection(UiSection),
	Gender(i32),
	Build(i32),
	Body(BodyMesh),
	Head(HeadMesh),
	Eye(EyeMesh),
	Nose(NoseMesh),
	Mouth(MouthMesh),
	Ear(EarMesh),
	Hair(HairMesh),
	Animation(ConceptAnimation),
	ToggleClothing(ClothingMesh),
	SetColor(UiColorTarget, BraidmanColor),
	ShoulderWidth(f32),
	HipWidth(f32),
	ChestThickness(f32),
}

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Crozon character concepts - / cmd - F1 drawer - L look - WASD".into(),
		empty_console_text: "Console: (errors & `help` output) - wheel or PgUp/PgDn".into(),
		root_background: Color::srgba(0.12, 0.14, 0.18, 0.82),
		controls_hint: "F1 hide drawer - L lock look - help - Enter - up/down history".into(),
	}
}

pub fn drawer_config() -> GameCommandDrawerConfig {
	GameCommandDrawerConfig { open_at_start: false, ..GameCommandDrawerConfig::default() }
}

pub(crate) fn sync_command_status_text(
	config: Res<ConceptPreviewConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = config.status_label();
}

pub fn setup_creator_ui(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut images: ResMut<Assets<Image>>,
	mut thumbnails: ResMut<ThumbnailCache>,
	config: Res<ConceptPreviewConfig>,
	ui_state: Res<CreatorUiState>,
	mut sync_state: ResMut<CreatorUiSyncState>,
) {
	sync_state.config_key = config.sync_key();
	sync_state.layout_revision = ui_state.layout_revision;
	spawn_creator_ui(
		&mut commands,
		asset_server.as_ref(),
		&mut images,
		&mut thumbnails,
		&config,
		&ui_state,
	);
}

pub fn sync_creator_ui(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut images: ResMut<Assets<Image>>,
	mut thumbnails: ResMut<ThumbnailCache>,
	config: Res<ConceptPreviewConfig>,
	ui_state: Res<CreatorUiState>,
	mut sync_state: ResMut<CreatorUiSyncState>,
	roots: Query<Entity, With<CreatorUiRoot>>,
) {
	let key = config.sync_key();
	if sync_state.config_key == key && sync_state.layout_revision == ui_state.layout_revision {
		return;
	}
	sync_state.config_key = key;
	sync_state.layout_revision = ui_state.layout_revision;
	for root in &roots {
		commands.entity(root).despawn();
	}
	spawn_creator_ui(
		&mut commands,
		asset_server.as_ref(),
		&mut images,
		&mut thumbnails,
		&config,
		&ui_state,
	);
}

pub fn react_creator_ui(
	mut interactions: Query<(&Interaction, &CreatorUiAction), (Changed<Interaction>, With<Button>)>,
	mut config: ResMut<ConceptPreviewConfig>,
	mut ui_state: ResMut<CreatorUiState>,
) {
	for (interaction, action) in &mut interactions {
		if *interaction == Interaction::Hovered {
			if let Some(target) = action.focus_target() {
				ui_state.hovered = Some(target);
			}
			continue;
		}
		if *interaction != Interaction::Pressed {
			continue;
		}
		if let Some(target) = action.focus_target() {
			ui_state.last_selected = Some(target);
		}
		let ConceptPreviewConfig::Braidman { config: braidman, animation } = config.as_mut();
		match *action {
			CreatorUiAction::ToggleSection(section) => ui_state.toggle(section),
			CreatorUiAction::Gender(delta) => {
				braidman.gender = cycle(GENDERS, braidman.gender, delta)
			}
			CreatorUiAction::Build(delta) => braidman.build = cycle(BUILDS, braidman.build, delta),
			CreatorUiAction::Body(value) => braidman.body = value,
			CreatorUiAction::Head(value) => braidman.head = value,
			CreatorUiAction::Eye(value) => braidman.eye = value,
			CreatorUiAction::Nose(value) => braidman.nose = value,
			CreatorUiAction::Mouth(value) => braidman.mouth = value,
			CreatorUiAction::Ear(value) => braidman.ear = value,
			CreatorUiAction::Hair(value) => braidman.hair = value,
			CreatorUiAction::Animation(value) => *animation = value,
			CreatorUiAction::ToggleClothing(clothing) => {
				toggle_clothing(&mut braidman.clothing, clothing);
			}
			CreatorUiAction::SetColor(target, color) => set_color(braidman, target, color),
			CreatorUiAction::ShoulderWidth(delta) => {
				braidman.sliders =
					braidman.sliders.with_shoulder_width(braidman.sliders.shoulder_width + delta);
			}
			CreatorUiAction::HipWidth(delta) => {
				braidman.sliders =
					braidman.sliders.with_hip_width(braidman.sliders.hip_width + delta);
			}
			CreatorUiAction::ChestThickness(delta) => {
				braidman.sliders =
					braidman.sliders.with_chest_thickness(braidman.sliders.chest_thickness + delta);
			}
		}
	}
}

impl CreatorUiAction {
	fn focus_target(self) -> Option<UiAssetTarget> {
		match self {
			Self::Body(value) => Some(UiAssetTarget::Body(value)),
			Self::Head(value) => Some(UiAssetTarget::Head(value)),
			Self::Eye(value) => Some(UiAssetTarget::Eye(value)),
			Self::Nose(value) => Some(UiAssetTarget::Nose(value)),
			Self::Mouth(value) => Some(UiAssetTarget::Mouth(value)),
			Self::Ear(value) => Some(UiAssetTarget::Ear(value)),
			Self::Hair(value) => Some(UiAssetTarget::Hair(value)),
			Self::Animation(value) => Some(UiAssetTarget::Animation(value)),
			Self::ToggleClothing(value) => Some(UiAssetTarget::Clothing(value)),
			Self::ToggleSection(_)
			| Self::Gender(_)
			| Self::Build(_)
			| Self::SetColor(_, _)
			| Self::ShoulderWidth(_)
			| Self::HipWidth(_)
			| Self::ChestThickness(_) => None,
		}
	}
}

fn spawn_creator_ui(
	commands: &mut Commands,
	asset_server: &AssetServer,
	images: &mut Assets<Image>,
	thumbnails: &mut ThumbnailCache,
	config: &ConceptPreviewConfig,
	ui_state: &CreatorUiState,
) {
	let ConceptPreviewConfig::Braidman { config: braidman, animation } = config;
	thumbnails.begin_ui_rebuild();
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(10.0),
				right: Val::Px(10.0),
				width: Val::Px(PANEL_WIDTH),
				max_height: Val::Percent(82.0),
				padding: UiRect::all(Val::Px(8.0)),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(5.0),
				overflow: Overflow::scroll_y(),
				..default()
			},
			BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.84)),
			CreatorUiRoot,
		))
		.with_children(|panel| {
			text(panel, "Character Concepts", 15.0, Color::WHITE);
			text(
				panel,
				"Expandable controls, thumbnails, colors, and focus camera.",
				10.0,
				muted(),
			);
			section(panel, UiSection::Presets, ui_state, |section| {
				selector(section, "Gender", braidman.gender.label(), CreatorUiAction::Gender);
				selector(section, "Build", braidman.build.label(), CreatorUiAction::Build);
			});
			section(panel, UiSection::Body, ui_state, |section| {
				asset_grid(
					section,
					asset_server,
					images,
					thumbnails,
					BODIES.iter().map(|value| UiAssetTarget::Body(*value)),
					Uis::new(ui_state, braidman, *animation),
				);
				slider(
					section,
					"Shoulders",
					braidman.sliders.shoulder_width,
					CreatorUiAction::ShoulderWidth,
				);
				slider(section, "Hips", braidman.sliders.hip_width, CreatorUiAction::HipWidth);
				slider(
					section,
					"Chest",
					braidman.sliders.chest_thickness,
					CreatorUiAction::ChestThickness,
				);
				color_swatches(section, UiColorTarget::Body, braidman.colors.body);
			});
			section(panel, UiSection::HeadFeatures, ui_state, |section| {
				asset_grid(
					section,
					asset_server,
					images,
					thumbnails,
					HEADS.iter().map(|value| UiAssetTarget::Head(*value)),
					Uis::new(ui_state, braidman, *animation),
				);
				asset_grid(
					section,
					asset_server,
					images,
					thumbnails,
					EYES.iter().map(|value| UiAssetTarget::Eye(*value)),
					Uis::new(ui_state, braidman, *animation),
				);
				asset_grid(
					section,
					asset_server,
					images,
					thumbnails,
					NOSES.iter().map(|value| UiAssetTarget::Nose(*value)),
					Uis::new(ui_state, braidman, *animation),
				);
				asset_grid(
					section,
					asset_server,
					images,
					thumbnails,
					MOUTHS.iter().map(|value| UiAssetTarget::Mouth(*value)),
					Uis::new(ui_state, braidman, *animation),
				);
				asset_grid(
					section,
					asset_server,
					images,
					thumbnails,
					EARS.iter().map(|value| UiAssetTarget::Ear(*value)),
					Uis::new(ui_state, braidman, *animation),
				);
				color_swatches(section, UiColorTarget::Head, braidman.colors.head);
				color_swatches(section, UiColorTarget::Eyes, braidman.colors.eyes);
				color_swatches(section, UiColorTarget::Nose, braidman.colors.nose);
				color_swatches(section, UiColorTarget::Mouth, braidman.colors.mouth);
				color_swatches(section, UiColorTarget::Ears, braidman.colors.ears);
			});
			section(panel, UiSection::Hair, ui_state, |section| {
				asset_grid(
					section,
					asset_server,
					images,
					thumbnails,
					HAIRS.iter().map(|value| UiAssetTarget::Hair(*value)),
					Uis::new(ui_state, braidman, *animation),
				);
				color_swatches(section, UiColorTarget::Hair, braidman.colors.hair);
			});
			section(panel, UiSection::Clothing, ui_state, |section| {
				asset_grid(
					section,
					asset_server,
					images,
					thumbnails,
					CLOTHING.iter().map(|value| UiAssetTarget::Clothing(*value)),
					Uis::new(ui_state, braidman, *animation),
				);
				if let Some(UiAssetTarget::Clothing(clothing)) = ui_state.focused_target() {
					color_swatches(
						section,
						UiColorTarget::Clothing(clothing),
						braidman.colors.clothing_color(clothing),
					);
				}
			});
			section(panel, UiSection::Animation, ui_state, |section| {
				asset_grid(
					section,
					asset_server,
					images,
					thumbnails,
					ANIMATIONS.iter().map(|value| UiAssetTarget::Animation(*value)),
					Uis::new(ui_state, braidman, *animation),
				);
			});
		});
}

struct Uis<'a> {
	ui_state: &'a CreatorUiState,
	braidman: &'a BraidmanConfig,
	animation: ConceptAnimation,
}

impl<'a> Uis<'a> {
	const fn new(
		ui_state: &'a CreatorUiState,
		braidman: &'a BraidmanConfig,
		animation: ConceptAnimation,
	) -> Self {
		Self { ui_state, braidman, animation }
	}
}

fn section(
	parent: &mut ChildSpawnerCommands,
	section: UiSection,
	state: &CreatorUiState,
	body: impl FnOnce(&mut ChildSpawnerCommands),
) {
	let open = state.is_open(section);
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(4.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|section_parent| {
			button(
				section_parent,
				&format!("{} {}", if open { "v" } else { ">" }, section.label()),
				CreatorUiAction::ToggleSection(section),
				open,
			);
			if open {
				body(section_parent);
			}
		});
}

fn asset_grid(
	parent: &mut ChildSpawnerCommands,
	asset_server: &AssetServer,
	images: &mut Assets<Image>,
	thumbnails: &mut ThumbnailCache,
	targets: impl Iterator<Item = UiAssetTarget>,
	uis: Uis,
) {
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Row,
				flex_wrap: FlexWrap::Wrap,
				column_gap: Val::Px(6.0),
				row_gap: Val::Px(6.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|grid| {
			for target in targets {
				let active = target_active(target, uis.braidman, uis.animation);
				let focus = uis.ui_state.focused_target() == Some(target);
				let color = target_color(target, uis.braidman);
				let camera = target_path(target).map(|path| {
					let mut commands = grid.commands();
					thumbnail::camera_for_target(
						&mut commands,
						images,
						asset_server,
						thumbnails,
						target,
						path,
						color,
					)
				});
				asset_button(grid, target, active, focus, camera);
			}
		});
}

fn asset_button(
	parent: &mut ChildSpawnerCommands,
	target: UiAssetTarget,
	active: bool,
	focus: bool,
	camera: Option<Entity>,
) {
	parent
		.spawn((
			Button,
			Node {
				width: Val::Px(96.0),
				min_height: Val::Px(82.0),
				padding: UiRect::all(Val::Px(4.0)),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(3.0),
				align_items: AlignItems::Center,
				justify_content: JustifyContent::Center,
				..default()
			},
			BackgroundColor(if focus {
				Color::srgba(0.30, 0.38, 0.48, 0.98)
			} else if active {
				Color::srgba(0.16, 0.34, 0.50, 0.95)
			} else {
				Color::srgba(0.18, 0.20, 0.24, 0.92)
			}),
			target_action(target),
		))
		.with_children(|button| {
			if let Some(camera) = camera {
				button.spawn((
					Node {
						width: Val::Px(THUMBNAIL_SIZE),
						height: Val::Px(THUMBNAIL_SIZE),
						..default()
					},
					ViewportNode::new(camera),
				));
			}
			text(button, target.label(), 9.0, Color::WHITE);
		});
}

fn selector(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	value: &'static str,
	action: fn(i32) -> CreatorUiAction,
) {
	parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, label, 11.0, Color::WHITE);
		button(row, "<", action(-1), false);
		text(row, value, 11.0, Color::srgb(0.85, 0.95, 1.0));
		button(row, ">", action(1), false);
	});
}

fn slider(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	value: f32,
	action: fn(f32) -> CreatorUiAction,
) {
	parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, label, 11.0, Color::WHITE);
		button(row, "-", action(-SLIDER_STEP), false);
		text(row, &format!("{value:.2}"), 11.0, Color::srgb(0.85, 0.95, 1.0));
		button(row, "+", action(SLIDER_STEP), false);
	});
}

fn button(parent: &mut ChildSpawnerCommands, label: &str, action: CreatorUiAction, active: bool) {
	parent
		.spawn((
			Button,
			Node {
				min_width: Val::Px(28.0),
				height: Val::Px(BUTTON_HEIGHT),
				padding: UiRect::axes(Val::Px(7.0), Val::Px(2.0)),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(if active {
				Color::srgba(0.16, 0.34, 0.50, 0.95)
			} else {
				Color::srgba(0.18, 0.20, 0.24, 0.92)
			}),
			action,
		))
		.with_children(|button| text(button, label, 10.0, Color::WHITE));
}

fn color_swatches(parent: &mut ChildSpawnerCommands, target: UiColorTarget, active: BraidmanColor) {
	parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, color_target_label(target), 11.0, Color::WHITE);
		for color in COLORS {
			row.spawn((
				Button,
				Node {
					width: Val::Px(22.0),
					height: Val::Px(18.0),
					border: UiRect::all(Val::Px(if *color == active { 2.0 } else { 1.0 })),
					..default()
				},
				BorderColor::all(if *color == active { Color::WHITE } else { muted() }),
				BackgroundColor(color.color()),
				CreatorUiAction::SetColor(target, *color),
			));
		}
	});
}

fn text(parent: &mut ChildSpawnerCommands, value: &str, size: f32, color: Color) {
	parent.spawn((
		Text::new(value.to_string()),
		TextFont { font_size: size, ..default() },
		TextColor(color),
		Pickable::IGNORE,
	));
}

fn row_node() -> Node {
	Node {
		width: Val::Percent(100.0),
		min_height: Val::Px(24.0),
		flex_direction: FlexDirection::Row,
		column_gap: Val::Px(5.0),
		align_items: AlignItems::Center,
		justify_content: JustifyContent::SpaceBetween,
		..default()
	}
}

fn muted() -> Color {
	Color::srgba(0.72, 0.78, 0.86, 1.0)
}

fn cycle<T: Copy + PartialEq>(values: &[T], current: T, delta: i32) -> T {
	let Some(index) = values.iter().position(|value| *value == current) else {
		return current;
	};
	let len = values.len() as i32;
	let next = (index as i32 + delta).rem_euclid(len) as usize;
	values[next]
}

fn toggle_clothing(clothing: &mut Vec<ClothingMesh>, value: ClothingMesh) {
	if let Some(index) = clothing.iter().position(|clothing| *clothing == value) {
		clothing.remove(index);
	} else {
		clothing.push(value);
	}
}

fn target_active(
	target: UiAssetTarget,
	braidman: &BraidmanConfig,
	animation: ConceptAnimation,
) -> bool {
	match target {
		UiAssetTarget::Body(value) => braidman.body == value,
		UiAssetTarget::Head(value) => braidman.head == value,
		UiAssetTarget::Eye(value) => braidman.eye == value,
		UiAssetTarget::Nose(value) => braidman.nose == value,
		UiAssetTarget::Mouth(value) => braidman.mouth == value,
		UiAssetTarget::Ear(value) => braidman.ear == value,
		UiAssetTarget::Hair(value) => braidman.hair == value,
		UiAssetTarget::Clothing(value) => braidman.clothing.contains(&value),
		UiAssetTarget::Animation(value) => animation == value,
	}
}

fn target_action(target: UiAssetTarget) -> CreatorUiAction {
	match target {
		UiAssetTarget::Body(value) => CreatorUiAction::Body(value),
		UiAssetTarget::Head(value) => CreatorUiAction::Head(value),
		UiAssetTarget::Eye(value) => CreatorUiAction::Eye(value),
		UiAssetTarget::Nose(value) => CreatorUiAction::Nose(value),
		UiAssetTarget::Mouth(value) => CreatorUiAction::Mouth(value),
		UiAssetTarget::Ear(value) => CreatorUiAction::Ear(value),
		UiAssetTarget::Hair(value) => CreatorUiAction::Hair(value),
		UiAssetTarget::Clothing(value) => CreatorUiAction::ToggleClothing(value),
		UiAssetTarget::Animation(value) => CreatorUiAction::Animation(value),
	}
}

fn target_path(target: UiAssetTarget) -> Option<&'static str> {
	match target {
		UiAssetTarget::Body(value) => Some(value.path().as_str()),
		UiAssetTarget::Head(value) => Some(value.path().as_str()),
		UiAssetTarget::Eye(value) => Some(value.path().as_str()),
		UiAssetTarget::Nose(value) => Some(value.path().as_str()),
		UiAssetTarget::Mouth(value) => Some(value.path().as_str()),
		UiAssetTarget::Ear(value) => Some(value.path().as_str()),
		UiAssetTarget::Hair(value) => value.path().map(|path| path.as_str()),
		UiAssetTarget::Clothing(value) => Some(value.path().as_str()),
		UiAssetTarget::Animation(_) => None,
	}
}

fn target_color(target: UiAssetTarget, braidman: &BraidmanConfig) -> BraidmanColor {
	match target {
		UiAssetTarget::Body(_) => braidman.colors.body,
		UiAssetTarget::Head(_) => braidman.colors.head,
		UiAssetTarget::Eye(_) => braidman.colors.eyes,
		UiAssetTarget::Nose(_) => braidman.colors.nose,
		UiAssetTarget::Mouth(_) => braidman.colors.mouth,
		UiAssetTarget::Ear(_) => braidman.colors.ears,
		UiAssetTarget::Hair(_) => braidman.colors.hair,
		UiAssetTarget::Clothing(value) => braidman.colors.clothing_color(value),
		UiAssetTarget::Animation(_) => BraidmanColor::Natural,
	}
}

fn set_color(braidman: &mut BraidmanConfig, target: UiColorTarget, color: BraidmanColor) {
	match target {
		UiColorTarget::Body => braidman.colors.body = color,
		UiColorTarget::Head => braidman.colors.head = color,
		UiColorTarget::Eyes => braidman.colors.eyes = color,
		UiColorTarget::Nose => braidman.colors.nose = color,
		UiColorTarget::Mouth => braidman.colors.mouth = color,
		UiColorTarget::Ears => braidman.colors.ears = color,
		UiColorTarget::Hair => braidman.colors.hair = color,
		UiColorTarget::Clothing(clothing) => braidman.colors.set_clothing_color(clothing, color),
	}
}

fn color_target_label(target: UiColorTarget) -> &'static str {
	match target {
		UiColorTarget::Body => "Body color",
		UiColorTarget::Head => "Head color",
		UiColorTarget::Eyes => "Eye color",
		UiColorTarget::Nose => "Nose color",
		UiColorTarget::Mouth => "Mouth color",
		UiColorTarget::Ears => "Ear color",
		UiColorTarget::Hair => "Hair color",
		UiColorTarget::Clothing(_) => "Clothing color",
	}
}

pub fn thumbnail_image() -> Image {
	let mut image = Image::new_uninit(
		default(),
		TextureDimension::D2,
		TextureFormat::Bgra8UnormSrgb,
		RenderAssetUsages::all(),
	);
	image.texture_descriptor.usage =
		TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
	image
}
