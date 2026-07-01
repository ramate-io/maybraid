use bevy::prelude::*;
use bevy_character_ui_menu_renderer::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};

use crate::{
	characters::braidman::{
		AnimationMenu, BraidmanBodyMenu, BraidmanClothingMenu, BraidmanHairMenu,
		BraidmanHeadFeaturesMenu, BraidmanMenu, BraidmanPresetsMenu, BraidmanSlidersMenu,
	},
	event::CharacterField,
	fields::ColoredMultiSelectField,
	render::{block_asset, labeled_cycle, labeled_slider, labeled_swatch, render_colored_clothing},
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
		labeled_cycle("Gender", CharacterField::Gender, self.gender).render_with(renderer, parent, context);
		labeled_cycle("Build", CharacterField::Build, self.build).render_with(renderer, parent, context);
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
		block_asset("Body Mesh", CharacterField::BodyMesh, self.body).render_with(renderer, parent, context);
		self.sliders.render_with(renderer, parent, context);
		labeled_swatch("Body Color", CharacterField::BodyColor, self.color)
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
		labeled_slider("Shoulder Width", CharacterField::ShoulderWidth, self.shoulder_width)
			.render_with(renderer, parent, context);
		labeled_slider("Hip Width", CharacterField::HipWidth, self.hip_width)
			.render_with(renderer, parent, context);
		labeled_slider("Chest Thickness", CharacterField::ChestThickness, self.chest_thickness)
			.render_with(renderer, parent, context);
		labeled_slider("Hip Thickness", CharacterField::HipThickness, self.hip_thickness)
			.render_with(renderer, parent, context);
		labeled_slider("Leg Thickness", CharacterField::LegThickness, self.leg_thickness)
			.render_with(renderer, parent, context);
		labeled_slider("Buttocks Thickness", CharacterField::ButtocksThickness, self.buttocks_thickness)
			.render_with(renderer, parent, context);
		labeled_slider("Waist Thickness", CharacterField::WaistThickness, self.waist_thickness)
			.render_with(renderer, parent, context);
		labeled_slider(
			"Lower Trunk Thickness",
			CharacterField::LowerTrunkThickness,
			self.lower_trunk_thickness,
		)
		.render_with(renderer, parent, context);
		labeled_slider("Arm Length", CharacterField::ArmLength, self.arm_length)
			.render_with(renderer, parent, context);
		labeled_slider("Arm Thickness", CharacterField::ArmThickness, self.arm_thickness)
			.render_with(renderer, parent, context);
		labeled_slider("Leg Length", CharacterField::LegLength, self.leg_length)
			.render_with(renderer, parent, context);
		labeled_slider("Eye Width", CharacterField::EyeWidth, self.eye_width)
			.render_with(renderer, parent, context);
		labeled_slider("Eye Height", CharacterField::EyeHeight, self.eye_height)
			.render_with(renderer, parent, context);
		labeled_slider("Eye Tilt", CharacterField::EyeTilt, self.eye_tilt)
			.render_with(renderer, parent, context);
		labeled_slider("Nose Width", CharacterField::NoseWidth, self.nose_width)
			.render_with(renderer, parent, context);
		labeled_slider("Nose Height", CharacterField::NoseHeight, self.nose_height)
			.render_with(renderer, parent, context);
		labeled_slider("Mouth Width", CharacterField::MouthWidth, self.mouth_width)
			.render_with(renderer, parent, context);
		labeled_slider("Mouth Height", CharacterField::MouthHeight, self.mouth_height)
			.render_with(renderer, parent, context);
		labeled_slider("Ear Width", CharacterField::EarWidth, self.ear_width)
			.render_with(renderer, parent, context);
		labeled_slider("Ear Height", CharacterField::EarHeight, self.ear_height)
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
		block_asset("Head", CharacterField::HeadMesh, self.head).render_with(renderer, parent, context);
		context.preview_color = self.eye_color.value.color();
		block_asset("Eyes", CharacterField::Eye, self.eye).render_with(renderer, parent, context);
		labeled_swatch("Eye Color", CharacterField::EyeColor, self.eye_color)
			.render_with(renderer, parent, context);
		context.preview_color = context.base_preview_color;
		block_asset("Nose", CharacterField::Nose, self.nose).render_with(renderer, parent, context);
		context.preview_color = self.mouth_color.value.color();
		block_asset("Mouth", CharacterField::Mouth, self.mouth).render_with(renderer, parent, context);
		labeled_swatch("Mouth Color", CharacterField::MouthColor, self.mouth_color)
			.render_with(renderer, parent, context);
		context.preview_color = context.base_preview_color;
		block_asset("Ears", CharacterField::Ear, self.ear).render_with(renderer, parent, context);
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
		block_asset("Hair", CharacterField::Hair, self.style).render_with(renderer, parent, context);
		labeled_swatch("Hair Color", CharacterField::HairColor, self.color)
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
		let field = ColoredMultiSelectField {
			label: "Clothing",
			layers: self.layers.clone(),
			default_color: self.default_color,
			item_colors: self.item_colors.clone(),
		};
		render_colored_clothing(&field, renderer, parent, context);
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
		block_asset("Animation", CharacterField::Animation, self.clip).render_with(renderer, parent, context);
	}
}
