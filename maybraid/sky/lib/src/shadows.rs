//! Sun cascade quality for the sky directional light.
//!
//! High matches Bevy's default (4 cascades to 150 m, 2048²). Low is the
//! integrated-GPU step from [#824](https://github.com/ramate-io/maybraid/issues/824):
//! the same four cascades to 60 m and a 512² map. Off disables the sun's
//! shadow maps.
//!
//! Cascade **count** stays at 4 for every preset. Bevy 0.19's directional
//! visibility pass keeps a per-thread queue sized to the last cascade count;
//! shrinking to 2 and later growing back to 4 panics
//! (`index out of bounds: the len is 2 but the index is 2`).

use bevy::camera::primitives::CascadesFrusta;
use bevy::camera::visibility::CascadesVisibleEntities;
use bevy::light::{
	CascadeShadowConfig, CascadeShadowConfigBuilder, Cascades, DirectionalLightShadowMap,
};
use bevy::prelude::*;

/// Marker on the shadow-casting sun. The fill light is unmarked.
#[derive(Component, Debug, Clone, Copy)]
pub struct SkySun;

impl SkySun {
	/// Late-afternoon key pose. Light shines along [`Transform::forward`].
	pub fn pose() -> Transform {
		Transform::from_rotation(Quat::from_euler(
			EulerRot::XYZ,
			crate::SUN_PITCH,
			crate::SUN_YAW,
			0.0,
		))
	}

	/// Direction the key illuminates (Bevy local −Z).
	pub fn shine_direction(transform: &Transform) -> Vec3 {
		transform.forward().as_vec3()
	}

	/// Direction from the camera toward the visible disk (opposite the shine ray).
	pub fn disk_direction(transform: &Transform) -> Vec3 {
		-Self::shine_direction(transform)
	}

	/// Camera-relative offset for an unlit disk sitting on the key axis.
	pub fn disk_offset(transform: &Transform, distance: f32) -> Vec3 {
		Self::disk_direction(transform) * distance
	}
}

/// User-facing sun shadow quality.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShadowQuality {
	Off,
	Low,
	#[default]
	High,
}

impl ShadowQuality {
	pub const ALL: [Self; 3] = [Self::High, Self::Low, Self::Off];

	pub fn cycle(self) -> Self {
		match self {
			Self::High => Self::Low,
			Self::Low => Self::Off,
			Self::Off => Self::High,
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::High => "High",
			Self::Low => "Low",
			Self::Off => "Off",
		}
	}

	pub fn maps_enabled(self) -> bool {
		!matches!(self, Self::Off)
	}

	pub fn shadow_map_size(self) -> usize {
		match self {
			Self::High => 2048,
			Self::Low => 512,
			Self::Off => 2048,
		}
	}

	pub fn cascade_config(self) -> CascadeShadowConfig {
		match self {
			Self::High | Self::Off => CascadeShadowConfigBuilder::default().build(),
			Self::Low => CascadeShadowConfigBuilder {
				num_cascades: 4,
				maximum_distance: 60.0,
				first_cascade_far_bound: 10.0,
				..default()
			}
			.build(),
		}
	}

	pub fn shadow_map(self) -> DirectionalLightShadowMap {
		DirectionalLightShadowMap { size: self.shadow_map_size() }
	}
}

pub(crate) fn apply_shadow_quality(
	quality: Res<ShadowQuality>,
	mut map: ResMut<DirectionalLightShadowMap>,
	mut lights: Query<(Entity, &mut DirectionalLight), With<SkySun>>,
	mut commands: Commands,
) {
	map.size = quality.shadow_map_size();
	let config = quality.cascade_config();
	let enabled = quality.maps_enabled();
	for (entity, mut light) in &mut lights {
		light.shadow_maps_enabled = enabled;
		commands.entity(entity).insert((
			config.clone(),
			Cascades::default(),
			CascadesFrusta::default(),
			CascadesVisibleEntities::default(),
		));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cycle_visits_every_preset() {
		assert_eq!(ShadowQuality::High.cycle(), ShadowQuality::Low);
		assert_eq!(ShadowQuality::Low.cycle(), ShadowQuality::Off);
		assert_eq!(ShadowQuality::Off.cycle(), ShadowQuality::High);
	}

	#[test]
	fn low_keeps_four_near_cascades() {
		let low = ShadowQuality::Low.cascade_config();
		let high = ShadowQuality::High.cascade_config();
		assert_eq!(low.bounds.len(), high.bounds.len());
		assert_eq!(low.bounds.len(), 4);
		assert!((low.bounds[3] - 60.0).abs() < 1e-3);
		assert_eq!(ShadowQuality::Low.shadow_map_size(), 512);
		assert!(ShadowQuality::Low.maps_enabled());
	}

	#[test]
	fn cycling_never_changes_cascade_count() {
		let count = ShadowQuality::High.cascade_config().bounds.len();
		let mut quality = ShadowQuality::High;
		for _ in 0..ShadowQuality::ALL.len() * 3 {
			quality = quality.cycle();
			assert_eq!(quality.cascade_config().bounds.len(), count);
		}
	}

	#[test]
	fn high_keeps_bevy_defaults() {
		let high = ShadowQuality::High.cascade_config();
		let bevy = CascadeShadowConfigBuilder::default().build();
		assert_eq!(high.bounds, bevy.bounds);
		assert_eq!(ShadowQuality::High.shadow_map_size(), 2048);
		assert!(ShadowQuality::High.maps_enabled());
		assert!(!ShadowQuality::Off.maps_enabled());
	}
}
