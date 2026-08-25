//! Shared Barlow faces and the Maybraid mark for panel widgets.

use bevy::prelude::*;

use crate::theme::{BARLOW_BLACK, BARLOW_REGULAR, BARLOW_SEMIBOLD, MAYBRAID_LOGO};

/// Font and logo handles loaded from [`crate::theme`] asset paths.
#[derive(Clone, Debug)]
pub struct HudFonts {
	pub black: Handle<Font>,
	pub semibold: Handle<Font>,
	pub regular: Handle<Font>,
	pub logo: Handle<Image>,
}

impl HudFonts {
	pub fn load(asset_server: &AssetServer) -> Self {
		Self {
			black: asset_server.load(BARLOW_BLACK),
			semibold: asset_server.load(BARLOW_SEMIBOLD),
			regular: asset_server.load(BARLOW_REGULAR),
			logo: asset_server.load(MAYBRAID_LOGO),
		}
	}

	pub fn header(&self, size: f32) -> TextFont {
		text_font(&self.black, size)
	}

	pub fn item(&self, size: f32) -> TextFont {
		text_font(&self.semibold, size)
	}

	pub fn body(&self, size: f32) -> TextFont {
		text_font(&self.regular, size)
	}
}

fn text_font(font: &Handle<Font>, size: f32) -> TextFont {
	TextFont { font: font.clone().into(), font_size: FontSize::Px(size), ..default() }
}
