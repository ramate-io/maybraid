pub mod camera_focus;
pub mod sliders;

use bevy::prelude::*;
use crozon_characters::{
	presets::{BuildPreset, GenderPreset},
	species::braidman::{
		assets::{
			BodyMesh, ClothingMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh,
		},
		BraidmanColor, BraidmanConfig,
	},
};

use crate::{
	animation::ConceptAnimation,
	preview::ConceptPreviewConfig,
	preview_color::PreviewColor,
	thumbnail::{self, ThumbnailCache},
	ui::{
		color_swatches, inline_color_swatches, section, selector, subsection, text,
		CreatorUiAction as ShellAction, CreatorUiState, CreatorUiValueBinding, THUMBNAIL_SIZE,
		UiAssetTarget as ShellTarget, UiSection,
	},
};

pub use camera_focus::CameraFocus;

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiColorTarget {
	Body,
	Eyes,
	Mouth,
	Hair,
	Clothing(ClothingMesh),
}

#[derive(Component, Clone, Copy, Debug)]
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
	HipThickness(f32),
	LegThickness(f32),
	ButtocksThickness(f32),
	WaistThickness(f32),
	LowerTrunkThickness(f32),
	ArmLength(f32),
	ArmThickness(f32),
	LegLength(f32),
	EyeWidth(f32),
	EyeHeight(f32),
	EyeTilt(f32),
	NoseWidth(f32),
	NoseHeight(f32),
	MouthWidth(f32),
	MouthHeight(f32),
	EarWidth(f32),
	EarHeight(f32),
}

impl CreatorUiAction {
	pub fn focus_target(self) -> Option<UiAssetTarget> {
		match self {
			Self::Body(value) => Some(UiAssetTarget::Body(value)),
			Self::Head(value) => Some(UiAssetTarget::Head(value)),
			Self::Eye(value) => Some(UiAssetTarget::Eye(value)),
			Self::Nose(value) => Some(UiAssetTarget::Nose(value)),
			Self::Mouth(value) => Some(UiAssetTarget::Mouth(value)),
			Self::Ear(value) => Some(UiAssetTarget::Ear(value)),
			Self::Hair(value) => Some(UiAssetTarget::Hair(value)),
			Self::Animation(_) => None,
			Self::ToggleClothing(value) => Some(UiAssetTarget::Clothing(value)),
			Self::ToggleSection(_)
			| Self::Gender(_)
			| Self::Build(_)
			| Self::SetColor(_, _)
			| Self::ShoulderWidth(_)
			| Self::HipWidth(_)
			| Self::ChestThickness(_)
			| Self::HipThickness(_)
			| Self::LegThickness(_)
			| Self::ButtocksThickness(_)
			| Self::WaistThickness(_)
			| Self::LowerTrunkThickness(_)
			| Self::ArmLength(_)
			| Self::ArmThickness(_)
			| Self::LegLength(_)
			| Self::EyeWidth(_)
			| Self::EyeHeight(_)
			| Self::EyeTilt(_)
			| Self::NoseWidth(_)
			| Self::NoseHeight(_)
			| Self::MouthWidth(_)
			| Self::MouthHeight(_)
			| Self::EarWidth(_)
			| Self::EarHeight(_) => None,
		}
	}
}

pub fn apply_action(
	action: CreatorUiAction,
	braidman: &mut BraidmanConfig,
	animation: &mut ConceptAnimation,
	ui_state: &mut CreatorUiState,
) {
	match action {
		CreatorUiAction::ToggleSection(section) => ui_state.toggle(section),
		CreatorUiAction::Gender(delta) => {
			braidman.gender = cycle(GENDERS, braidman.gender, delta);
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
		CreatorUiAction::ToggleClothing(clothing) => toggle_clothing(&mut braidman.clothing, clothing),
		CreatorUiAction::SetColor(target, color) => set_color(braidman, target, color),
		other if sliders::apply_action(&mut braidman.sliders, other) => {}
		_ => {}
	}
}

pub fn populate_panel(
	panel: &mut ChildSpawnerCommands,
	asset_server: &AssetServer,
	images: &mut Assets<Image>,
	thumbnails: &mut ThumbnailCache,
	config: &ConceptPreviewConfig,
	ui_state: &CreatorUiState,
) {
	let ConceptPreviewConfig::Braidman { config: braidman, animation } = config else {
		return;
	};
	let uis = PanelContext { ui_state, braidman, animation: *animation };

	section(
		panel,
		UiSection::Presets,
		ui_state,
		ShellAction::Braidman(CreatorUiAction::ToggleSection(UiSection::Presets)),
		|section| {
		selector(
			section,
			"Gender",
			CreatorUiValueBinding::Gender,
			|delta| ShellAction::Braidman(CreatorUiAction::Gender(delta)),
		);
		selector(
			section,
			"Build",
			CreatorUiValueBinding::Build,
			|delta| ShellAction::Braidman(CreatorUiAction::Build(delta)),
		);
	},
	);
	section(
		panel,
		UiSection::Body,
		ui_state,
		ShellAction::Braidman(CreatorUiAction::ToggleSection(UiSection::Body)),
		|section| {
		subsection(section, "Body Mesh", |sub| {
			asset_grid(
				sub,
				asset_server,
				images,
				thumbnails,
				BODIES.iter().map(|value| UiAssetTarget::Body(*value)),
				uis,
			);
		});
		subsection(section, "Proportions", |sub| {
			sliders::spawn_body(sub, braidman);
		});
		subsection(section, "Color", |sub| {
			color_swatches(sub, UiColorTarget::Body, braidman.colors.body);
		});
	},
	);
	section(
		panel,
		UiSection::HeadFeatures,
		ui_state,
		ShellAction::Braidman(CreatorUiAction::ToggleSection(UiSection::HeadFeatures)),
		|section| {
		subsection(section, "Head", |sub| {
			asset_grid(
				sub,
				asset_server,
				images,
				thumbnails,
				HEADS.iter().map(|value| UiAssetTarget::Head(*value)),
				uis,
			);
		});
		subsection(section, "Eyes", |sub| {
			asset_grid(
				sub,
				asset_server,
				images,
				thumbnails,
				EYES.iter().map(|value| UiAssetTarget::Eye(*value)),
				uis,
			);
			sliders::spawn_eyes(sub, braidman);
			color_swatches(sub, UiColorTarget::Eyes, braidman.colors.eyes);
		});
		subsection(section, "Nose", |sub| {
			asset_grid(
				sub,
				asset_server,
				images,
				thumbnails,
				NOSES.iter().map(|value| UiAssetTarget::Nose(*value)),
				uis,
			);
			sliders::spawn_nose(sub, braidman);
		});
		subsection(section, "Mouth", |sub| {
			asset_grid(
				sub,
				asset_server,
				images,
				thumbnails,
				MOUTHS.iter().map(|value| UiAssetTarget::Mouth(*value)),
				uis,
			);
			sliders::spawn_mouth(sub, braidman);
			color_swatches(sub, UiColorTarget::Mouth, braidman.colors.mouth);
		});
		subsection(section, "Ears", |sub| {
			asset_grid(
				sub,
				asset_server,
				images,
				thumbnails,
				EARS.iter().map(|value| UiAssetTarget::Ear(*value)),
				uis,
			);
			sliders::spawn_ears(sub, braidman);
		});
	},
	);
	section(
		panel,
		UiSection::Hair,
		ui_state,
		ShellAction::Braidman(CreatorUiAction::ToggleSection(UiSection::Hair)),
		|section| {
		subsection(section, "Style", |sub| {
			asset_grid(
				sub,
				asset_server,
				images,
				thumbnails,
				HAIRS.iter().map(|value| UiAssetTarget::Hair(*value)),
				uis,
			);
		});
		subsection(section, "Color", |sub| {
			color_swatches(sub, UiColorTarget::Hair, braidman.colors.hair);
		});
	},
	);
	section(
		panel,
		UiSection::Clothing,
		ui_state,
		ShellAction::Braidman(CreatorUiAction::ToggleSection(UiSection::Clothing)),
		|section| {
		clothing_list(section, asset_server, images, thumbnails, braidman, uis);
	},
	);
	section(
		panel,
		UiSection::Animation,
		ui_state,
		ShellAction::Braidman(CreatorUiAction::ToggleSection(UiSection::Animation)),
		|section| {
		subsection(section, "Clip", |sub| {
			asset_grid(
				sub,
				asset_server,
				images,
				thumbnails,
				ANIMATIONS.iter().map(|value| UiAssetTarget::Animation(*value)),
				uis,
			);
		});
	},
	);
}

#[derive(Clone, Copy)]
struct PanelContext<'a> {
	ui_state: &'a CreatorUiState,
	braidman: &'a BraidmanConfig,
	animation: ConceptAnimation,
}

fn clothing_list(
	parent: &mut ChildSpawnerCommands,
	asset_server: &AssetServer,
	images: &mut Assets<Image>,
	thumbnails: &mut ThumbnailCache,
	braidman: &BraidmanConfig,
	ctx: PanelContext,
) {
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(8.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|list| {
			for clothing in CLOTHING {
				let target = UiAssetTarget::Clothing(*clothing);
				let active = braidman.clothing.contains(clothing);
				let focus = ctx.ui_state.focused_target() == Some(wrap(target));
				let color = braidman.colors.clothing_color(*clothing);
				list
					.spawn((
						Node {
							width: Val::Percent(100.0),
							flex_direction: FlexDirection::Column,
							row_gap: Val::Px(4.0),
							padding: UiRect::all(Val::Px(4.0)),
							..default()
						},
						BackgroundColor(Color::srgba(0.10, 0.11, 0.14, 0.65)),
						Pickable::IGNORE,
					))
					.with_children(|item| {
				let camera = thumbnail::camera_for_target(
					&mut item.commands(),
					images,
					asset_server,
					thumbnails,
					wrap(target),
					clothing.path().as_str(),
					PreviewColor::Braidman(color),
				);
						asset_button(item, target, active, focus, Some(camera));
						inline_color_swatches(
							item,
							UiColorTarget::Clothing(*clothing),
							color,
						);
					});
			}
		});
}

fn asset_grid(
	parent: &mut ChildSpawnerCommands,
	asset_server: &AssetServer,
	images: &mut Assets<Image>,
	thumbnails: &mut ThumbnailCache,
	targets: impl Iterator<Item = UiAssetTarget>,
	ctx: PanelContext,
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
				let active = target_active(target, ctx.braidman, ctx.animation);
				let focus = ctx.ui_state.focused_target() == Some(wrap(target));
				let color = target_color(target, ctx.braidman);
				let camera = target_path(target).map(|path| {
					let mut commands = grid.commands();
					thumbnail::camera_for_target(
						&mut commands,
						images,
						asset_server,
						thumbnails,
						wrap(target),
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
			ShellAction::Braidman(target_action(target)),
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

fn target_color(target: UiAssetTarget, braidman: &BraidmanConfig) -> PreviewColor {
	let skin = braidman.colors.skin_color();
	let color = match target {
		UiAssetTarget::Body(_) => braidman.colors.body,
		UiAssetTarget::Head(_) | UiAssetTarget::Nose(_) | UiAssetTarget::Ear(_) => skin,
		UiAssetTarget::Eye(_) => braidman.colors.eyes,
		UiAssetTarget::Mouth(_) => braidman.colors.mouth,
		UiAssetTarget::Hair(_) => braidman.colors.hair,
		UiAssetTarget::Clothing(value) => braidman.colors.clothing_color(value),
		UiAssetTarget::Animation(_) => BraidmanColor::Natural,
	};
	PreviewColor::Braidman(color)
}

fn wrap(target: UiAssetTarget) -> ShellTarget {
	ShellTarget::Braidman(target)
}

fn set_color(braidman: &mut BraidmanConfig, target: UiColorTarget, color: BraidmanColor) {
	match target {
		UiColorTarget::Body => {
			braidman.colors.body = color;
			braidman.colors.sync_skin_from_body();
		}
		UiColorTarget::Eyes => braidman.colors.eyes = color,
		UiColorTarget::Mouth => braidman.colors.mouth = color,
		UiColorTarget::Hair => braidman.colors.hair = color,
		UiColorTarget::Clothing(clothing) => braidman.colors.set_clothing_color(clothing, color),
	}
}

pub fn color_target_label(target: UiColorTarget) -> &'static str {
	match target {
		UiColorTarget::Body => "Body color",
		UiColorTarget::Eyes => "Eye color",
		UiColorTarget::Mouth => "Mouth color",
		UiColorTarget::Hair => "Hair color",
		UiColorTarget::Clothing(_) => "Clothing color",
	}
}

pub const COLORS: &[BraidmanColor] = &[
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

pub fn format_value_binding(binding: CreatorUiValueBinding, braidman: &BraidmanConfig) -> String {
	let sliders = &braidman.sliders;
	match binding {
		CreatorUiValueBinding::Gender => braidman.gender.label().into(),
		CreatorUiValueBinding::Build => braidman.build.label().into(),
		CreatorUiValueBinding::ShoulderWidth => format!("{:.2}", sliders.shoulder_width),
		CreatorUiValueBinding::HipWidth => format!("{:.2}", sliders.hip_width),
		CreatorUiValueBinding::ChestThickness => format!("{:.2}", sliders.chest_thickness),
		CreatorUiValueBinding::HipThickness => format!("{:.2}", sliders.hip_thickness),
		CreatorUiValueBinding::LegThickness => format!("{:.2}", sliders.leg_thickness),
		CreatorUiValueBinding::ButtocksThickness => {
			format!("{:.2}", sliders.buttocks_thickness)
		}
		CreatorUiValueBinding::WaistThickness => format!("{:.2}", sliders.waist_thickness),
		CreatorUiValueBinding::LowerTrunkThickness => {
			format!("{:.2}", sliders.lower_trunk_thickness)
		}
		CreatorUiValueBinding::ArmLength => format!("{:.2}", sliders.arm_length),
		CreatorUiValueBinding::ArmThickness => format!("{:.2}", sliders.arm_thickness),
		CreatorUiValueBinding::LegLength => format!("{:.2}", sliders.leg_length),
		CreatorUiValueBinding::EyeWidth => format!("{:.2}", sliders.eye_width),
		CreatorUiValueBinding::EyeHeight => format!("{:.2}", sliders.eye_height),
		CreatorUiValueBinding::EyeTilt => format!("{:.1} deg.", sliders.eye_tilt),
		CreatorUiValueBinding::NoseWidth => format!("{:.2}", sliders.nose_width),
		CreatorUiValueBinding::NoseHeight => format!("{:.2}", sliders.nose_height),
		CreatorUiValueBinding::MouthWidth => format!("{:.2}", sliders.mouth_width),
		CreatorUiValueBinding::MouthHeight => format!("{:.2}", sliders.mouth_height),
		CreatorUiValueBinding::EarWidth => format!("{:.2}", sliders.ear_width),
		CreatorUiValueBinding::EarHeight => format!("{:.2}", sliders.ear_height),
	}
}

pub fn color_target_value(target: UiColorTarget, braidman: &BraidmanConfig) -> BraidmanColor {
	match target {
		UiColorTarget::Body => braidman.colors.body,
		UiColorTarget::Eyes => braidman.colors.eyes,
		UiColorTarget::Mouth => braidman.colors.mouth,
		UiColorTarget::Hair => braidman.colors.hair,
		UiColorTarget::Clothing(clothing) => braidman.colors.clothing_color(clothing),
	}
}

pub fn selection_button_color(
	action: CreatorUiAction,
	braidman: &BraidmanConfig,
	animation: ConceptAnimation,
	ui_state: &CreatorUiState,
) -> Option<Color> {
	const INACTIVE: Color = Color::srgba(0.18, 0.20, 0.24, 0.92);
	const ACTIVE: Color = Color::srgba(0.16, 0.34, 0.50, 0.95);

	match action {
		CreatorUiAction::ToggleSection(section) => {
			Some(if ui_state.is_open(section) { ACTIVE } else { INACTIVE })
		}
		CreatorUiAction::Body(value) => Some(asset_button_color(
			UiAssetTarget::Body(value),
			braidman,
			animation,
			ui_state,
		)),
		CreatorUiAction::Head(value) => Some(asset_button_color(
			UiAssetTarget::Head(value),
			braidman,
			animation,
			ui_state,
		)),
		CreatorUiAction::Eye(value) => Some(asset_button_color(
			UiAssetTarget::Eye(value),
			braidman,
			animation,
			ui_state,
		)),
		CreatorUiAction::Nose(value) => Some(asset_button_color(
			UiAssetTarget::Nose(value),
			braidman,
			animation,
			ui_state,
		)),
		CreatorUiAction::Mouth(value) => Some(asset_button_color(
			UiAssetTarget::Mouth(value),
			braidman,
			animation,
			ui_state,
		)),
		CreatorUiAction::Ear(value) => Some(asset_button_color(
			UiAssetTarget::Ear(value),
			braidman,
			animation,
			ui_state,
		)),
		CreatorUiAction::Hair(value) => Some(asset_button_color(
			UiAssetTarget::Hair(value),
			braidman,
			animation,
			ui_state,
		)),
		CreatorUiAction::ToggleClothing(value) => Some(asset_button_color(
			UiAssetTarget::Clothing(value),
			braidman,
			animation,
			ui_state,
		)),
		CreatorUiAction::Animation(value) => Some(asset_button_color(
			UiAssetTarget::Animation(value),
			braidman,
			animation,
			ui_state,
		)),
		_ => None,
	}
}

fn asset_button_color(
	target: UiAssetTarget,
	braidman: &BraidmanConfig,
	animation: ConceptAnimation,
	ui_state: &CreatorUiState,
) -> Color {
	const INACTIVE: Color = Color::srgba(0.18, 0.20, 0.24, 0.92);
	const ACTIVE: Color = Color::srgba(0.16, 0.34, 0.50, 0.95);
	const FOCUS: Color = Color::srgba(0.30, 0.38, 0.48, 0.98);

	let active = target_active(target, braidman, animation);
	let focus = ui_state.focused_target() == Some(wrap(target));
	if focus {
		FOCUS
	} else if active {
		ACTIVE
	} else {
		INACTIVE
	}
}
