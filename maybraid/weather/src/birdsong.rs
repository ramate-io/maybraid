//! Periodic birdsong near the listener. Independent of live herds.

use bevy::prelude::*;
use maybraid_audio::{AmbientPick, AmbientSounds, Audio, AudioClip, Mixer};

use crate::wind::{self, point_near_listener};

const BIRDSONG_GAP_MIN: f32 = 10.0;
const BIRDSONG_GAP_MAX: f32 = 22.0;
const BIRDSONG_LIFT: f32 = 3.4;

/// When the next song may play.
#[derive(Resource, Debug)]
pub struct BirdsongClock {
	pub next: f32,
	pub noise: u64,
	pub pick: AmbientPick,
}

impl Default for BirdsongClock {
	fn default() -> Self {
		Self { next: 3.0, noise: 0xA5A5_C3C3_1F2E_8D90, pick: AmbientPick::default() }
	}
}

pub(crate) fn spawn_birdsong_near_listener(
	time: Res<Time>,
	mut commands: Commands,
	mut clock: ResMut<BirdsongClock>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	sounds: Option<Res<AmbientSounds>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
) {
	let (Some(audio), Some(sounds), Some(mut mixer), Some(listener)) =
		(audio.as_deref(), sounds.as_deref(), mixer, listeners.iter().next())
	else {
		return;
	};
	clock.next = (clock.next - time.delta_secs()).max(0.0);
	if clock.next > 0.0 {
		return;
	}
	let origin = listener.translation();
	let BirdsongClock { next, noise, pick } = &mut *clock;
	let mut world = point_near_listener(origin, noise);
	world.y += BIRDSONG_LIFT;
	sounds.play_birdsong(
		&mut commands,
		pick,
		noise,
		&clips,
		audio,
		&mut mixer,
		listener,
		world,
	);
	*next = wind::lerp(BIRDSONG_GAP_MIN, BIRDSONG_GAP_MAX, wind::unit(noise));
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn first_song_is_not_immediate() {
		let clock = BirdsongClock::default();
		assert!(clock.next > 0.0);
		assert!(clock.next < BIRDSONG_GAP_MIN);
	}
}
