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

/// Age at which capture should fade live instances, then keep playing.
#[derive(Resource, Default)]
pub struct CaptureCancel(pub Option<f32>);

pub fn apply_env(
	mut looping: ResMut<LoopEnabled>,
	mut freeze: ResMut<PreviewFreeze>,
	mut cancel_at: ResMut<CaptureCancel>,
) {
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
