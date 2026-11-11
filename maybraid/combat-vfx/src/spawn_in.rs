//! Celestial wisps converge when a combat body gains its character visual.

use bevy::prelude::*;
use character_ragdoll::Corpse;
use characters::CharacterRoot;
use damage::Health;
use crate::tint::resolve_spirit_palette;
use maybraid_vfx::{EffectRecipe, SpawnVfxExt, VfxInstance, VfxLibrary, VfxSpawn};
use player::{Npc, Player};

/// Character mesh stays hidden until wisps converge.
#[derive(Component, Clone, Debug)]
pub struct SpiritMaterialize {
	pub vfx: Entity,
	pub reveal_at: f32,
}

/// At most one spawn effect per body (lubikan replacement bodies get a new entity).
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct SpiritSpawnPlayed;

/// Wisps and soft light duration; character reveals partway through.
pub const SPIRIT_SPAWN_SECS: f32 = 1.65;
pub const SPIRIT_SPAWN_REVEAL: f32 = 0.82;

pub(crate) fn begin_spirit_materialize(
	time: Res<Time>,
	mut commands: Commands,
	library: Option<Res<VfxLibrary>>,
	added: Query<(Entity, &GlobalTransform, &ChildOf), Added<CharacterRoot>>,
	bodies: Query<(Entity, &GlobalTransform, Has<SpiritSpawnPlayed>), (Or<(With<Player>, With<Npc>)>, With<Health>)>,
	tints: Query<&crate::SpiritTint>,
	corpses: Query<(), With<Corpse>>,
	dissolving: Query<(), With<super::spirit::SpiritDissolveStarted>>,
) {
	let Some(library) = library else {
		return;
	};
	let now = time.elapsed_secs();
	for (visual, transform, child_of) in &added {
		if corpses.get(visual).is_ok() || dissolving.get(visual).is_ok() {
			continue;
		}
		let Ok((body, _, played)) = bodies.get(child_of.parent()) else {
			continue;
		};
		if played {
			continue;
		}
		commands.entity(body).insert(SpiritSpawnPlayed);
		let reveal_at = now + SPIRIT_SPAWN_REVEAL;
		let vfx = commands.spawn_vfx(
			&library.spirit_spawn.effect_definition(),
			VfxSpawn {
				transform: Transform::from_translation(
					transform.translation() + Vec3::new(0.0, 0.35, 0.0),
				),
				scale: 1.35,
				intensity: 1.65,
				palette: Some(resolve_spirit_palette(body, &tints)),
				seed: Some(body.to_bits() ^ visual.to_bits() ^ 0x5a17_5ca4),
				..default()
			},
		);
		commands.entity(visual).insert((
			SpiritMaterialize { vfx, reveal_at },
			Visibility::Hidden,
		));
	}
}

pub(crate) fn tick_spirit_materialize(
	time: Res<Time>,
	mut commands: Commands,
	mut materializing: Query<(Entity, &SpiritMaterialize, &mut Visibility)>,
	instances: Query<&VfxInstance>,
) {
	let now = time.elapsed_secs();
	for (visual, materialize, mut visibility) in &mut materializing {
		if now >= materialize.reveal_at {
			*visibility = Visibility::Inherited;
		}
		let finished = instances.get(materialize.vfx).map_or(true, |instance| {
			!instance.armed || instance.age >= instance.duration
		});
		if finished {
			*visibility = Visibility::Inherited;
			commands.entity(visual).remove::<SpiritMaterialize>();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_vfx::{
		Dust, DustLayer, EffectDefinition, FieryExplosion, FlipbookAsset, FlipbookPart, LobeKind,
		MeshPart, OozeBurst, ParticlePart, ParticleShade, SpiritDissolve, SpiritSpawn, VfxFlipbooks,
		VfxInstance, VfxLibrary,
	};

	fn stub_library() -> VfxLibrary {
		let particle = ParticlePart {
			name: "stub".into(),
			effect: Default::default(),
			images: Vec::new(),
			count: 1.0,
			capacity: 1,
			max_lifetime: SPIRIT_SPAWN_SECS,
			shade: ParticleShade::Spirit,
		};
		let mesh = MeshPart {
			name: "stub".into(),
			mesh: Handle::default(),
			kind: LobeKind::Flash,
			lobes: Vec::new(),
			duration: SPIRIT_SPAWN_SECS,
		};
		VfxLibrary {
			dust: Dust::new(DustLayer::new(FlipbookPart::default())),
			flipbooks: VfxFlipbooks {
				fire: FlipbookAsset {
					image: Handle::default(),
					grid: UVec2::ONE,
					frame_count: 1,
					playback: 1.0,
				},
				smoke: FlipbookAsset {
					image: Handle::default(),
					grid: UVec2::ONE,
					frame_count: 1,
					playback: 1.0,
				},
				spark: Handle::default(),
				ooze: Handle::default(),
			},
			flash: EffectDefinition::new("flash", []),
			fireball: EffectDefinition::new("fireball", []),
			smoke: EffectDefinition::new("smoke", []),
			sparks: EffectDefinition::new("sparks", []),
			ooze_burst: OozeBurst::new(mesh.clone(), particle.clone(), particle.clone()),
			spirit_dissolve: SpiritDissolve::new(
				mesh.clone(),
				particle.clone(),
				particle.clone(),
				particle.clone(),
			),
			spirit_spawn: SpiritSpawn::new(
				mesh.clone(),
				particle.clone(),
				particle.clone(),
				particle.clone(),
			),
			fiery_explosion: FieryExplosion::new(
				mesh.clone(),
				mesh.clone(),
				particle.clone(),
				mesh.clone(),
				particle.clone(),
				particle,
			),
		}
	}

	#[test]
	fn materialize_reveals_and_cleans_up() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_systems(Update, tick_spirit_materialize);
		let vfx = app.world_mut().spawn_empty().id();
		let visual = app
			.world_mut()
			.spawn((
				SpiritMaterialize { vfx, reveal_at: f32::MAX },
				Visibility::Hidden,
			))
			.id();
		app.world_mut().entity_mut(vfx).insert(VfxInstance {
			name: maybraid_vfx::SPIRIT_SPAWN.into(),
			age: SPIRIT_SPAWN_SECS,
			duration: SPIRIT_SPAWN_SECS,
			playback: 1.0,
			seed: 0,
			armed: true,
		});
		app.world_mut()
			.get_mut::<SpiritMaterialize>(visual)
			.unwrap()
			.reveal_at = 0.0;
		app.update();
		assert!(
			app.world().get::<SpiritMaterialize>(visual).is_none(),
			"finished spawn should remove marker"
		);
		assert_eq!(
			app.world().get::<Visibility>(visual),
			Some(&Visibility::Inherited),
			"character should be visible"
		);
	}
}
