use bevy::prelude::*;
use bevy_character_ui_menu_renderer::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};

use crate::{
	characters::lero::{
		LeroBodyMenu, LeroClothingMenu, LeroHairMenu, LeroHeadFeaturesMenu, LeroHeadMenu, LeroMenu,
	},
	event::CharacterField,
	fields::ColoredMultiSelectField,
	render::{block_asset, labeled_swatch, render_colored_clothing},
};

impl RenderMenu for LeroMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.base_preview_color = self.head.value.skin.value.color();
		context.accent_preview_color = self.body.value.tail_color.value.color();
		self.head.render_with(renderer, parent, context);
		self.head_features.render_with(renderer, parent, context);
		self.body.render_with(renderer, parent, context);
		self.hair.render_with(renderer, parent, context);
		self.clothing.render_with(renderer, parent, context);
		self.animation.render_with(renderer, parent, context);
	}
}

impl RenderMenu for LeroHeadMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.preview_color = self.skin.value.color();
		block_asset("Head", CharacterField::LeroHead, self.head).render_with(renderer, parent, context);
		labeled_swatch("Scales", CharacterField::LeroSkinColor, self.skin).render_with(renderer, parent, context);
	}
}

impl RenderMenu for LeroHeadFeaturesMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.preview_color = self.eye_color.value.color();
		labeled_swatch("Eye Color", CharacterField::LeroEyeColor, self.eye_color)
			.render_with(renderer, parent, context);
		context.preview_color = context.base_preview_color;
		block_asset("Snout", CharacterField::LeroMouth, self.snout).render_with(renderer, parent, context);
	}
}

impl RenderMenu for LeroBodyMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.preview_color = self.tail_color.value.color();
		labeled_swatch("Tail Color", CharacterField::LeroTailColor, self.tail_color)
			.render_with(renderer, parent, context);
		context.preview_color = self.spine_color.value.color();
		labeled_swatch("Spine Color", CharacterField::LeroSpineColor, self.spine_color)
			.render_with(renderer, parent, context);
	}
}

impl RenderMenu for LeroHairMenu {
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

impl RenderMenu for LeroClothingMenu {
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
