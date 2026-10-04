//! Public spawn API, delayed layers, and hierarchy cleanup.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy_hanabi::prelude::{EffectMaterial, EffectSpawner, ParticleEffect, SpawnerSettings};

use crate::composition::{EffectDefinition, EffectLayer, EffectPart};

pub const MAX_INTENSITY: f32 = 2.0;
pub const MIN_SCALE: f32 = 0.25;
pub const MAX_SCALE: f32 = 4.0;

/// Per-instance overrides. Never mutates a shared Hanabi asset.
#[derive(Clone, Debug)]
pub struct VfxSpawn {
	pub transform: Transform,
	pub scale: f32,
	pub intensity: f32,
	pub tint: Option<Color>,
	pub seed: u64,
}

impl Default for VfxSpawn {
	fn default() -> Self {
		Self { transform: Transform::IDENTITY, scale: 1.0, intensity: 1.0, tint: None, seed: 0 }
	}
}

impl VfxSpawn {
	pub fn clamped_scale(&self) -> f32 {
		self.scale.clamp(MIN_SCALE, MAX_SCALE)
	}

	pub fn clamped_intensity(&self) -> f32 {
		self.intensity.clamp(0.25, MAX_INTENSITY)
	}
}

/// Root of one spawned instance.
#[derive(Component, Debug)]
pub struct VfxInstance {
	pub name: String,
	pub age: f32,
	pub duration: f32,
}

/// Layers waiting for their start delay relative to the root.
#[derive(Component, Debug)]
pub struct VfxPendingLayers {
	pub layers: Vec<EffectLayer>,
	pub spawn: VfxSpawn,
}

/// Light pulse fading from peak to zero.
#[derive(Component, Debug)]
pub struct VfxFlash {
	pub age: f32,
	pub fade: f32,
	pub peak: f32,
}

/// Spawn a named definition at `spawn.transform`. Returns the root entity.
pub fn spawn_vfx(
	commands: &mut Commands,
	definition: &EffectDefinition,
	spawn: VfxSpawn,
) -> Entity {
	let spawn =
		VfxSpawn { scale: spawn.clamped_scale(), intensity: spawn.clamped_intensity(), ..spawn };
	let duration = definition.duration();
	let mut pending = Vec::new();
	let mut immediate = Vec::new();
	for layer in &definition.layers {
		if layer.delay <= 0.0 {
			immediate.push(layer.clone());
		} else {
			pending.push(layer.clone());
		}
	}

	let root_transform = Transform {
		translation: spawn.transform.translation,
		rotation: spawn.transform.rotation,
		scale: Vec3::splat(spawn.clamped_scale()),
	};
	let root = commands
		.spawn((
			Name::new(format!("vfx-{}", definition.name)),
			root_transform,
			Visibility::default(),
			VfxInstance { name: definition.name.clone(), age: 0.0, duration },
			VfxPendingLayers { layers: pending, spawn: spawn.clone() },
			NotShadowCaster,
		))
		.id();

	for layer in immediate {
		realize_layer(commands, root, &layer, &spawn);
	}
	root
}

/// `Commands` extension: `commands.spawn_vfx(&library.firey_explosion, VfxSpawn { .. })`.
pub trait SpawnVfxExt {
	fn spawn_vfx(&mut self, definition: &EffectDefinition, spawn: VfxSpawn) -> Entity;
}

impl SpawnVfxExt for Commands<'_, '_> {
	fn spawn_vfx(&mut self, definition: &EffectDefinition, spawn: VfxSpawn) -> Entity {
		spawn_vfx(self, definition, spawn)
	}
}

pub fn tick_vfx_instances(
	mut commands: Commands,
	time: Res<Time>,
	mut instances: Query<(Entity, &mut VfxInstance, &mut VfxPendingLayers)>,
) {
	let dt = time.delta_secs();
	for (entity, mut instance, mut pending) in &mut instances {
		instance.age += dt;
		let spawn = pending.spawn.clone();
		let mut remain = Vec::new();
		for layer in pending.layers.drain(..) {
			if instance.age + 1e-4 >= layer.delay {
				realize_layer(&mut commands, entity, &layer, &spawn);
			} else {
				remain.push(layer);
			}
		}
		pending.layers = remain;
		if instance.age >= instance.duration && pending.layers.is_empty() {
			commands.entity(entity).try_despawn();
		}
	}
}

pub fn tick_vfx_flashes(
	mut commands: Commands,
	time: Res<Time>,
	mut flashes: Query<(Entity, &mut VfxFlash, &mut PointLight)>,
) {
	let dt = time.delta_secs();
	for (entity, mut flash, mut light) in &mut flashes {
		flash.age += dt;
		let t = if flash.fade > 1e-4 { (flash.age / flash.fade).clamp(0.0, 1.0) } else { 1.0 };
		light.intensity = flash.peak * (1.0 - t);
		if t >= 1.0 {
			commands.entity(entity).try_despawn();
		}
	}
}

fn realize_layer(commands: &mut Commands, parent: Entity, layer: &EffectLayer, spawn: &VfxSpawn) {
	// Emitters simulate in local space and inherit the root's world pose + scale.
	let transform = Transform {
		translation: layer.transform.translation,
		rotation: layer.transform.rotation,
		scale: Vec3::splat(layer.scale),
	};
	match &layer.part {
		EffectPart::Particle(part) => {
			let count = (part.count * spawn.clamped_intensity()).clamp(1.0, part.capacity as f32);
			let mut entity = commands.spawn((
				Name::new(format!("vfx-layer-{}", part.name)),
				ChildOf(parent),
				transform,
				Visibility::Inherited,
				ParticleEffect::new(part.effect.clone()),
				EffectSpawner::new(&SpawnerSettings::once(count.into())),
				NotShadowCaster,
			));
			if !part.images.is_empty() {
				entity.insert(EffectMaterial { images: part.images.clone() });
			}
		}
		EffectPart::Light(pulse) => {
			let peak = pulse.peak_intensity * spawn.clamped_intensity();
			let color = spawn.tint.unwrap_or(pulse.color);
			let range = pulse.range * spawn.clamped_scale() * layer.scale;
			commands.spawn((
				Name::new("vfx-layer-flash"),
				ChildOf(parent),
				transform,
				PointLight {
					color,
					intensity: peak,
					range,
					shadow_maps_enabled: false,
					..default()
				},
				VfxFlash { age: 0.0, fade: pulse.fade, peak },
			));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::composition::{EffectLayer, LightPulse};
	use crate::layers::flash::FLASH_FADE;

	#[test]
	fn spawn_clamps_scale_and_intensity() {
		let spawn = VfxSpawn { scale: 99.0, intensity: 0.01, ..default() };
		assert_eq!(spawn.clamped_scale(), MAX_SCALE);
		assert_eq!(spawn.clamped_intensity(), 0.25);
	}

	#[test]
	fn light_layer_duration_is_fade() {
		let layer = EffectLayer::light(LightPulse {
			color: Color::WHITE,
			peak_intensity: 1.0,
			range: 1.0,
			fade: FLASH_FADE,
		});
		assert!((layer.duration() - FLASH_FADE).abs() < 1e-4);
	}
}
