//! World-specific contact behavior for the controlled character.

use avian3d::prelude::{CollisionHooks, ContactPair};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::Player;

/// Controlled movement owns traction; physical contacts only block penetration.
#[derive(SystemParam)]
pub(crate) struct WorldCollisionHooks<'w, 's> {
	players: Query<'w, 's, (), With<Player>>,
}

impl CollisionHooks for WorldCollisionHooks<'_, '_> {
	fn modify_contacts(&self, contacts: &mut ContactPair, _commands: &mut Commands) -> bool {
		let player_contact = self.players.contains(contacts.collider1)
			|| self.players.contains(contacts.collider2)
			|| contacts.body1.is_some_and(|body| self.players.contains(body))
			|| contacts.body2.is_some_and(|body| self.players.contains(body));
		if player_contact {
			for manifold in &mut contacts.manifolds {
				manifold.friction = 0.0;
			}
		}
		true
	}
}
