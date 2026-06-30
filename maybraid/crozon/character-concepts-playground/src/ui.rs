use bevy::prelude::*;
use crozon_characters::{
	presets::{BuildPreset, GenderPreset},
	species::braidman::{
		assets::{
			BodyMesh, ClothingMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh,
		},
		BraidmanConfig,
	},
};
use game_commands::ui::{GameCommandDrawerConfig, GameCommandStatusText, GameCommandUiConfig};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

const PANEL_WIDTH: f32 = 360.0;
const BUTTON_HEIGHT: f32 = 22.0;
const SLIDER_STEP: f32 = 0.05;

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

#[derive(Component)]
pub struct CreatorUiRoot;

#[derive(Component, Clone, Copy)]
pub enum CreatorUiAction {
	Gender(i32),
	Build(i32),
	Body(i32),
	Head(i32),
	Eye(i32),
	Nose(i32),
	Mouth(i32),
	Ear(i32),
	Hair(i32),
	Animation(i32),
	ToggleClothing(ClothingMesh),
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

pub fn setup_creator_ui(mut commands: Commands, config: Res<ConceptPreviewConfig>) {
	spawn_creator_ui(&mut commands, &config);
}

pub fn sync_creator_ui(
	mut commands: Commands,
	config: Res<ConceptPreviewConfig>,
	roots: Query<Entity, With<CreatorUiRoot>>,
) {
	if !config.is_changed() {
		return;
	}
	for root in &roots {
		commands.entity(root).despawn();
	}
	spawn_creator_ui(&mut commands, &config);
}

pub fn react_creator_ui(
	mut interactions: Query<(&Interaction, &CreatorUiAction), (Changed<Interaction>, With<Button>)>,
	mut config: ResMut<ConceptPreviewConfig>,
) {
	for (interaction, action) in &mut interactions {
		if *interaction != Interaction::Pressed {
			continue;
		}
		let ConceptPreviewConfig::Braidman { config: braidman, animation } = config.as_mut();
		match *action {
			CreatorUiAction::Gender(delta) => {
				braidman.gender = cycle(GENDERS, braidman.gender, delta)
			}
			CreatorUiAction::Build(delta) => braidman.build = cycle(BUILDS, braidman.build, delta),
			CreatorUiAction::Body(delta) => braidman.body = cycle(BODIES, braidman.body, delta),
			CreatorUiAction::Head(delta) => braidman.head = cycle(HEADS, braidman.head, delta),
			CreatorUiAction::Eye(delta) => braidman.eye = cycle(EYES, braidman.eye, delta),
			CreatorUiAction::Nose(delta) => braidman.nose = cycle(NOSES, braidman.nose, delta),
			CreatorUiAction::Mouth(delta) => braidman.mouth = cycle(MOUTHS, braidman.mouth, delta),
			CreatorUiAction::Ear(delta) => braidman.ear = cycle(EARS, braidman.ear, delta),
			CreatorUiAction::Hair(delta) => braidman.hair = cycle(HAIRS, braidman.hair, delta),
			CreatorUiAction::Animation(delta) => *animation = cycle(ANIMATIONS, *animation, delta),
			CreatorUiAction::ToggleClothing(clothing) => {
				toggle_clothing(&mut braidman.clothing, clothing);
			}
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

fn spawn_creator_ui(commands: &mut Commands, config: &ConceptPreviewConfig) {
	let ConceptPreviewConfig::Braidman { config: braidman, animation } = config;
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
			text(panel, "First UI cut: selectors, sliders, clothing toggles.", 10.0, muted());
			selector(panel, "Gender", braidman.gender.label(), CreatorUiAction::Gender);
			selector(panel, "Build", braidman.build.label(), CreatorUiAction::Build);
			selector(panel, "Body", braidman.body.label(), CreatorUiAction::Body);
			selector(panel, "Head", braidman.head.label(), CreatorUiAction::Head);
			selector(panel, "Eye", braidman.eye.label(), CreatorUiAction::Eye);
			selector(panel, "Nose", braidman.nose.label(), CreatorUiAction::Nose);
			selector(panel, "Mouth", braidman.mouth.label(), CreatorUiAction::Mouth);
			selector(panel, "Ear", braidman.ear.label(), CreatorUiAction::Ear);
			selector(panel, "Hair", braidman.hair.label(), CreatorUiAction::Hair);
			selector(panel, "Animation", animation.label(), CreatorUiAction::Animation);
			slider(
				panel,
				"Shoulders",
				braidman.sliders.shoulder_width,
				CreatorUiAction::ShoulderWidth,
			);
			slider(panel, "Hips", braidman.sliders.hip_width, CreatorUiAction::HipWidth);
			slider(
				panel,
				"Chest",
				braidman.sliders.chest_thickness,
				CreatorUiAction::ChestThickness,
			);
			text(panel, "Clothing", 12.0, Color::WHITE);
			clothing_grid(panel, braidman);
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

fn clothing_grid(parent: &mut ChildSpawnerCommands, braidman: &BraidmanConfig) {
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Row,
				flex_wrap: FlexWrap::Wrap,
				column_gap: Val::Px(4.0),
				row_gap: Val::Px(4.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|grid| {
			for clothing in CLOTHING {
				let active = braidman.clothing.contains(clothing);
				button(grid, clothing.label(), CreatorUiAction::ToggleClothing(*clothing), active);
			}
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
