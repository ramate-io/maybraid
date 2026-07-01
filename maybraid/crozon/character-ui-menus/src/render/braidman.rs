use bevy::prelude::*;
use bevy_character_ui_menu_renderer::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};
use crozon_characters::{
	presets::{BuildPreset, GenderPreset},
	species::braidman::BraidmanColor,
	species::common::ClothingMesh,
};

use crate::{
	characters::braidman::{
		AnimationMenu, BraidmanBodyMenu, BraidmanClothingMenu, BraidmanHairMenu,
		BraidmanHeadFeaturesMenu, BraidmanMenu, BraidmanPresetsMenu, BraidmanSlidersMenu,
	},
	event::CharacterField,
	fields::ColoredMultiSelectField,
	render::{asset_field, cycle_field, slider_field, swatch_field},
};

impl RenderMenu for BraidmanMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.base_preview_color = self.body.value.color.value.color();
		self.presets.render_with(renderer, parent, context);
		self.body.render_with(renderer, parent, context);
		self.head_features.render_with(renderer, parent, context);
		self.hair.render_with(renderer, parent, context);
		self.clothing.render_with(renderer, parent, context);
		self.animation.render_with(renderer, parent, context);
	}
}

impl RenderMenu for BraidmanPresetsMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		cycle_field("Gender", CharacterField::Gender, self.gender).render_with(renderer, parent, context);
		cycle_field("Build", CharacterField::Build, self.build).render_with(renderer, parent, context);
	}
}

impl RenderMenu for BraidmanBodyMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.preview_color = self.color.value.color();
		asset_field("Body Mesh", CharacterField::BodyMesh, self.body).render_with(renderer, parent, context);
		self.sliders.render_with(renderer, parent, context);
		swatch_field("Body Color", CharacterField::BodyColor, self.color)
			.render_with(renderer, parent, context);
	}
}

impl RenderMenu for BraidmanSlidersMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		slider_field("Shoulder Width", CharacterField::ShoulderWidth, self.shoulder_width)
			.render_with(renderer, parent, context);
		slider_field("Hip Width", CharacterField::HipWidth, self.hip_width)
			.render_with(renderer, parent, context);
		slider_field("Chest Thickness", CharacterField::ChestThickness, self.chest_thickness)
			.render_with(renderer, parent, context);
		slider_field("Hip Thickness", CharacterField::HipThickness, self.hip_thickness)
			.render_with(renderer, parent, context);
		slider_field("Leg Thickness", CharacterField::LegThickness, self.leg_thickness)
			.render_with(renderer, parent, context);
		slider_field("Buttocks Thickness", CharacterField::ButtocksThickness, self.buttocks_thickness)
			.render_with(renderer, parent, context);
		slider_field("Waist Thickness", CharacterField::WaistThickness, self.waist_thickness)
			.render_with(renderer, parent, context);
		slider_field(
			"Lower Trunk Thickness",
			CharacterField::LowerTrunkThickness,
			self.lower_trunk_thickness,
		)
		.render_with(renderer, parent, context);
		slider_field("Arm Length", CharacterField::ArmLength, self.arm_length)
			.render_with(renderer, parent, context);
		slider_field("Arm Thickness", CharacterField::ArmThickness, self.arm_thickness)
			.render_with(renderer, parent, context);
		slider_field("Leg Length", CharacterField::LegLength, self.leg_length)
			.render_with(renderer, parent, context);
		slider_field("Eye Width", CharacterField::EyeWidth, self.eye_width)
			.render_with(renderer, parent, context);
		slider_field("Eye Height", CharacterField::EyeHeight, self.eye_height)
			.render_with(renderer, parent, context);
		slider_field("Eye Tilt", CharacterField::EyeTilt, self.eye_tilt)
			.render_with(renderer, parent, context);
		slider_field("Nose Width", CharacterField::NoseWidth, self.nose_width)
			.render_with(renderer, parent, context);
		slider_field("Nose Height", CharacterField::NoseHeight, self.nose_height)
			.render_with(renderer, parent, context);
		slider_field("Mouth Width", CharacterField::MouthWidth, self.mouth_width)
			.render_with(renderer, parent, context);
		slider_field("Mouth Height", CharacterField::MouthHeight, self.mouth_height)
			.render_with(renderer, parent, context);
		slider_field("Ear Width", CharacterField::EarWidth, self.ear_width)
			.render_with(renderer, parent, context);
		slider_field("Ear Height", CharacterField::EarHeight, self.ear_height)
			.render_with(renderer, parent, context);
	}
}

impl RenderMenu for BraidmanHeadFeaturesMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.preview_color = context.base_preview_color;
		asset_field("Head", CharacterField::HeadMesh, self.head).render_with(renderer, parent, context);
		context.preview_color = self.eye_color.value.color();
		asset_field("Eyes", CharacterField::Eye, self.eye).render_with(renderer, parent, context);
		swatch_field("Eye Color", CharacterField::EyeColor, self.eye_color)
			.render_with(renderer, parent, context);
		context.preview_color = context.base_preview_color;
		asset_field("Nose", CharacterField::Nose, self.nose).render_with(renderer, parent, context);
		context.preview_color = self.mouth_color.value.color();
		asset_field("Mouth", CharacterField::Mouth, self.mouth).render_with(renderer, parent, context);
		swatch_field("Mouth Color", CharacterField::MouthColor, self.mouth_color)
			.render_with(renderer, parent, context);
		context.preview_color = context.base_preview_color;
		asset_field("Ears", CharacterField::Ear, self.ear).render_with(renderer, parent, context);
	}
}

impl RenderMenu for BraidmanHairMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.preview_color = self.color.value.color();
		asset_field("Hair", CharacterField::Hair, self.style).render_with(renderer, parent, context);
		swatch_field("Hair Color", CharacterField::HairColor, self.color)
			.render_with(renderer, parent, context);
	}
}

impl RenderMenu for BraidmanClothingMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		ColoredMultiSelectField {
			label: "Clothing",
			layers: self.layers.clone(),
			default_color: self.default_color,
			item_colors: self.item_colors.clone(),
		}
		.render_with(renderer, parent, context);
	}
}

impl RenderMenu for AnimationMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.preview_color = Color::WHITE;
		asset_field("Animation", CharacterField::Animation, self.clip)
			.render_with(renderer, parent, context);
	}
}
