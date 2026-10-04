//! Hanabi helpers: SPRITE_INDEX from age / playback, clamped to the last frame.

use bevy_hanabi::prelude::{Attribute, ExprWriter, SetAttributeModifier, WriterExpr};
use bevy_hanabi::ScalarType;

use crate::assets::FlipbookAsset;

/// `min(age / playback * frames, last_frame)` as [`Attribute::SPRITE_INDEX`].
pub fn sprite_index_from_playback(writer: &ExprWriter, flipbook: &FlipbookAsset) -> WriterExpr {
	let frames = writer.lit(flipbook.frame_count as f32);
	let last = writer.lit(flipbook.last_frame() as f32);
	let playback = writer.lit(flipbook.playback.max(1e-3));
	writer.attr(Attribute::AGE).div(playback).mul(frames).min(last)
}

pub fn update_sprite_index(writer: &ExprWriter, flipbook: &FlipbookAsset) -> SetAttributeModifier {
	SetAttributeModifier::new(
		Attribute::SPRITE_INDEX,
		sprite_index_from_playback(writer, flipbook).cast(ScalarType::Int).expr(),
	)
}
