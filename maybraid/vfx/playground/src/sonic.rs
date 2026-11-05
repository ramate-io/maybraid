//! Playground-only driver. Writes [`VfxWaveStates`]; the effect only renders.

use bevy::prelude::*;
use maybraid_vfx::spawn::VfxPendingLayers;
use maybraid_vfx::{VfxInstance, VfxWaveStates};

pub fn drive_sonic_waves(
	mut waves: Query<(&VfxInstance, &VfxPendingLayers, &mut VfxWaveStates, &Transform)>,
) {
	for (instance, pending, mut states, transform) in &mut waves {
		if !instance.armed {
			states.waves.clear();
			continue;
		}
		states.waves = pending.spawn.drive.sonic_waves_at(
			transform.translation,
			transform.rotation * Vec3::Z,
			instance.age,
		);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use maybraid_vfx::{VfxDrive, SONIC_WAVE_INTERVAL, SONIC_WAVE_SPEED};

	#[test]
	fn playground_driver_follows_authored_state() -> Result<(), String> {
		let origin = Vec3::new(0.0, 1.55, 0.0);
		let waves = VfxDrive::sonic_wave().sonic_waves_at(origin, Vec3::Z, SONIC_WAVE_INTERVAL);
		if waves.len() != 2 {
			return Err(format!("expected first two rings, got {}", waves.len()));
		}
		if waves[1].travel() > 1e-3 {
			return Err("second ring should still be at the mouth".into());
		}
		if (waves[0].travel() - SONIC_WAVE_SPEED * SONIC_WAVE_INTERVAL).abs() >= 1e-2 {
			return Err(format!("first ring {}", waves[0].travel()));
		}
		if (waves[0].position.y - origin.y).abs() >= 1e-4 {
			return Err("left mouth height".into());
		}
		Ok(())
	}
}
