//! Generated rosters presented as semantic LodScene mobs with persistent brains.

mod brain;
mod kind;
mod plugin;
mod roster;
mod roster_ref;
mod scene;

pub use brain::{player_affiliations, MobBrain, FFA_GROUP, PLAYER_GROUP};
pub use kind::MobKind;
pub use plugin::{MobLodRefreshMode, MobSceneSystems, MobScenesPlugin};
pub use roster::{MobMemberRecipe, MobRosterRecipe};
pub use scene::{Mob, MobScene, DEFAULT_MOB_HIGH_RADIUS};
