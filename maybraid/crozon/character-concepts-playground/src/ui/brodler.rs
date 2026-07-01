pub mod camera_focus;

use bevy::prelude::*;
use crozon_characters::species::{
	braidman::BraidmanColor,
	brodler::{BrodlerConfig, BrodlerEyeColor, BrodlerHeadMesh, BrodlerSkinColor, HornMesh},
	common::{ClothingMesh, HairMesh},
};

use crate::{
	animation::ConceptAnimation,
	preview::ConceptPreviewConfig,
	preview_color::PreviewColor,
	thumbnail::{self, ThumbnailCache},
	ui::{
		section, text, CreatorUiAction as ShellAction, CreatorUiState, THUMBNAIL_SIZE,
		UiAssetTarget as ShellTarget, UiSection,
	},
};

const HEADS: &[BrodlerHeadMesh] = &[BrodlerHeadMesh::Gaunt, BrodlerHeadMesh::Full];
const HORNS: &[HornMesh] = &[HornMesh::HarrowedCrown, HornMesh::LorkenCrown];
const SKIN_COLORS: &[BrodlerSkinColor] =
	&[BrodlerSkinColor::Red, BrodlerSkinColor::Black, BrodlerSkinColor::Yellow];
const EYE_COLORS: &[BrodlerEyeColor] =
	&[BrodlerEyeColor::Red, BrodlerEyeColor::Green, BrodlerEyeColor::Black];
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
const HAIR_COLORS: &[BraidmanColor] = &[
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UiAssetTarget {
	Body,
	Head(BrodlerHeadMesh),
	Horns(HornMesh),
	Eye,
	Hair(HairMesh),
	Clothing(ClothingMesh),
	Animation(ConceptAnimation),
}

impl UiAssetTarget {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Body => "body",
			Self::Head(value) => value.label(),
			Self::Horns(value) => value.label(),
			Self::Eye => "eyes",
			Self::Hair(value) => value.label(),
			Self::Clothing(value) => value.label(),
			Self::Animation(value) => value.label(),
		}
	}
}

#[derive(Component, Clone, Copy, Debug)]
pub enum CreatorUiAction {
	ToggleSection(UiSection),
	Head(BrodlerHeadMesh),
	Horns(HornMesh),
	Hair(HairMesh),
	Animation(ConceptAnimation),
	ToggleClothing(ClothingMesh),
	SetSkin(BrodlerSkinColor),
	SetEyes(BrodlerEyeColor),
	SetHairColor(BraidmanColor),
	SetClothingColor(ClothingMesh, BraidmanColor),
}

impl CreatorUiAction {
	pub fn focus_target(self) -> Option<UiAssetTarget> {
		match self {
			Self::Head(value) => Some(UiAssetTarget::Head(value)),
			Self::Horns(value) => Some(UiAssetTarget::Horns(value)),
			Self::Hair(value) => Some(UiAssetTarget::Hair(value)),
			Self::ToggleClothing(value) => Some(UiAssetTarget::Clothing(value)),
			Self::Animation(_) => None,
			Self::ToggleSection(_)
			| Self::SetSkin(_)
			| Self::SetEyes(_)
			| Self::SetHairColor(_)
			| Self::SetClothingColor(_, _) => None,
		}
	}
}

#[derive(Clone, Copy)]
struct PanelContext<'a> {
	brodler: &'a BrodlerConfig,
	animation: ConceptAnimation,
	ui_state: &'a CreatorUiState,
}

pub fn populate_panel(
	parent: &mut ChildSpawnerCommands,
	asset_server: &AssetServer,
	images: &mut Assets<Image>,
	thumbnails: &mut ThumbnailCache,
	config: &ConceptPreviewConfig,
	ui_state: &CreatorUiState,
) {
	let ConceptPreviewConfig::Brodler { config: brodler, animation } = config else {
		return;
	};
	let ctx = PanelContext { brodler, animation: *animation, ui_state };

	section(
		parent,
		UiSection::Head,
		ui_state,
		ShellAction::Brodler(CreatorUiAction::ToggleSection(UiSection::Head)),
		|section| {
		asset_grid(
			section,
			asset_server,
			images,
			thumbnails,
			HEADS.iter().copied().map(UiAssetTarget::Head),
			ctx,
		);
	},
	);

	section(
		parent,
		UiSection::HeadFeatures,
		ui_state,
		ShellAction::Brodler(CreatorUiAction::ToggleSection(UiSection::HeadFeatures)),
		|section| {
		text(section, "Horns", 11.0, Color::srgb(0.82, 0.86, 0.92));
		asset_grid(
			section,
			asset_server,
			images,
			thumbnails,
			HORNS.iter().copied().map(UiAssetTarget::Horns),
			ctx,
		);
		text(section, "Colors", 11.0, Color::srgb(0.82, 0.86, 0.92));
		text(section, "Skin", 10.0, Color::srgb(0.72, 0.76, 0.82));
		skin_color_swatches(section, brodler.colors.skin);
		text(section, "Eyes", 10.0, Color::srgb(0.72, 0.76, 0.82));
		eye_color_swatches(section, brodler.colors.eyes);
		text(section, "Hair color", 10.0, Color::srgb(0.72, 0.76, 0.82));
		hair_color_swatches(section, brodler.colors.hair);
	},
	);

	section(
		parent,
		UiSection::Hair,
		ui_state,
		ShellAction::Brodler(CreatorUiAction::ToggleSection(UiSection::Hair)),
		|section| {
		asset_grid(
			section,
			asset_server,
			images,
			thumbnails,
			HAIRS.iter().copied().map(UiAssetTarget::Hair),
			ctx,
		);
	},
	);

	section(
		parent,
		UiSection::Clothing,
		ui_state,
		ShellAction::Brodler(CreatorUiAction::ToggleSection(UiSection::Clothing)),
		|section| {
		clothing_list(section, asset_server, images, thumbnails, ctx);
	},
	);

	section(
		parent,
		UiSection::Animation,
		ui_state,
		ShellAction::Brodler(CreatorUiAction::ToggleSection(UiSection::Animation)),
		|section| {
		asset_grid(
			section,
			asset_server,
			images,
			thumbnails,
			ANIMATIONS.iter().copied().map(UiAssetTarget::Animation),
			ctx,
		);
	},
	);
}

pub fn apply_action(
	action: CreatorUiAction,
	brodler: &mut BrodlerConfig,
	animation: &mut ConceptAnimation,
	ui_state: &mut CreatorUiState,
) {
	match action {
		CreatorUiAction::ToggleSection(section) => ui_state.toggle(section),
		CreatorUiAction::Head(value) => brodler.head = value,
		CreatorUiAction::Horns(value) => brodler.horns = value,
		CreatorUiAction::Hair(value) => brodler.hair = value,
		CreatorUiAction::Animation(value) => *animation = value,
		CreatorUiAction::ToggleClothing(value) => toggle_clothing(&mut brodler.clothing, value),
		CreatorUiAction::SetSkin(color) => brodler.colors.skin = color,
		CreatorUiAction::SetEyes(color) => brodler.colors.eyes = color,
		CreatorUiAction::SetHairColor(color) => brodler.colors.hair = color,
		CreatorUiAction::SetClothingColor(clothing, color) => {
			brodler.colors.set_clothing_color(clothing, color);
		}
	}
}

pub fn selection_button_color(
	action: CreatorUiAction,
	brodler: &BrodlerConfig,
	animation: ConceptAnimation,
	ui_state: &CreatorUiState,
) -> Option<Color> {
	const INACTIVE: Color = Color::srgba(0.18, 0.20, 0.24, 0.92);
	const ACTIVE: Color = Color::srgba(0.16, 0.34, 0.50, 0.95);

	match action {
		CreatorUiAction::ToggleSection(section) => {
			Some(if ui_state.is_open(section) { ACTIVE } else { INACTIVE })
		}
		CreatorUiAction::Head(value) => Some(asset_button_color(
			UiAssetTarget::Head(value),
			brodler,
			animation,
			ui_state,
		)),
		CreatorUiAction::Horns(value) => Some(asset_button_color(
			UiAssetTarget::Horns(value),
			brodler,
			animation,
			ui_state,
		)),
		CreatorUiAction::Hair(value) => Some(asset_button_color(
			UiAssetTarget::Hair(value),
			brodler,
			animation,
			ui_state,
		)),
		CreatorUiAction::Animation(value) => Some(asset_button_color(
			UiAssetTarget::Animation(value),
			brodler,
			animation,
			ui_state,
		)),
		CreatorUiAction::ToggleClothing(value) => Some(asset_button_color(
			UiAssetTarget::Clothing(value),
			brodler,
			animation,
			ui_state,
		)),
		CreatorUiAction::SetSkin(color) => {
			Some(if brodler.colors.skin == color { ACTIVE } else { INACTIVE })
		}
		CreatorUiAction::SetEyes(color) => {
			Some(if brodler.colors.eyes == color { ACTIVE } else { INACTIVE })
		}
		CreatorUiAction::SetHairColor(color) => {
			Some(if brodler.colors.hair == color { ACTIVE } else { INACTIVE })
		}
		CreatorUiAction::SetClothingColor(clothing, color) => Some(
			if brodler.colors.clothing_color(clothing) == color {
				ACTIVE
			} else {
				INACTIVE
			},
		),
	}
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
				let shell = wrap(target);
				let active = target_active(target, ctx.brodler, ctx.animation);
				let focus = ctx.ui_state.focused_target() == Some(shell);
				let color = target_color(target, ctx.brodler);
				let camera = target_path(target).map(|path| {
					let mut commands = grid.commands();
					thumbnail::camera_for_target(
						&mut commands,
						images,
						asset_server,
						thumbnails,
						shell,
						path,
						color,
					)
				});
				asset_button(grid, target, active, focus, camera);
			}
		});
}

fn clothing_list(
	parent: &mut ChildSpawnerCommands,
	asset_server: &AssetServer,
	images: &mut Assets<Image>,
	thumbnails: &mut ThumbnailCache,
	ctx: PanelContext,
) {
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(6.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|list| {
			for clothing in CLOTHING {
				let target = UiAssetTarget::Clothing(*clothing);
				let shell = wrap(target);
				let active = ctx.brodler.clothing.contains(clothing);
				let focus = ctx.ui_state.focused_target() == Some(shell);
				let color = PreviewColor::Braidman(ctx.brodler.colors.clothing_color(*clothing));
				let camera = thumbnail::camera_for_target(
					&mut list.commands(),
					images,
					asset_server,
					thumbnails,
					shell,
					clothing.path().as_str(),
					color,
				);
				list.spawn((
					Node {
						width: Val::Percent(100.0),
						flex_direction: FlexDirection::Row,
						column_gap: Val::Px(6.0),
						align_items: AlignItems::Center,
						..default()
					},
					Pickable::IGNORE,
				))
				.with_children(|item| {
					asset_button(item, target, active, focus, Some(camera));
					inline_clothing_color_swatches(
						item,
						*clothing,
						ctx.brodler.colors.clothing_color(*clothing),
					);
				});
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
			ShellAction::Brodler(target_action(target)),
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

fn skin_color_swatches(parent: &mut ChildSpawnerCommands, active: BrodlerSkinColor) {
	color_row(parent, |row| {
		for color in SKIN_COLORS {
			let selected = *color == active;
			row.spawn((
				Button,
				Node { width: Val::Px(22.0), height: Val::Px(22.0), ..default() },
				BackgroundColor(color.color()),
				BorderColor::all(if selected { Color::WHITE } else { Color::srgba(0.3, 0.3, 0.3, 0.8) }),
				ShellAction::Brodler(CreatorUiAction::SetSkin(*color)),
			));
		}
	});
}

fn eye_color_swatches(parent: &mut ChildSpawnerCommands, active: BrodlerEyeColor) {
	color_row(parent, |row| {
		for color in EYE_COLORS {
			let selected = *color == active;
			row.spawn((
				Button,
				Node { width: Val::Px(22.0), height: Val::Px(22.0), ..default() },
				BackgroundColor(color.color()),
				BorderColor::all(if selected { Color::WHITE } else { Color::srgba(0.3, 0.3, 0.3, 0.8) }),
				ShellAction::Brodler(CreatorUiAction::SetEyes(*color)),
			));
		}
	});
}

fn hair_color_swatches(parent: &mut ChildSpawnerCommands, active: BraidmanColor) {
	color_row(parent, |row| {
		for color in HAIR_COLORS {
			let selected = *color == active;
			row.spawn((
				Button,
				Node { width: Val::Px(22.0), height: Val::Px(22.0), ..default() },
				BackgroundColor(color.color()),
				BorderColor::all(if selected { Color::WHITE } else { Color::srgba(0.3, 0.3, 0.3, 0.8) }),
				ShellAction::Brodler(CreatorUiAction::SetHairColor(*color)),
			));
		}
	});
}

fn color_row(parent: &mut ChildSpawnerCommands, build: impl FnOnce(&mut ChildSpawnerCommands)) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				flex_wrap: FlexWrap::Wrap,
				column_gap: Val::Px(4.0),
				row_gap: Val::Px(4.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(build);
}

fn inline_clothing_color_swatches(
	parent: &mut ChildSpawnerCommands,
	clothing: ClothingMesh,
	active: BraidmanColor,
) {
	color_row(parent, |row| {
		for color in HAIR_COLORS {
			let selected = *color == active;
			row.spawn((
				Button,
				Node { width: Val::Px(20.0), height: Val::Px(16.0), ..default() },
				BackgroundColor(color.color()),
				BorderColor::all(if selected { Color::WHITE } else { Color::srgba(0.3, 0.3, 0.3, 0.8) }),
				ShellAction::Brodler(CreatorUiAction::SetClothingColor(clothing, *color)),
			));
		}
	});
}

fn wrap(target: UiAssetTarget) -> ShellTarget {
	ShellTarget::Brodler(target)
}

fn target_active(target: UiAssetTarget, brodler: &BrodlerConfig, animation: ConceptAnimation) -> bool {
	match target {
		UiAssetTarget::Body => true,
		UiAssetTarget::Head(value) => brodler.head == value,
		UiAssetTarget::Horns(value) => brodler.horns == value,
		UiAssetTarget::Eye => true,
		UiAssetTarget::Hair(value) => brodler.hair == value,
		UiAssetTarget::Clothing(value) => brodler.clothing.contains(&value),
		UiAssetTarget::Animation(value) => animation == value,
	}
}

fn target_action(target: UiAssetTarget) -> CreatorUiAction {
	match target {
		UiAssetTarget::Head(value) => CreatorUiAction::Head(value),
		UiAssetTarget::Horns(value) => CreatorUiAction::Horns(value),
		UiAssetTarget::Hair(value) => CreatorUiAction::Hair(value),
		UiAssetTarget::Clothing(value) => CreatorUiAction::ToggleClothing(value),
		UiAssetTarget::Animation(value) => CreatorUiAction::Animation(value),
		UiAssetTarget::Body | UiAssetTarget::Eye => CreatorUiAction::Head(BrodlerHeadMesh::Gaunt),
	}
}

fn target_path(target: UiAssetTarget) -> Option<&'static str> {
	match target {
		UiAssetTarget::Body => Some(crozon_characters::species::common::BODY_STANDARD.as_str()),
		UiAssetTarget::Head(value) => Some(value.path().as_str()),
		UiAssetTarget::Horns(value) => Some(value.path().as_str()),
		UiAssetTarget::Eye => Some(crozon_characters::species::common::EYE_STANDARD.as_str()),
		UiAssetTarget::Hair(value) => value.path().map(|path| path.as_str()),
		UiAssetTarget::Clothing(value) => Some(value.path().as_str()),
		UiAssetTarget::Animation(_) => None,
	}
}

fn target_color(target: UiAssetTarget, brodler: &BrodlerConfig) -> PreviewColor {
	match target {
		UiAssetTarget::Body | UiAssetTarget::Head(_) | UiAssetTarget::Horns(_) => {
			PreviewColor::BrodlerSkin(brodler.colors.skin)
		}
		UiAssetTarget::Eye => PreviewColor::BrodlerEye(brodler.colors.eyes),
		UiAssetTarget::Hair(_) => PreviewColor::Braidman(brodler.colors.hair),
		UiAssetTarget::Clothing(value) => {
			PreviewColor::Braidman(brodler.colors.clothing_color(value))
		}
		UiAssetTarget::Animation(_) => PreviewColor::BrodlerSkin(brodler.colors.skin),
	}
}

fn asset_button_color(
	target: UiAssetTarget,
	brodler: &BrodlerConfig,
	animation: ConceptAnimation,
	ui_state: &CreatorUiState,
) -> Color {
	const INACTIVE: Color = Color::srgba(0.18, 0.20, 0.24, 0.92);
	const ACTIVE: Color = Color::srgba(0.16, 0.34, 0.50, 0.95);
	const FOCUS: Color = Color::srgba(0.30, 0.38, 0.48, 0.98);
	if ui_state.focused_target() == Some(wrap(target)) {
		return FOCUS;
	}
	if target_active(target, brodler, animation) {
		ACTIVE
	} else {
		INACTIVE
	}
}

fn toggle_clothing(clothing: &mut Vec<ClothingMesh>, value: ClothingMesh) {
	if let Some(index) = clothing.iter().position(|item| *item == value) {
		clothing.remove(index);
	} else {
		clothing.push(value);
	}
}

pub fn default_focus_target() -> ShellTarget {
	wrap(UiAssetTarget::Body)
}
