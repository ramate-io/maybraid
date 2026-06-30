use bevy::input::gamepad::GamepadButton;
use bevy::prelude::*;

/// Whether mouse / stick look input should rotate the camera.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CameraLookEnabled(pub bool);

/// Keyboard and gamepad bindings that flip [`CameraLookEnabled`].
#[derive(Clone, Debug)]
pub struct CameraLookConfig {
	pub enabled_at_start: bool,
	pub toggle_keys: Vec<KeyCode>,
	pub toggle_gamepad_buttons: Vec<GamepadButton>,
}

impl Default for CameraLookConfig {
	fn default() -> Self {
		Self {
			enabled_at_start: true,
			toggle_keys: vec![KeyCode::KeyL],
			toggle_gamepad_buttons: Vec::new(),
		}
	}
}

/// Stored so toggle systems can read bindings without cloning each frame.
#[derive(Resource, Clone)]
pub struct CameraLookConfigResource(pub CameraLookConfig);

pub struct CameraLookPlugin {
	pub config: CameraLookConfig,
}

impl Default for CameraLookPlugin {
	fn default() -> Self {
		Self { config: CameraLookConfig::default() }
	}
}

impl CameraLookPlugin {
	pub fn new(config: CameraLookConfig) -> Self {
		Self { config }
	}
}

impl Plugin for CameraLookPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(CameraLookConfigResource(self.config.clone()))
			.insert_resource(CameraLookEnabled(self.config.enabled_at_start))
			.add_systems(Update, toggle_camera_look);
	}
}

pub fn toggle_camera_look(
	keyboard: Res<ButtonInput<KeyCode>>,
	gamepads: Query<&Gamepad>,
	config: Res<CameraLookConfigResource>,
	mut look_enabled: ResMut<CameraLookEnabled>,
) {
	if toggle_binding_pressed(
		&keyboard,
		&gamepads,
		&config.0.toggle_keys,
		&config.0.toggle_gamepad_buttons,
	) {
		look_enabled.0 = !look_enabled.0;
	}
}

pub fn look_input_active(look_enabled: Option<Res<CameraLookEnabled>>) -> bool {
	look_enabled.map(|enabled| enabled.0).unwrap_or(true)
}

fn toggle_binding_pressed(
	keyboard: &ButtonInput<KeyCode>,
	gamepads: &Query<&Gamepad>,
	keys: &[KeyCode],
	buttons: &[GamepadButton],
) -> bool {
	keys.iter().any(|key| keyboard.just_pressed(*key))
		|| gamepads
			.iter()
			.any(|gamepad| buttons.iter().any(|button| gamepad.just_pressed(*button)))
}
