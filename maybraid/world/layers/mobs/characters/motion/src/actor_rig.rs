//! Gameplay actor → typed body-rig binding for clip hosts.
//!
//! [`AnimRefRoot`] lives on the body rig. Combat / attack cues name the gameplay
//! actor, which is a distinct parent after an actor/visual split. This 1:1
//! relationship is the explicit binding; consumers must not copy [`AnimRefRoot`]
//! onto the actor or search children by name.

use bevy::prelude::*;
use rigs::AssemblyRoot;

use crate::clip::AnimRefRoot;
use crate::rig::{CharacterRig, CharacterRigRole};

/// Gameplay actor → body rig that carries [`AnimRefRoot`].
///
/// Inserting this on the actor stamps [`BodyRigOf`] on the rig.
#[derive(Component, Debug, Clone, Copy)]
#[relationship(relationship_target = BodyRigOf)]
pub struct ActorBodyRig {
	#[relationship]
	pub rig: Entity,
}

impl ActorBodyRig {
	pub fn of(rig: Entity) -> Self {
		Self { rig }
	}
}

/// Body-rig side of [`ActorBodyRig`].
#[derive(Component, Debug)]
#[relationship_target(relationship = ActorBodyRig)]
pub struct BodyRigOf(Entity);

impl BodyRigOf {
	pub fn actor(&self) -> Entity {
		self.0
	}
}

/// Bind each unbound body clip host to the gameplay actor above its visual.
///
/// The visual is the [`AssemblyRoot`]. Its parent is the physics / combat actor
/// when spawn split the two (Bikanwarda unit-scale root + scaled visual child).
/// Standalone visuals with no parent are left unbound.
pub fn bind_actor_body_rigs(
	mut commands: Commands,
	unbound: Query<(Entity, &CharacterRig), (With<AnimRefRoot>, Without<BodyRigOf>)>,
	child_of: Query<&ChildOf>,
	visuals: Query<(), With<AssemblyRoot>>,
	bound_actors: Query<(), With<ActorBodyRig>>,
) {
	for (rig, character_rig) in &unbound {
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}
		let Some(visual) = assembly_root_of(rig, &child_of, &visuals) else {
			continue;
		};
		let Ok(parent) = child_of.get(visual) else {
			continue;
		};
		let actor = parent.parent();
		if bound_actors.contains(actor) {
			continue;
		}
		commands.entity(actor).insert(ActorBodyRig::of(rig));
	}
}

fn assembly_root_of(
	mut entity: Entity,
	child_of: &Query<&ChildOf>,
	visuals: &Query<(), With<AssemblyRoot>>,
) -> Option<Entity> {
	loop {
		if visuals.contains(entity) {
			return Some(entity);
		}
		entity = child_of.get(entity).ok()?.parent();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::clip::{AnimClip, AnimRef};
	use rigs::AssemblyHost;

	fn stamp_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_systems(PreUpdate, bind_actor_body_rigs);
		app
	}

	#[test]
	fn stamps_actor_to_body_rig_under_visual_child() {
		let mut app = stamp_app();
		let actor = app.world_mut().spawn_empty().id();
		let visual = app.world_mut().spawn((AssemblyRoot, ChildOf(actor))).id();
		let prior = AnimRef::new(AnimClip::quadruped_run());
		let rig = app
			.world_mut()
			.spawn((
				AssemblyHost,
				CharacterRig { role: CharacterRigRole::Body, skeleton: Default::default() },
				AnimRefRoot(prior),
				ChildOf(visual),
			))
			.id();

		app.update();

		let binding =
			app.world().get::<ActorBodyRig>(actor).expect("actor should bind the body rig");
		assert_eq!(binding.rig, rig);
		assert_eq!(app.world().get::<BodyRigOf>(rig).map(BodyRigOf::actor), Some(actor));
		assert!(app.world().get::<AnimRefRoot>(actor).is_none());
		let anim = app.world().get::<AnimRefRoot>(rig).expect("clip stays on the body rig");
		assert_eq!(anim.0.clip.id(), prior.clip.id());
	}

	#[test]
	fn does_not_bind_when_visual_is_the_root() {
		let mut app = stamp_app();
		let visual = app.world_mut().spawn(AssemblyRoot).id();
		app.world_mut().spawn((
			CharacterRig { role: CharacterRigRole::Body, skeleton: Default::default() },
			AnimRefRoot::default(),
			ChildOf(visual),
		));

		app.update();

		assert_eq!(app.world_mut().query::<&ActorBodyRig>().iter(app.world()).count(), 0);
	}

	#[test]
	fn ignores_non_body_clip_hosts() {
		let mut app = stamp_app();
		let actor = app.world_mut().spawn_empty().id();
		let visual = app.world_mut().spawn((AssemblyRoot, ChildOf(actor))).id();
		app.world_mut().spawn((
			CharacterRig { role: CharacterRigRole::Head, skeleton: Default::default() },
			AnimRefRoot::default(),
			ChildOf(visual),
		));

		app.update();

		assert!(app.world().get::<ActorBodyRig>(actor).is_none());
	}
}
