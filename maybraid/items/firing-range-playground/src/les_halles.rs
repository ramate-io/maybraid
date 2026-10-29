//! Fit and spawn a small mixed-use Les Halles development for multi-storey pathing.

use bevy::prelude::*;
use les_halles_arena::{spawn_development, PLAYGROUND_SEED};
use movement_intelligence_richmond::CirculationStairwell;

pub use les_halles_arena::LesHallesSpawn;

pub(crate) fn setup_les_halles(mut commands: Commands) {
	spawn_development(&mut commands, PLAYGROUND_SEED);
}

pub(crate) fn draw_circulation_gizmos(mut gizmos: Gizmos, links: Query<&CirculationStairwell>) {
	for link in &links {
		let mut prev: Option<Vec3> = None;
		for p in &link.polyline {
			let lifted = *p + Vec3::Y * 0.15;
			gizmos.sphere(Isometry3d::from_translation(lifted), 0.12, Color::srgb(0.95, 0.85, 0.2));
			if let Some(a) = prev {
				gizmos.line(a, lifted, Color::srgb(0.95, 0.85, 0.2));
			}
			prev = Some(lifted);
		}
		gizmos.sphere(
			Isometry3d::from_translation(link.mouth + Vec3::Y * 0.2),
			0.18,
			Color::srgb(0.2, 0.9, 0.35),
		);
		gizmos.sphere(
			Isometry3d::from_translation(link.landing + Vec3::Y * 0.2),
			0.18,
			Color::srgb(0.9, 0.25, 0.3),
		);
	}
}
