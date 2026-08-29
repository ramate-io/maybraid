//! Shared virtual pad for Maybraid: device producers, current-frame snapshot,
//! optional history, and derived surfaces (menu nav, cursor).

pub mod analog;
pub mod button;
pub mod config;
pub mod gate;
pub mod history;
pub mod pad;
pub mod produce;
pub mod surface;

pub use analog::{Cardinal, Deadzone};
pub use button::{ButtonPhase, ButtonStroke, PadButton, PAD_BUTTON_COUNT};
pub use config::VirtualPadConfig;
pub use gate::PadGameplayEnabled;
pub use history::{PadEdge, PadHistory, PadSnapshot, Timed};
pub use pad::VirtualPad;
pub use surface::cursor::PadCursor;
pub use surface::menu::{MenuNav, MenuNavPad};

use bevy::input::InputSystems;
use bevy::prelude::*;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VirtualPadSystems {
	Produce,
	Derive,
}

#[derive(Default)]
pub struct VirtualPadPlugin {
	pub config: VirtualPadConfig,
}

impl VirtualPadPlugin {
	pub fn new(config: VirtualPadConfig) -> Self {
		Self { config }
	}
}

impl Plugin for VirtualPadPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(self.config.clone())
			.init_resource::<VirtualPad>()
			.init_resource::<PadHistory>()
			.init_resource::<PadGameplayEnabled>()
			.init_resource::<MenuNavPad>()
			.init_resource::<PadCursor>()
			.configure_sets(
				PreUpdate,
				VirtualPadSystems::Produce.after(InputSystems).before(VirtualPadSystems::Derive),
			)
			.configure_sets(PreUpdate, VirtualPadSystems::Derive.after(VirtualPadSystems::Produce));
		produce::configure_produce(app);
		app.add_systems(
			PreUpdate,
			(
				history::push_history,
				surface::menu::derive_menu_nav,
				surface::cursor::integrate_cursor,
			)
				.chain()
				.in_set(VirtualPadSystems::Derive),
		);
	}
}
