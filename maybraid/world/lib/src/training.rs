//! Training Ground does not seat a second scene on the streamed world.
//!
//! While this flag is set, Discovery waypoints are not written. The game shell
//! sets it for the Training session. The free-for-all itself lives in
//! `maybraid-game-mode-training-ground`.

use bevy::prelude::*;

/// Set by the game shell while Training Ground is the live session.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingGrounds(pub bool);
