//! Capture-only env for evidence stills. Interactive play is unchanged.
//!
//! `VFX_CAPTURE=1` hides the command drawer, disables looping, and is the
//! switch the evidence script uses. Optional `VFX_FREEZE`, `VFX_NOLOOP`, and
//! `VFX_CANCEL_AT` pin lifetime frames.

use bevy::prelude::*;
use maybraid_vfx::{apply_cancel, VfxCancel, VfxInstance};

use crate::commands::{LoopEnabled, PreviewFreeze};

pub fn requested() -> bool {
	std::env::var_os("VFX_CAPTURE").is_some()
}

/// `base` — God Beam close enough that the 1.8 m stand-in reads.
/// `side` — sonic rings from the side; anything else is 3/4 for sonic.
pub fn frame() -> Option<String> {
	std::env::var("VFX_FRAME").ok().filter(|value| !value.is_empty())
}

/// Age at which capture should fade live instances, then keep playing.
#[derive(Resource, Default)]
pub struct CaptureCancel(pub Option<f32>);

/// Dim scene lighting for night readability captures (`VFX_NIGHT=1`).
#[derive(Resource, Default)]
pub struct NightCapture(pub bool);

pub fn apply_env(
	mut looping: ResMut<LoopEnabled>,
	mut freeze: ResMut<PreviewFreeze>,
	mut cancel_at: ResMut<CaptureCancel>,
	mut night: ResMut<NightCapture>,
) {
	if std::env::var_os("VFX_NIGHT").is_some() {
		night.0 = true;
	}
	if requested() || std::env::var_os("VFX_NOLOOP").is_some() {
		looping.0 = false;
	}
	if let Ok(age) = std::env::var("VFX_FREEZE") {
		if let Ok(age) = age.parse::<f32>() {
			freeze.age = Some(age);
		}
	}
	if let Ok(age) = std::env::var("VFX_CANCEL_AT") {
		if let Ok(age) = age.parse::<f32>() {
			cancel_at.0 = Some(age);
		}
	}
}

pub fn tune_night_lighting(
	night: Res<NightCapture>,
	mut ambient: ResMut<GlobalAmbientLight>,
	mut sun: Query<&mut DirectionalLight>,
) {
	if !night.0 {
		return;
	}
	ambient.brightness = 18.0;
	ambient.color = Color::srgb(0.22, 0.28, 0.42);
	for mut light in &mut sun {
		light.illuminance = 420.0;
		light.color = Color::srgb(0.55, 0.62, 0.82);
	}
}

pub fn cancel_at_age(world: &mut World) {
	let Some(at) = world.resource::<CaptureCancel>().0 else {
		return;
	};
	let due: Vec<Entity> = world
		.query::<(Entity, &VfxInstance, Option<&VfxCancel>)>()
		.iter(world)
		.filter(|(_, instance, cancel)| {
			cancel.is_none() && instance.armed && instance.age + 1e-3 >= at
		})
		.map(|(entity, _, _)| entity)
		.collect();
	for entity in due {
		apply_cancel(world, entity);
	}
}
