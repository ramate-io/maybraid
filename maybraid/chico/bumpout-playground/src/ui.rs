use bevy::prelude::*;
use chico_bumpout::BumpOut;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::{NeighborhoodControls, PresenterLayer, TileCoordinate};

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Chico bump-outs - / cmd - F1 drawer - RMB look - WASD/QE move".into(),
		empty_console_text: "Try `help`, `neighborhood set`, or `visibility toggle`".into(),
		root_background: Color::srgba(0.08, 0.16, 0.12, 0.84),
		controls_hint: "help - Enter - up/down history - PgUp/PgDn - Shift+up/down scroll".into(),
	}
}

pub fn sync_command_status_text(
	controls: Res<NeighborhoodControls>,
	layers: Query<(&PresenterLayer, &TileCoordinate, &BumpOut)>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let Some((_, _, bump_out)) = layers
		.iter()
		.find(|(layer, tile, _)| **layer == controls.layer && tile.0 == IVec2::ZERO)
	else {
		status.0 = "Waiting for center bump-out tile…".into();
		return;
	};
	let neighborhood = bump_out.neighborhood();
	let index = controls.sample_index();
	let coordinate = controls.selected_coordinate();
	status.0 = format!(
		"5×5 tiles | editing {} sample ({}, {})\n\
		 density {:.3} | bite-size {:.2} | bite-size-deviation {:.2}\n\
		 average-height {:.2} | height-deviation {:.2}\n\
		 / neighborhood layer <ground-cover|canopy-proxy>\n\
		 / neighborhood select --x <-1..1> --z <-1..1>\n\
		 / neighborhood set --density … --bite-size … --bite-size-deviation …\n\
		 / neighborhood adjust --average-height … --height-deviation …",
		controls.layer.label(),
		coordinate.x,
		coordinate.y,
		neighborhood.densities[index],
		neighborhood.bite_sizes[index],
		neighborhood.bite_size_deviations[index],
		neighborhood.average_heights[index],
		neighborhood.height_deviations[index],
	);
}
