//! Headless message-export fixture; does not launch a game or renderer.
use bevy::prelude::*;
use std::time::Duration;
fn main() -> Result<(), bevy::ecs::system::RunSystemError> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, bevy::log::LogPlugin::default()));
	app.finish();
	std::thread::sleep(Duration::from_secs(2));
	packed_grove_diagnostics::headless_fixture_once()?;
	std::thread::sleep(Duration::from_secs(2));
	Ok(())
}
