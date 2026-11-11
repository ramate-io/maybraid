//! Per-body hues for spiritual spawn and despawn bursts.

use bevy::prelude::*;
use damage::Health;
use maybraid_vfx::ExplosionPalette;
use player::{Npc, Player};

/// Hue that drives `ExplosionPalette::spirit_burst` for this character.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct SpiritTint(pub Color);

pub const DEFAULT_SPIRIT_TINT: Color = Color::srgba(0.12, 0.88, 1.05, 1.0);

impl Default for SpiritTint {
	fn default() -> Self {
		Self(DEFAULT_SPIRIT_TINT)
	}
}

impl SpiritTint {
	pub fn palette(self) -> ExplosionPalette {
		ExplosionPalette::spirit_burst(self.0)
	}
}

pub(crate) fn configure_spirit_tints(app: &mut App) {
	app.add_systems(
		PostUpdate,
		ensure_default_spirit_tints
			.after(super::CombatVfxSystems::Impact)
			.before(super::CombatVfxSystems::Materialize),
	);
}

fn ensure_default_spirit_tints(
	mut commands: Commands,
	bodies: Query<Entity, (With<Health>, Or<(With<Player>, With<Npc>)>, Without<SpiritTint>)>,
) {
	for entity in &bodies {
		commands.entity(entity).insert(SpiritTint::default());
	}
}

pub(crate) fn resolve_spirit_palette(
	body: Entity,
	tints: &Query<&SpiritTint>,
) -> ExplosionPalette {
	tints
		.get(body)
		.map(|tint| tint.palette())
		.unwrap_or_else(|_| ExplosionPalette::spirit_burst(DEFAULT_SPIRIT_TINT))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn spirit_burst_tracks_tint() {
		let cyan = SpiritTint(DEFAULT_SPIRIT_TINT).palette();
		let rose = SpiritTint(Color::srgb(1.0, 0.22, 0.55)).palette();
		assert!(rose.fire_hot.red > cyan.fire_hot.red + 0.15);
		assert!(cyan.fire_hot.blue > rose.fire_hot.blue + 0.1);
	}
}
