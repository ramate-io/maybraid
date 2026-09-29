//! Weather events near the listener. Wind is breeze (looped) and gust (one-shot).

mod swirl;
mod wind;

use bevy::prelude::*;
use maybraid_audio::{AudioPlugin, AudioSystems};

pub use swirl::{crossing_pose, WindSwirl, WindSwirlEffects};
pub use wind::{
	point_near_listener, WeatherClock, WeatherEvent, WeatherKind, WeatherSounds, BREEZE_LIFE_MAX,
	BREEZE_LIFE_MIN, BREEZE_VOLUME, GUST_LIFE, GUST_VOLUME, WIND_NEAR_MAX, WIND_NEAR_MIN,
	WIND_SPATIAL_RADIUS, WIND_SPATIAL_SCALE,
};

/// Loads wind clips and keeps a breeze / gust pair near the listener.
pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<AudioPlugin>() {
			app.add_plugins(AudioPlugin);
		}
		if !app.is_plugin_added::<bevy_hanabi::HanabiPlugin>() {
			app.add_plugins(bevy_hanabi::HanabiPlugin);
		}
		app.init_resource::<WeatherClock>()
			.add_systems(Startup, (wind::setup_weather_sounds, swirl::setup_wind_swirls))
			.add_systems(
				PostUpdate,
				(
					wind::despawn_finished_weather,
					wind::spawn_weather_near_listener,
					swirl::tick_wind_swirls,
				)
					.chain()
					.after(TransformSystems::Propagate)
					.before(AudioSystems::Sync),
			);
	}
}
