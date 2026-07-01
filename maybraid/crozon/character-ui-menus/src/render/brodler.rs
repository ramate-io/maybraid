use bevy::prelude::*;
use bevy_character_ui_menu_renderer::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};

use crate::{
	characters::brodler::{
		BrodlerClothingMenu, BrodlerHairMenu, BrodlerHeadFeaturesMenu, BrodlerHeadMenu, BrodlerMenu,
	},
	event::CharacterField,
	fields::ColoredMultiSelectField,
	render::{asset_field, swatch_field},
};

impl RenderMenu for BrodlerMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.base_preview_color = self.head.value.skin.value.color();
		context.accent_preview_color = self.head_features.value.horn_color.value.color();
		self.head.render_with(renderer, parent, context);
		self.head_features.render_with(renderer, parent, context);
		self.hair.render_with(renderer, parent, context);
		self.clothing.render_with(renderer, parent, context);
		self.animation.render_with(renderer, parent, context);
	}
}

impl RenderMenu for BrodlerHeadMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.preview_color = self.skin.value.color();
		asset_field("Head", CharacterField::BrodlerHead, self.head).render_with(renderer, parent, context);
		context.preview_color = context.accent_preview_color;
		asset_field("Horns", CharacterField::Horns, self.horns).render_with(renderer, parent, context);
		swatch_field("Skin", CharacterField::SkinColor, self.skin).render_with(renderer, parent, context);
	}
}

impl RenderMenu for BrodlerHeadFeaturesMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.preview_color = self.eye_color.value.color();
		asset_field("Eyes", CharacterField::Eye, self.eye).render_with(renderer, parent, context);
		swatch_field("Eye Color", CharacterField::BrodlerEyeColor, self.eye_color)
			.render_with(renderer, parent, context);
		swatch_field("Horn Color", CharacterField::HornColor, self.horn_color)
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

impl RenderMenu for BrodlerHairMenu {
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

impl RenderMenu for BrodlerClothingMenu {
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
