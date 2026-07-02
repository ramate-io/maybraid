use bevy::prelude::*;
use bevy_character_ui_menu_renderer::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};

use crate::{
	characters::wumbus::{
		WumbusClothingMenu, WumbusHairMenu, WumbusHeadFeaturesMenu, WumbusHeadMenu, WumbusMenu,
	},
	event::CharacterField,
	fields::ColoredMultiSelectField,
	render::{block_asset, labeled_cycle, labeled_swatch, render_colored_clothing},
};

impl RenderMenu for WumbusMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.base_preview_color = self.head.value.skin.value.color();
		context.accent_preview_color = self.head.value.horn_color.value.color();
		self.head.render_with(renderer, parent, context);
		self.head_features.render_with(renderer, parent, context);
		self.hair.render_with(renderer, parent, context);
		self.clothing.render_with(renderer, parent, context);
		self.animation.render_with(renderer, parent, context);
	}
}

impl RenderMenu for WumbusHeadMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.preview_color = self.skin.value.color();
		block_asset("Head", CharacterField::WumbusHead, self.head).render_with(renderer, parent, context);
		labeled_swatch("Fur", CharacterField::WumbusSkinColor, self.skin).render_with(renderer, parent, context);
		context.preview_color = context.accent_preview_color;
		labeled_cycle("Horns", CharacterField::WumbusHorns, self.horns).render_with(renderer, parent, context);
		labeled_swatch("Horn Color", CharacterField::WumbusHornColor, self.horn_color)
			.render_with(renderer, parent, context);
	}
}

impl RenderMenu for WumbusHeadFeaturesMenu {
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		context.preview_color = self.eye_color.value.color();
		block_asset("Eyes", CharacterField::Eye, self.eye).render_with(renderer, parent, context);
		labeled_swatch("Eye Color", CharacterField::WumbusEyeColor, self.eye_color)
			.render_with(renderer, parent, context);
		context.preview_color = self.ear_color.value.color();
		labeled_swatch("Ear Color", CharacterField::WumbusEarColor, self.ear_color)
			.render_with(renderer, parent, context);
		context.preview_color = self.mouth_color.value.color();
		block_asset("Snout", CharacterField::WumbusMouth, self.snout).render_with(renderer, parent, context);
		labeled_swatch("Mouth Color", CharacterField::WumbusMouthColor, self.mouth_color)
			.render_with(renderer, parent, context);
	}
}

impl RenderMenu for WumbusHairMenu {
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

impl RenderMenu for WumbusClothingMenu {
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
