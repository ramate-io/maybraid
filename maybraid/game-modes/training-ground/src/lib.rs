//! Training Ground is a seeded FinePatch of the Maybraid world.
//!
//! The shell requests this mode on [`terrain_layer_model::ActiveGenerationMode`].
//! Later children implement layer schemes on [`TrainingGround`].

use terrain_layer_model::GenerationMode;

mod round;

pub use round::{TrainingMap, TrainingRound, TRAINING_FINE_HALF_EXTENT_CELLS};

/// Pinned FinePatch generation. Layers take this as a plugin parameter.
pub struct TrainingGround;

impl GenerationMode for TrainingGround {}
