//! Generated rosters presented as semantic LodScene mobs with persistent brains.

mod brain;
mod kind;
mod plugin;
mod roster;
mod scene;

pub use brain::MobBrain;
pub use kind::MobKind;
pub use plugin::{MobSceneSystems, MobScenesPlugin};
pub use roster::{MobMemberRecipe, MobRosterRecipe};
pub use scene::{Mob, MobScene, DEFAULT_MOB_HIGH_RADIUS};
