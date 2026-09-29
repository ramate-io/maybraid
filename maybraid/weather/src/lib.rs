//! Weather events near the listener. Wind is breeze (looped) and gust (one-shot).

mod wind;

use bevy::prelude::*;
use maybraid_audio::{AudioPlugin, AudioSystems};

pub use wind::{
	BREEZE_LIFE_MAX, BREEZE_LIFE_MIN, BREEZE_VOLUME, GUST_LIFE, GUST_VOLUME, WIND_NEAR_MAX,
	WIND_NEAR_MIN, WIND_SPATIAL_RADIUS, WIND_SPATIAL_SCALE, WeatherClock, WeatherEvent,
	WeatherKind, WeatherSounds, point_near_listener,
};

/// Loads wind clips and keeps a breeze / gust pair near the listener.
pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<AudioPlugin>() {
			app.add_plugins(AudioPlugin);
		}
		app.init_resource::<WeatherClock>()
			.add_systems(Startup, wind::setup_weather_sounds)
			.add_systems(
				PostUpdate,
				(wind::despawn_finished_weather, wind::spawn_weather_near_listener)
					.chain()
					.after(TransformSystems::Propagate)
					.before(AudioSystems::Sync),
			);
	}
}
