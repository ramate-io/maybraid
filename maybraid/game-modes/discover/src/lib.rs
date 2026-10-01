//! Discovery: the streamed Maybraid world.
//!
//! The game shell still owns cameras, loading, and pause. This crate is the
//! session those systems ask when the home row enters Discovery.

use terrain_layer_model::GenerationMode;

pub const LABEL: &str = "Discovery";

/// Terrain, vegetation, and urbanization stream while Discovery is in the world shell.
pub fn streams_terrain(session_is_discovery: bool, in_world_shell: bool) -> bool {
	session_is_discovery && in_world_shell
}

/// Playable-world generation. Layers take this as a plugin parameter.
pub struct Discovery;

impl GenerationMode for Discovery {}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn discovery_streams_only_inside_the_world_shell() {
		assert!(streams_terrain(true, true));
		assert!(!streams_terrain(true, false));
		assert!(!streams_terrain(false, true));
	}
}
