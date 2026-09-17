//! Blue / haze dome over an opaque Cosimo field.
//!
//! The inner shell is atmosphere: 4D blue and haze patches with alpha holes.
//! The outer field is cosmos. [`SkyClock`] drives palette and key pose.

mod apply;
mod celestial;
mod clock;
mod dome;
mod field;
mod shadows;

use bevy::prelude::*;
use std::f32::consts::PI;

use apply::apply_sky_mood;
use celestial::spawn_sky_celestial;
use dome::{spawn_sky_wash, DomeSettings, SkyDomeMaterialPlugin};
use field::{spawn_sky_field, SkyFieldMaterialPlugin};

pub use celestial::{
	SkyFill, SkyMoon, SkySunDisk, CELESTIAL_DISTANCE_FACTOR, MOON_COLOR, MOON_LIFT, MOON_RADIUS_M,
	MOON_YAW_OFFSET, SUN_CORONA_COLOR, SUN_CORONA_RADIUS_M, SUN_DISK_COLOR, SUN_DISK_RADIUS_M,
};
pub use clock::{
	SkyClock, SkyCommand, SkyMood, DEFAULT_SKY_PERIOD_SECS, SKY_PHASE_DAWN, SKY_PHASE_DUSK,
	SKY_PHASE_GOLDEN, SKY_PHASE_MORNING, SKY_PHASE_NIGHT, SKY_PHASE_NOON,
};
pub use dome::{SkyDomeMaterial, SkyWash};
pub use field::{SkyField, SkyFieldMaterial, FIELD_RADIUS_FACTOR};
pub use shadows::{ShadowQuality, SkySun};

/// Start a light haze at this XZ radius (m).
pub const DEFAULT_INNER_FADE_M: f32 = 350.0;
/// Haze reaches [`SkyDomePlugin::max_alpha`] at this XZ radius (m).
pub const DEFAULT_OUTER_FADE_M: f32 = 1_200.0;
/// Sphere mesh radius. Larger than the fade so the shell stays off the near ground.
pub const DEFAULT_SPHERE_RADIUS_M: f32 = 2_800.0;
/// Peak atmosphere alpha. Holes still open onto cosmos.
pub const DEFAULT_MAX_ALPHA: f32 = 0.78;

/// Cooler overhead. More chroma than the old dusty pale.
pub const SKY_ZENITH: Color = Color::hsla(210.0, 0.42, 0.58, 1.0);
/// Warm peach / wheat wash at the horizon band.
pub const SKY_HORIZON: Color = Color::hsla(36.0, 0.62, 0.70, 1.0);
/// Darker below-horizon ring so the shell does not glow under the camera.
pub const SKY_NADIR: Color = Color::hsla(26.0, 0.28, 0.22, 1.0);
/// Late-afternoon clear / hole fallback. Not the old cyan, not dust.
pub const SKY_CLEAR: Color = Color::hsla(208.0, 0.38, 0.64, 1.0);
/// Playground alias for [`SKY_CLEAR`].
pub const SKY_BLUE: Color = SKY_CLEAR;

/// Soft amber key. Not sodium orange.
pub const SUN_COLOR: Color = Color::hsla(38.0, 0.38, 0.78, 1.0);
/// Lower than the old 12 klux white key so mid-ground is not flash-lit.
pub const SUN_ILLUMINANCE: f32 = 8_000.0;
/// Cooler, dimmer fill so the key keeps a direction.
pub const FILL_COLOR: Color = Color::hsla(218.0, 0.22, 0.70, 1.0);
pub const FILL_ILLUMINANCE: f32 = 1_600.0;
/// Lifted so Off-shadow tree wells do not go black.
pub const AMBIENT_BRIGHTNESS: f32 = 620.0;

/// Existing Discovery key pose (pitch / yaw). Golden-hour default.
pub const SUN_PITCH: f32 = -PI / 4.0;
pub const SUN_YAW: f32 = PI / 4.0;

#[derive(Component)]
pub struct SkyDome;

pub struct SkyDomePlugin {
	pub inner_fade_m: f32,
	pub outer_fade_m: f32,
	pub sphere_radius_m: f32,
	pub max_alpha: f32,
	/// Horizon wash. Zenith / nadir stay on the named palette constants.
	pub color: Color,
	pub clear: Color,
}

impl Default for SkyDomePlugin {
	fn default() -> Self {
		Self {
			inner_fade_m: DEFAULT_INNER_FADE_M,
			outer_fade_m: DEFAULT_OUTER_FADE_M,
			sphere_radius_m: DEFAULT_SPHERE_RADIUS_M,
			max_alpha: DEFAULT_MAX_ALPHA,
			color: SKY_HORIZON,
			clear: SKY_CLEAR,
		}
	}
}

impl Plugin for SkyDomePlugin {
	fn build(&self, app: &mut App) {
		let settings = DomeSettings {
			inner_fade_m: self.inner_fade_m,
			outer_fade_m: self.outer_fade_m.max(self.inner_fade_m + 1.0),
			sphere_radius_m: self.sphere_radius_m.max(self.outer_fade_m),
			max_alpha: self.max_alpha.clamp(0.0, 1.0),
			horizon: self.color,
			zenith: SKY_ZENITH,
			nadir: SKY_NADIR,
		};
		app.add_plugins((SkyDomeMaterialPlugin, SkyFieldMaterialPlugin))
			.insert_resource(ClearColor(self.clear))
			.insert_resource(settings)
			.init_resource::<ShadowQuality>()
			.init_resource::<SkyClock>()
			.add_systems(
				Startup,
				(
					spawn_sky_dome,
					spawn_sky_wash,
					spawn_sky_lights,
					spawn_sky_celestial,
					spawn_sky_field,
					shadows::apply_shadow_quality,
				)
					.chain(),
			)
			.add_systems(Update, (follow_camera, apply_sky_mood).chain())
			.add_systems(
				PostUpdate,
				shadows::apply_shadow_quality.run_if(resource_changed::<ShadowQuality>),
			);
	}
}

fn spawn_sky_dome(mut commands: Commands) {
	commands.spawn((SkyDome, Transform::IDENTITY, Visibility::Visible, Name::new("sky-dome")));
}

fn follow_camera(
	camera: Query<&GlobalTransform, With<Camera3d>>,
	mut dome: Query<&mut Transform, With<SkyDome>>,
) {
	let Ok(cam) = camera.single() else {
		return;
	};
	let Ok(mut tf) = dome.single_mut() else {
		return;
	};
	tf.translation = cam.translation();
	tf.rotation = Quat::IDENTITY;
}

fn spawn_sky_lights(mut commands: Commands, quality: Res<ShadowQuality>, clock: Res<SkyClock>) {
	let mood = clock.sample();
	commands.insert_resource(GlobalAmbientLight { brightness: mood.ambient, ..default() });
	commands.insert_resource(quality.shadow_map());
	commands.spawn((
		SkySun,
		DirectionalLight {
			color: mood.sun_color,
			illuminance: mood.sun_illuminance,
			shadow_maps_enabled: quality.maps_enabled(),
			..default()
		},
		quality.cascade_config(),
		mood.sun_pose(),
	));
	commands.spawn((
		SkyFill,
		DirectionalLight {
			color: mood.fill_color,
			illuminance: mood.fill_illuminance,
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}

#[cfg(test)]
mod tests {
	use super::*;

	fn sky_test_app(quality: ShadowQuality) -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.insert_resource(quality)
			.insert_resource(SkyClock::golden())
			.add_plugins(SkyDomePlugin::default());
		app.update();
		app
	}

	#[test]
	fn sun_disk_tracks_the_key() -> anyhow::Result<()> {
		let mut app = sky_test_app(ShadowQuality::High);
		let sun_dir = {
			let mut suns = app.world_mut().query_filtered::<&Transform, With<SkySun>>();
			let sun = *suns.single(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?;
			SkySun::disk_direction(&sun)
		};
		let disk_dir = {
			let mut disks = app.world_mut().query_filtered::<&Transform, With<SkySunDisk>>();
			let disk = disks.single(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?;
			disk.translation.normalize_or_zero()
		};
		assert!(sun_dir.dot(disk_dir) > 0.995, "disk={disk_dir} sun={sun_dir}");
		Ok(())
	}

	#[test]
	fn no_extra_shadow_casters() -> anyhow::Result<()> {
		let mut app = sky_test_app(ShadowQuality::High);
		let lights: Vec<(bool, bool)> = {
			let mut query = app.world_mut().query::<(&DirectionalLight, Option<&SkySun>)>();
			query
				.iter(app.world())
				.map(|(light, sun)| (sun.is_some(), light.shadow_maps_enabled))
				.collect()
		};
		assert_eq!(lights.len(), 2, "still two directionals");
		let sun_shadows = lights.iter().filter(|(is_sun, shadows)| *is_sun && *shadows).count();
		let fill_shadows = lights.iter().filter(|(is_sun, shadows)| !*is_sun && *shadows).count();
		assert_eq!(sun_shadows, 1);
		assert_eq!(fill_shadows, 0);
		Ok(())
	}

	#[test]
	fn off_quality_disables_sun_maps() -> anyhow::Result<()> {
		let mut app = sky_test_app(ShadowQuality::Off);
		let mut query = app.world_mut().query_filtered::<&DirectionalLight, With<SkySun>>();
		let sun = query.single(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!sun.shadow_maps_enabled);
		Ok(())
	}

	#[test]
	fn field_shell_spawns_on_the_dome() -> anyhow::Result<()> {
		let mut app = sky_test_app(ShadowQuality::High);
		let mut fields = app.world_mut().query_filtered::<&Transform, With<SkyField>>();
		fields.single(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn blue_dome_spawns_on_the_root() -> anyhow::Result<()> {
		let mut app = sky_test_app(ShadowQuality::High);
		let mut wash = app.world_mut().query_filtered::<&Transform, With<SkyWash>>();
		wash.single(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn night_phase_hides_the_sun_disk() -> anyhow::Result<()> {
		let mut app = sky_test_app(ShadowQuality::High);
		app.insert_resource(SkyClock {
			phase: SKY_PHASE_NIGHT,
			period_secs: DEFAULT_SKY_PERIOD_SECS,
			paused: true,
		});
		app.update();
		let mut disks = app.world_mut().query_filtered::<&Visibility, With<SkySunDisk>>();
		let visibility =
			*disks.single(app.world()).map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(visibility, Visibility::Hidden);
		Ok(())
	}
}
