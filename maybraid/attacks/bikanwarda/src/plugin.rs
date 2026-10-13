use attacks::AttackCatalog;
use bevy::prelude::*;

use crate::{
	cue_plugin, parade_plugin, placeholder_presentation_plugin, register_bikanwarda_attacks,
	BikanwardaAttackParams,
};

/// Registers Parade / God Beam / Bray recipes, the Parade sequencer, and cue
/// messages. The host still adds [`attacks::AttacksPlugin`] and physics.
///
/// This plugin is **not** installed by `maybraid-world` or the Self loop.
pub struct BikanwardaAttacksPlugin {
	pub placeholders: bool,
}

impl Default for BikanwardaAttacksPlugin {
	fn default() -> Self {
		Self { placeholders: true }
	}
}

impl Plugin for BikanwardaAttacksPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<BikanwardaAttackParams>();
		app.add_systems(Startup, register_catalog);
		parade_plugin(app);
		cue_plugin(app);
		if self.placeholders {
			placeholder_presentation_plugin(app);
		}
	}
}

fn register_catalog(mut catalog: ResMut<AttackCatalog>, params: Res<BikanwardaAttackParams>) {
	register_bikanwarda_attacks(&mut catalog, &params);
}
