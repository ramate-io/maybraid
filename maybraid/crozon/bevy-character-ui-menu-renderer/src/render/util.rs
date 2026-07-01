use bevy::prelude::*;

pub(crate) fn color_to_key(color: Color) -> [u8; 4] {
	[
		(color.to_srgba().red * 255.0) as u8,
		(color.to_srgba().green * 255.0) as u8,
		(color.to_srgba().blue * 255.0) as u8,
		(color.to_srgba().alpha * 255.0) as u8,
	]
}
