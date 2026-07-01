use bevy::prelude::*;
use character_ui_menu::ThumbnailCamera;
use crozon_character_ui_menus::{
	AssetOption, AssetValue, BraidmanMenu, BrodlerMenu, CharacterField, CharacterMenu,
	ConceptSpecies, LabelOption, ListValues, MenuEvent, SectionId, SectionOpenState, SwatchOption,
	SwatchValue,
};

const BUTTON_HEIGHT: f32 = 22.0;
const ACTIVE: Color = Color::srgba(0.16, 0.34, 0.50, 0.95);
const INACTIVE: Color = Color::srgba(0.18, 0.20, 0.24, 0.92);
const MUTED: Color = Color::srgba(0.72, 0.78, 0.86, 1.0);

#[derive(Default)]
pub struct Renderer;

pub struct RenderContext<'a, T> {
	pub sections: SectionOpenState,
	pub thumbnails: &'a mut T,
}

pub trait RenderMenu {
	fn render_with<T: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, T>,
	);
}

impl Renderer {
	pub fn render<M, T>(
		&self,
		parent: &mut ChildSpawnerCommands,
		menu: &M,
		context: &mut RenderContext<'_, T>,
	) where
		M: RenderMenu,
		T: MenuThumbnailContext,
	{
		menu.render_with(self, parent, context);
	}

	fn render_braidman<T: MenuThumbnailContext>(
		&self,
		parent: &mut ChildSpawnerCommands,
		menu: &BraidmanMenu,
		context: &mut RenderContext<'_, T>,
	) {
		render_section(
			parent,
			SectionId::Presets,
			context.sections.is_open(SectionId::Presets),
			|body| {
				render_single_cycle(
					body,
					"Gender",
					menu.presets.value.gender.value.label(),
					CharacterField::Gender,
				);
				render_single_cycle(
					body,
					"Build",
					menu.presets.value.build.value.label(),
					CharacterField::Build,
				);
			},
		);
		render_section(
			parent,
			SectionId::Body,
			context.sections.is_open(SectionId::Body),
			|body| {
				render_asset_select(
					body,
					"Body Mesh",
					menu.body.value.body.value,
					CharacterField::BodyMesh,
					AssetValue::Body,
					|_| menu.body.value.color.value.color(),
					context.thumbnails,
				);
				render_braidman_body_sliders(body, menu);
				render_swatch_select(body, "Body Color", menu.body.value.color.value, |color| {
					MenuEvent::SetSwatch(CharacterField::BodyColor, SwatchValue::Braidman(color))
				});
			},
		);
		render_section(
			parent,
			SectionId::HeadFeatures,
			context.sections.is_open(SectionId::HeadFeatures),
			|body| {
				render_asset_select(
					body,
					"Head",
					menu.head_features.value.head.value,
					CharacterField::HeadMesh,
					AssetValue::Head,
					|_| menu.body.value.color.value.color(),
					context.thumbnails,
				);
				render_asset_select(
					body,
					"Eyes",
					menu.head_features.value.eye.value,
					CharacterField::Eye,
					AssetValue::Eye,
					|_| menu.head_features.value.eye_color.value.color(),
					context.thumbnails,
				);
				render_swatch_select(
					body,
					"Eye Color",
					menu.head_features.value.eye_color.value,
					|color| {
						MenuEvent::SetSwatch(CharacterField::EyeColor, SwatchValue::Braidman(color))
					},
				);
				render_asset_select(
					body,
					"Nose",
					menu.head_features.value.nose.value,
					CharacterField::Nose,
					AssetValue::Nose,
					|_| menu.body.value.color.value.color(),
					context.thumbnails,
				);
				render_asset_select(
					body,
					"Mouth",
					menu.head_features.value.mouth.value,
					CharacterField::Mouth,
					AssetValue::Mouth,
					|_| menu.head_features.value.mouth_color.value.color(),
					context.thumbnails,
				);
				render_swatch_select(
					body,
					"Mouth Color",
					menu.head_features.value.mouth_color.value,
					|color| {
						MenuEvent::SetSwatch(
							CharacterField::MouthColor,
							SwatchValue::Braidman(color),
						)
					},
				);
				render_asset_select(
					body,
					"Ears",
					menu.head_features.value.ear.value,
					CharacterField::Ear,
					AssetValue::Ear,
					|_| menu.body.value.color.value.color(),
					context.thumbnails,
				);
			},
		);
		render_section(
			parent,
			SectionId::Hair,
			context.sections.is_open(SectionId::Hair),
			|body| {
				render_asset_select(
					body,
					"Hair",
					menu.hair.value.style.value,
					CharacterField::Hair,
					AssetValue::Hair,
					|_| menu.hair.value.color.value.color(),
					context.thumbnails,
				);
				render_swatch_select(body, "Hair Color", menu.hair.value.color.value, |color| {
					MenuEvent::SetSwatch(CharacterField::HairColor, SwatchValue::Braidman(color))
				});
			},
		);
		render_section(
			parent,
			SectionId::Clothing,
			context.sections.is_open(SectionId::Clothing),
			|body| {
				render_colored_multi_select(
					body,
					"Clothing",
					&menu.clothing.value.layers.selected,
					|clothing| menu.clothing_color(clothing),
					MenuEvent::ToggleClothing,
					|clothing, color| {
						MenuEvent::SetSwatch(
							CharacterField::Clothing(clothing),
							SwatchValue::Braidman(color),
						)
					},
					|clothing| menu.clothing_color(clothing).color(),
					context.thumbnails,
				);
			},
		);
		render_section(
			parent,
			SectionId::Animation,
			context.sections.is_open(SectionId::Animation),
			|body| {
				render_asset_select(
					body,
					"Animation",
					menu.animation.value.clip.value,
					CharacterField::Animation,
					AssetValue::Animation,
					|_| Color::WHITE,
					context.thumbnails,
				);
			},
		);
	}

	fn render_brodler<T: MenuThumbnailContext>(
		&self,
		parent: &mut ChildSpawnerCommands,
		menu: &BrodlerMenu,
		context: &mut RenderContext<'_, T>,
	) {
		render_section(
			parent,
			SectionId::Head,
			context.sections.is_open(SectionId::Head),
			|body| {
				render_asset_select(
					body,
					"Head",
					menu.head.value.head.value,
					CharacterField::BrodlerHead,
					AssetValue::BrodlerHead,
					|_| menu.head.value.skin.value.color(),
					context.thumbnails,
				);
				render_asset_select(
					body,
					"Horns",
					menu.head.value.horns.value,
					CharacterField::Horns,
					AssetValue::Horns,
					|_| menu.head_features.value.horn_color.value.color(),
					context.thumbnails,
				);
				render_swatch_select(body, "Skin", menu.head.value.skin.value, |color| {
					MenuEvent::SetSwatch(CharacterField::SkinColor, SwatchValue::BrodlerSkin(color))
				});
			},
		);
		render_section(
			parent,
			SectionId::HeadFeatures,
			context.sections.is_open(SectionId::HeadFeatures),
			|body| {
				render_asset_select(
					body,
					"Eyes",
					menu.head_features.value.eye.value,
					CharacterField::Eye,
					AssetValue::Eye,
					|_| menu.head_features.value.eye_color.value.color(),
					context.thumbnails,
				);
				render_swatch_select(
					body,
					"Eye Color",
					menu.head_features.value.eye_color.value,
					|color| {
						MenuEvent::SetSwatch(
							CharacterField::BrodlerEyeColor,
							SwatchValue::BrodlerEye(color),
						)
					},
				);
				render_swatch_select(
					body,
					"Horn Color",
					menu.head_features.value.horn_color.value,
					|color| {
						MenuEvent::SetSwatch(
							CharacterField::HornColor,
							SwatchValue::BrodlerHorn(color),
						)
					},
				);
				render_asset_select(
					body,
					"Nose",
					menu.head_features.value.nose.value,
					CharacterField::Nose,
					AssetValue::Nose,
					|_| menu.head.value.skin.value.color(),
					context.thumbnails,
				);
				render_asset_select(
					body,
					"Mouth",
					menu.head_features.value.mouth.value,
					CharacterField::Mouth,
					AssetValue::Mouth,
					|_| menu.head_features.value.mouth_color.value.color(),
					context.thumbnails,
				);
				render_swatch_select(
					body,
					"Mouth Color",
					menu.head_features.value.mouth_color.value,
					|color| {
						MenuEvent::SetSwatch(
							CharacterField::MouthColor,
							SwatchValue::Braidman(color),
						)
					},
				);
				render_asset_select(
					body,
					"Ears",
					menu.head_features.value.ear.value,
					CharacterField::Ear,
					AssetValue::Ear,
					|_| menu.head.value.skin.value.color(),
					context.thumbnails,
				);
			},
		);
		render_section(
			parent,
			SectionId::Hair,
			context.sections.is_open(SectionId::Hair),
			|body| {
				render_asset_select(
					body,
					"Hair",
					menu.hair.value.style.value,
					CharacterField::Hair,
					AssetValue::Hair,
					|_| menu.hair.value.color.value.color(),
					context.thumbnails,
				);
				render_swatch_select(body, "Hair Color", menu.hair.value.color.value, |color| {
					MenuEvent::SetSwatch(CharacterField::HairColor, SwatchValue::Braidman(color))
				});
			},
		);
		render_section(
			parent,
			SectionId::Clothing,
			context.sections.is_open(SectionId::Clothing),
			|body| {
				render_colored_multi_select(
					body,
					"Clothing",
					&menu.clothing.value.layers.selected,
					|clothing| menu.clothing_color(clothing),
					MenuEvent::ToggleClothing,
					|clothing, color| {
						MenuEvent::SetSwatch(
							CharacterField::Clothing(clothing),
							SwatchValue::Braidman(color),
						)
					},
					|clothing| menu.clothing_color(clothing).color(),
					context.thumbnails,
				);
			},
		);
		render_section(
			parent,
			SectionId::Animation,
			context.sections.is_open(SectionId::Animation),
			|body| {
				render_asset_select(
					body,
					"Animation",
					menu.animation.value.clip.value,
					CharacterField::Animation,
					AssetValue::Animation,
					|_| Color::WHITE,
					context.thumbnails,
				);
			},
		);
	}
}

impl RenderMenu for CharacterMenu {
	fn render_with<T: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, T>,
	) {
		match self.species.value {
			ConceptSpecies::Braidman => self.braidman.render_with(renderer, parent, context),
			ConceptSpecies::Brodler => self.brodler.render_with(renderer, parent, context),
		}
	}
}

impl RenderMenu for BraidmanMenu {
	fn render_with<T: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, T>,
	) {
		renderer.render_braidman(parent, self, context);
	}
}

impl RenderMenu for BrodlerMenu {
	fn render_with<T: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, T>,
	) {
		renderer.render_brodler(parent, self, context);
	}
}

#[derive(Component, Clone, Copy, Debug)]
pub struct MenuButton(pub MenuEvent);

/// Renderer-owned thumbnail bridge. The playground can adapt this to its cache.
pub trait MenuThumbnailContext {
	fn image_for_asset(
		&mut self,
		label: &'static str,
		asset_path: &'static str,
		color: Color,
		camera: ThumbnailCamera,
	) -> Option<Handle<Image>>;
}

pub fn render_section(
	parent: &mut ChildSpawnerCommands,
	section: SectionId,
	open: bool,
	body: impl FnOnce(&mut ChildSpawnerCommands),
) {
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
			render_button(
				section_parent,
				&format!("{} {}", if open { "v" } else { ">" }, section.label()),
				MenuEvent::ToggleSection(section),
				open,
			);
			if open {
				body(section_parent);
			}
		});
}

pub fn render_single_cycle(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	value_label: &'static str,
	field: CharacterField,
) {
	parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, label, 11.0, Color::WHITE);
		render_button(row, "<", MenuEvent::Cycle(field, -1), false);
		text(row, value_label, 11.0, Color::srgb(0.85, 0.95, 1.0));
		render_button(row, ">", MenuEvent::Cycle(field, 1), false);
	});
}

pub fn render_slider(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	value: f32,
	step: f32,
	field: CharacterField,
) {
	parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, label, 11.0, Color::WHITE);
		render_button(row, "-", MenuEvent::SliderDelta(field, -step), false);
		text(row, &format!("{value:.2}"), 11.0, Color::srgb(0.85, 0.95, 1.0));
		render_button(row, "+", MenuEvent::SliderDelta(field, step), false);
	});
}

pub fn render_swatch_select<T>(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	active: T,
	to_event: impl Fn(T) -> MenuEvent,
) where
	T: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
{
	parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, label, 11.0, Color::WHITE);
		for value in T::values() {
			let active = *value == active;
			row.spawn((
				Button,
				Node {
					width: Val::Px(22.0),
					height: Val::Px(18.0),
					border: UiRect::all(Val::Px(if active { 2.0 } else { 1.0 })),
					..default()
				},
				BorderColor::all(if active { Color::WHITE } else { MUTED }),
				BackgroundColor(color_from_hex(value.color_hex())),
				MenuButton(to_event(*value)),
			));
		}
	});
}

pub fn render_asset_select<T>(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	active: T,
	field: CharacterField,
	to_value: impl Fn(T) -> AssetValue,
	preview_color: impl Fn(T) -> Color,
	thumbnails: &mut impl MenuThumbnailContext,
) where
	T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
{
	text(parent, label, 12.0, Color::srgb(0.78, 0.84, 0.92));
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
			for value in T::values() {
				let asset = value.asset();
				let thumbnail = thumbnails.image_for_asset(
					asset.label,
					asset.path,
					preview_color(*value),
					asset.thumbnail_camera,
				);
				render_asset_button(
					grid,
					value.label(),
					MenuEvent::SetAsset(field, to_value(*value)),
					*value == active,
					thumbnail,
				);
			}
		});
}

pub fn render_multi_select<T>(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	selected: &[T],
	to_event: impl Fn(T) -> MenuEvent,
	preview_color: impl Fn(T) -> Color,
	thumbnails: &mut impl MenuThumbnailContext,
) where
	T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
{
	text(parent, label, 12.0, Color::srgb(0.78, 0.84, 0.92));
	for value in T::values() {
		let asset = value.asset();
		let thumbnail = thumbnails.image_for_asset(
			asset.label,
			asset.path,
			preview_color(*value),
			asset.thumbnail_camera,
		);
		let active = selected.iter().any(|selected| *selected == *value);
		render_asset_button(parent, value.label(), to_event(*value), active, thumbnail);
	}
}

pub fn render_colored_multi_select<T, C>(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	selected: &[T],
	active_color: impl Fn(T) -> C,
	to_toggle_event: impl Fn(T) -> MenuEvent,
	to_color_event: impl Fn(T, C) -> MenuEvent,
	preview_color: impl Fn(T) -> Color,
	thumbnails: &mut impl MenuThumbnailContext,
) where
	T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
	C: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
{
	text(parent, label, 12.0, Color::srgb(0.78, 0.84, 0.92));
	for value in T::values() {
		let asset = value.asset();
		let thumbnail = thumbnails.image_for_asset(
			asset.label,
			asset.path,
			preview_color(*value),
			asset.thumbnail_camera,
		);
		let active = selected.iter().any(|selected| *selected == *value);
		parent
			.spawn((
				Node {
					width: Val::Percent(100.0),
					flex_direction: FlexDirection::Row,
					column_gap: Val::Px(6.0),
					row_gap: Val::Px(4.0),
					align_items: AlignItems::Center,
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|row| {
				render_asset_button(row, value.label(), to_toggle_event(*value), active, thumbnail);
				render_inline_swatches(row, active_color(*value), |color| {
					to_color_event(*value, color)
				});
			});
	}
}

fn render_braidman_body_sliders(parent: &mut ChildSpawnerCommands, menu: &BraidmanMenu) {
	let sliders = &menu.body.value.sliders;
	render_slider(
		parent,
		"Shoulder Width",
		sliders.shoulder_width.value,
		sliders.shoulder_width.step,
		CharacterField::ShoulderWidth,
	);
	render_slider(
		parent,
		"Hip Width",
		sliders.hip_width.value,
		sliders.hip_width.step,
		CharacterField::HipWidth,
	);
	render_slider(
		parent,
		"Chest Thickness",
		sliders.chest_thickness.value,
		sliders.chest_thickness.step,
		CharacterField::ChestThickness,
	);
	render_slider(
		parent,
		"Hip Thickness",
		sliders.hip_thickness.value,
		sliders.hip_thickness.step,
		CharacterField::HipThickness,
	);
	render_slider(
		parent,
		"Leg Thickness",
		sliders.leg_thickness.value,
		sliders.leg_thickness.step,
		CharacterField::LegThickness,
	);
	render_slider(
		parent,
		"Buttocks Thickness",
		sliders.buttocks_thickness.value,
		sliders.buttocks_thickness.step,
		CharacterField::ButtocksThickness,
	);
	render_slider(
		parent,
		"Waist Thickness",
		sliders.waist_thickness.value,
		sliders.waist_thickness.step,
		CharacterField::WaistThickness,
	);
	render_slider(
		parent,
		"Lower Trunk Thickness",
		sliders.lower_trunk_thickness.value,
		sliders.lower_trunk_thickness.step,
		CharacterField::LowerTrunkThickness,
	);
	render_slider(
		parent,
		"Arm Length",
		sliders.arm_length.value,
		sliders.arm_length.step,
		CharacterField::ArmLength,
	);
	render_slider(
		parent,
		"Arm Thickness",
		sliders.arm_thickness.value,
		sliders.arm_thickness.step,
		CharacterField::ArmThickness,
	);
	render_slider(
		parent,
		"Leg Length",
		sliders.leg_length.value,
		sliders.leg_length.step,
		CharacterField::LegLength,
	);
	render_slider(
		parent,
		"Eye Width",
		sliders.eye_width.value,
		sliders.eye_width.step,
		CharacterField::EyeWidth,
	);
	render_slider(
		parent,
		"Eye Height",
		sliders.eye_height.value,
		sliders.eye_height.step,
		CharacterField::EyeHeight,
	);
	render_slider(
		parent,
		"Eye Tilt",
		sliders.eye_tilt.value,
		sliders.eye_tilt.step,
		CharacterField::EyeTilt,
	);
	render_slider(
		parent,
		"Nose Width",
		sliders.nose_width.value,
		sliders.nose_width.step,
		CharacterField::NoseWidth,
	);
	render_slider(
		parent,
		"Nose Height",
		sliders.nose_height.value,
		sliders.nose_height.step,
		CharacterField::NoseHeight,
	);
	render_slider(
		parent,
		"Mouth Width",
		sliders.mouth_width.value,
		sliders.mouth_width.step,
		CharacterField::MouthWidth,
	);
	render_slider(
		parent,
		"Mouth Height",
		sliders.mouth_height.value,
		sliders.mouth_height.step,
		CharacterField::MouthHeight,
	);
	render_slider(
		parent,
		"Ear Width",
		sliders.ear_width.value,
		sliders.ear_width.step,
		CharacterField::EarWidth,
	);
	render_slider(
		parent,
		"Ear Height",
		sliders.ear_height.value,
		sliders.ear_height.step,
		CharacterField::EarHeight,
	);
}

fn render_button(parent: &mut ChildSpawnerCommands, label: &str, event: MenuEvent, active: bool) {
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
			BackgroundColor(if active { ACTIVE } else { INACTIVE }),
			MenuButton(event),
		))
		.with_children(|button| text(button, label, 10.0, Color::WHITE));
}

fn render_asset_button(
	parent: &mut ChildSpawnerCommands,
	label: &str,
	event: MenuEvent,
	active: bool,
	thumbnail: Option<Handle<Image>>,
) {
	parent
		.spawn((
			Button,
			Node {
				min_width: Val::Px(72.0),
				min_height: Val::Px(54.0),
				padding: UiRect::axes(Val::Px(5.0), Val::Px(4.0)),
				flex_direction: FlexDirection::Column,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				row_gap: Val::Px(3.0),
				..default()
			},
			BackgroundColor(if active { ACTIVE } else { INACTIVE }),
			MenuButton(event),
		))
		.with_children(|button| {
			if let Some(thumbnail) = thumbnail {
				button.spawn((
					ImageNode::new(thumbnail),
					Node { width: Val::Px(54.0), height: Val::Px(54.0), ..default() },
					Pickable::IGNORE,
				));
			}
			text(button, label, 9.0, Color::WHITE);
		});
}

fn render_inline_swatches<T>(
	parent: &mut ChildSpawnerCommands,
	active: T,
	to_event: impl Fn(T) -> MenuEvent,
) where
	T: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
{
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				flex_wrap: FlexWrap::Wrap,
				column_gap: Val::Px(3.0),
				row_gap: Val::Px(3.0),
				align_items: AlignItems::Center,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			for value in T::values() {
				let selected = *value == active;
				row.spawn((
					Button,
					Node {
						width: Val::Px(20.0),
						height: Val::Px(16.0),
						border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
						..default()
					},
					BorderColor::all(if selected { Color::WHITE } else { MUTED }),
					BackgroundColor(color_from_hex(value.color_hex())),
					MenuButton(to_event(*value)),
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

fn color_from_hex(hex: &str) -> Color {
	let hex = hex.strip_prefix('#').unwrap_or(hex);
	if hex.len() != 6 {
		return INACTIVE;
	}
	let Ok(red) = u8::from_str_radix(&hex[0..2], 16) else {
		return INACTIVE;
	};
	let Ok(green) = u8::from_str_radix(&hex[2..4], 16) else {
		return INACTIVE;
	};
	let Ok(blue) = u8::from_str_radix(&hex[4..6], 16) else {
		return INACTIVE;
	};
	Color::srgb(red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0)
}

#[allow(dead_code)]
fn swatch_event(field: CharacterField, value: SwatchValue) -> MenuEvent {
	MenuEvent::SetSwatch(field, value)
}
