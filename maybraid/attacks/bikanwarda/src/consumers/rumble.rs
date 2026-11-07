//! [`SpatialRumbleCue`] → [`maybraid_spatial_rumble::SpatialRumble`].

use bevy::prelude::*;
use maybraid_spatial_rumble::{SpatialRumble, SpatialRumbleSystems};

use crate::consumers::BikanwardaPresentationSystems;
use crate::SpatialRumbleCue;

pub fn plugin(app: &mut App) {
	app.add_systems(
		Update,
		consume_spatial_rumble_cues
			.in_set(BikanwardaPresentationSystems::Consume)
			.before(SpatialRumbleSystems::Ingest),
	);
}

fn consume_spatial_rumble_cues(
	mut cues: MessageReader<SpatialRumbleCue>,
	mut rumbles: MessageWriter<SpatialRumble>,
) {
	for cue in cues.read() {
		let rumble = cue.to_spatial_rumble();
		if !rumble.is_silent() {
			rumbles.write(rumble);
		}
	}
}
