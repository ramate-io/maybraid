//! Emitter readiness, instance clocks, animation, and cleanup.

use bevy::prelude::*;
use bevy_hanabi::prelude::{CompiledParticleEffect, EffectSpawner, EffectSystems, SpawnerSettings};

use crate::lobe_instances::sync_lobe_instance_buffer;
use crate::lobes::{lobe_transform, stamp_lobe_materials, VfxLobe};
use crate::spawn::{
	realize_layer, VfxEmitter, VfxEmitterArmed, VfxEmitterBurst, VfxFlash, VfxInstance, VfxLayerLife,
	VfxPendingLayers,
};

/// Mark GPU-ready emitters. Burst is deferred to [`gate_vfx_instances`].
pub fn arm_vfx_emitters(
	mut commands: Commands,
	emitters: Query<(Entity, &CompiledParticleEffect), (With<VfxEmitter>, Without<VfxEmitterArmed>)>,
) {
	for (entity, compiled) in &emitters {
		if compiled.is_ready() {
			commands.entity(entity).insert(VfxEmitterArmed);
		}
	}
}

/// Common start: once every immediate emitter is ready, burst and release the clocks.
/// Already-armed instances also burst late layers as they become ready.
pub fn gate_vfx_instances(
	mut commands: Commands,
	children: Query<&Children>,
	emitters: Query<(&VfxEmitter, Has<VfxEmitterArmed>, Has<VfxEmitterBurst>)>,
	mut spawners: Query<Option<&mut EffectSpawner>, With<VfxEmitter>>,
	mut instances: Query<(Entity, &mut VfxInstance)>,
) {
	for (entity, mut instance) in &mut instances {
		if !instance.armed {
			if !instance_emitters_ready(entity, &children, &emitters) {
				continue;
			}
			instance.armed = true;
			instance.age = 0.0;
		}
		burst_child_emitters(entity, &children, &emitters, &mut spawners, &mut commands);
	}
}

pub fn tick_vfx_instances(
	mut commands: Commands,
	time: Res<Time>,
	lives: Query<&VfxLayerLife>,
	children: Query<&Children>,
	mut instances: Query<(Entity, &mut VfxInstance, &mut VfxPendingLayers)>,
) {
	let dt = time.delta_secs();
	for (entity, mut instance, mut pending) in &mut instances {
		if !instance.armed {
			continue;
		}
		instance.age += dt * instance.playback;
		let spawn = pending.spawn.clone();
		let mut remain = Vec::new();
		for layer in pending.layers.drain(..) {
			if instance.age + 1e-4 >= layer.delay {
				let actual_start = instance.age;
				instance.duration = instance.duration.max(actual_start + layer.part_lifetime());
				realize_layer(&mut commands, entity, &layer, &spawn);
			} else {
				remain.push(layer);
			}
		}
		pending.layers = remain;
		if pending.layers.is_empty() && layers_finished(entity, &children, &lives) {
			if instance.age >= instance.duration {
				commands.entity(entity).try_despawn();
			}
		}
	}
}

pub fn tick_vfx_flashes(
	mut commands: Commands,
	time: Res<Time>,
	child_of: Query<&ChildOf>,
	instances: Query<&VfxInstance>,
	mut flashes: Query<(Entity, &mut VfxFlash, &mut PointLight, Option<&mut VfxLayerLife>)>,
) {
	let dt = time.delta_secs();
	for (entity, mut flash, mut light, life) in &mut flashes {
		if !ancestor_armed(entity, &child_of, &instances) {
			light.intensity = 0.0;
			continue;
		}
		flash.age += dt * flash.playback;
		if let Some(mut life) = life {
			life.age = flash.age;
		}
		let t = if flash.fade > 1e-4 { (flash.age / flash.fade).clamp(0.0, 1.0) } else { 1.0 };
		light.intensity = flash.peak * (1.0 - t);
		if t >= 1.0 {
			commands.entity(entity).try_despawn();
		}
	}
}

pub fn tick_vfx_lobes(
	time: Res<Time>,
	child_of: Query<&ChildOf>,
	instances: Query<&VfxInstance>,
	mut lobes: Query<(
		Entity,
		&mut VfxLobe,
		&mut Transform,
		&mut Visibility,
		Option<&mut VfxLayerLife>,
	)>,
) {
	let dt = time.delta_secs();
	for (entity, mut lobe, mut transform, mut visibility, life) in &mut lobes {
		if !ancestor_armed(entity, &child_of, &instances) {
			*visibility = Visibility::Hidden;
			continue;
		}
		*visibility = Visibility::Inherited;
		lobe.age += dt * lobe.playback;
		if let Some(mut life) = life {
			life.age = lobe.age;
		}
		*transform = lobe_transform(&lobe.spec, lobe.age);
	}
}

pub fn tick_vfx_layer_lives(
	time: Res<Time>,
	armed: Query<Has<VfxEmitterArmed>, With<VfxEmitter>>,
	mut lives: Query<(Entity, &mut VfxLayerLife), With<VfxEmitter>>,
) {
	let dt = time.delta_secs();
	for (entity, mut life) in &mut lives {
		if life.waiting_for_emitter {
			if armed.get(entity).ok() != Some(true) {
				continue;
			}
			life.waiting_for_emitter = false;
			life.age = 0.0;
		}
		life.age += dt * life.playback;
	}
}

pub fn vfx_lifecycle_plugin(app: &mut App) {
	app.add_systems(
		PostUpdate,
		(arm_vfx_emitters, gate_vfx_instances)
			.chain()
			.before(EffectSystems::TickSpawners),
	)
	.add_systems(
		Update,
		(
			stamp_lobe_materials,
			tick_vfx_instances,
			tick_vfx_flashes,
			tick_vfx_lobes,
			sync_lobe_instance_buffer,
			tick_vfx_layer_lives,
		)
			.chain(),
	);
}

fn instance_emitters_ready(
	entity: Entity,
	children: &Query<&Children>,
	emitters: &Query<(&VfxEmitter, Has<VfxEmitterArmed>, Has<VfxEmitterBurst>)>,
) -> bool {
	let Ok(kids) = children.get(entity) else {
		return true;
	};
	for child in kids.iter() {
		let Ok((_, armed, _)) = emitters.get(child) else {
			continue;
		};
		if !armed {
			return false;
		}
	}
	true
}

fn burst_child_emitters(
	entity: Entity,
	children: &Query<&Children>,
	emitters: &Query<(&VfxEmitter, Has<VfxEmitterArmed>, Has<VfxEmitterBurst>)>,
	spawners: &mut Query<Option<&mut EffectSpawner>, With<VfxEmitter>>,
	commands: &mut Commands,
) {
	let Ok(kids) = children.get(entity) else {
		return;
	};
	for child in kids.iter() {
		let Ok((emitter, armed, burst)) = emitters.get(child) else {
			continue;
		};
		if !armed || burst {
			continue;
		}
		let settings = SpawnerSettings::once(emitter.count.into());
		if let Ok(Some(mut spawner)) = spawners.get_mut(child) {
			spawner.settings = settings;
			spawner.reset();
			spawner.active = true;
		} else {
			commands.entity(child).insert(EffectSpawner::new(&settings));
		}
		commands.entity(child).insert(VfxEmitterBurst);
	}
}

fn layers_finished(
	entity: Entity,
	children: &Query<&Children>,
	lives: &Query<&VfxLayerLife>,
) -> bool {
	let Ok(kids) = children.get(entity) else {
		return true;
	};
	let mut saw = false;
	for child in kids.iter() {
		if let Ok(life) = lives.get(child) {
			saw = true;
			if life.waiting_for_emitter || life.age + 1e-3 < life.duration {
				return false;
			}
		}
		if let Ok(grand) = children.get(child) {
			for g in grand.iter() {
				if let Ok(life) = lives.get(g) {
					saw = true;
					if life.waiting_for_emitter || life.age + 1e-3 < life.duration {
						return false;
					}
				}
			}
		}
	}
	saw
}

pub fn ancestor_armed(
	entity: Entity,
	child_of: &Query<&ChildOf>,
	instances: &Query<&VfxInstance>,
) -> bool {
	let mut current = entity;
	for _ in 0..4 {
		let Ok(parent) = child_of.get(current) else {
			return true;
		};
		if let Ok(instance) = instances.get(parent.parent()) {
			return instance.armed;
		}
		current = parent.parent();
	}
	true
}
