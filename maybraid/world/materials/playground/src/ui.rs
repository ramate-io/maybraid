use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::chain_preview::ChainPreview;
use crate::preview::{PreviewConfig, PreviewHost};
use crate::shoot::ShootConfig;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "World materials — / cmd — WASD/Space/Shift — mouse look".into(),
		empty_console_text:
			"Console: `show pulse|doric|chain`, `mesh blast|cylinder|taper`, `chain hit 6`, `chain capture`, `help`"
				.into(),
		root_background: Color::srgba(0.08, 0.09, 0.12, 0.86),
		controls_hint: "help — mesh blast — show pulse — chain show pair — Enter — history".into(),
	}
}

pub(crate) fn sync_command_status_text(
	host: Res<PreviewHost>,
	preview: Res<PreviewConfig>,
	chain: Res<ChainPreview>,
	shoot: Res<ShootConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = match *host {
		PreviewHost::Catalog => {
			let firing = if shoot.enabled { "shooting" } else { "idle" };
			format!("{} · {firing}", preview.status_label())
		}
		PreviewHost::Chain => chain.status_label(),
	};
}
