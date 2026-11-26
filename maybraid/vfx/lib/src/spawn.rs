//! Public spawn API and layer dispatch.

use bevy::ecs::template::template;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use bevy::scene::{ResolveContext, ResolveSceneError, SceneDependencies};

use crate::composition::{EffectDefinition, EffectLayer, EffectPart, EffectRecipe};
use crate::membership::VfxMemberOf;
use crate::palette::ExplosionPalette;
use crate::seed;

pub const MAX_INTENSITY: f32 = 2.0;
pub const MIN_PLAYBACK: f32 = 0.25;
pub const MAX_PLAYBACK: f32 = 4.0;

/// Scalar world extent for billboard size and light range. Axis lengths also
/// handle mirrored, collapsed, and sheared transforms without decomposing rotation.
pub(crate) fn visual_scale(transform: &GlobalTransform) -> f32 {
	let axes = transform.affine().matrix3;
	axes.x_axis.length().max(axes.y_axis.length()).max(axes.z_axis.length())
}

/// Per-instance overrides. Never mutates a shared Hanabi asset.
#[derive(Clone, Debug)]
pub struct VfxSpawn {
	pub intensity: f32,
	/// 1.0 is the authored layer envelope. Higher plays the same ratios faster.
	pub playback: f32,
	/// Optional overall multiplier applied on top of [`Self::palette`].
	pub tint: Option<Color>,
	/// `None` picks a new seed. `Some` reproduces that instance.
	pub seed: Option<u64>,
	/// `None` uses [`ExplosionPalette::maybraid`].
	pub palette: Option<ExplosionPalette>,
}

impl Default for VfxSpawn {
	fn default() -> Self {
		Self { intensity: 1.0, playback: 1.0, tint: None, seed: None, palette: None }
	}
}

impl VfxSpawn {
	pub fn clamped_intensity(&self) -> f32 {
		self.intensity.clamp(0.25, MAX_INTENSITY)
	}

	pub fn clamped_playback(&self) -> f32 {
		self.playback.clamp(MIN_PLAYBACK, MAX_PLAYBACK)
	}

	pub fn tint_or_white(&self) -> Color {
		self.tint.unwrap_or(Color::WHITE)
	}

	pub fn resolved_seed(&self) -> u64 {
		self.seed.unwrap_or(0)
	}

	pub fn palette(&self) -> ExplosionPalette {
		self.palette.unwrap_or_default()
	}

	/// Clamp numeric overrides and fill a missing seed. Call once before spawn.
	pub fn resolved(self) -> Self {
		Self {
			intensity: self.clamped_intensity(),
			playback: self.clamped_playback(),
			tint: self.tint,
			seed: Some(self.seed.unwrap_or_else(seed::generate)),
			palette: self.palette,
		}
	}
}

/// Spawn payload: typed recipe plus per-instance overrides.
#[derive(Clone, Debug)]
pub struct Effect<T> {
	pub recipe: T,
	pub spawn: VfxSpawn,
}

impl<T> Effect<T> {
	pub fn new(recipe: T) -> Self {
		Self { recipe, spawn: VfxSpawn::default() }
	}

	pub fn with_spawn(mut self, spawn: VfxSpawn) -> Self {
		self.spawn = spawn;
		self
	}
}

impl<T: EffectRecipe + Component + Clone> Scene for Effect<T> {
	fn resolve(
		self,
		context: &mut ResolveContext,
		scene: &mut bevy::scene::ResolvedScene,
	) -> Result<(), ResolveSceneError> {
		let definition = self.recipe.effect_definition();
		let recipe = self.recipe;
		(Vfx { definition, spawn: self.spawn }, bsn! { template(move |_ctx| Ok(recipe.clone())) })
			.resolve(context, scene)
	}

	fn register_dependencies(&self, dependencies: &mut SceneDependencies) {
		Vfx { definition: self.recipe.effect_definition(), spawn: self.spawn.clone() }
			.register_dependencies(dependencies);
	}
}

/// Flattened definition path. Does not insert a typed recipe identity.
#[derive(Clone, Debug)]
pub struct Vfx {
	pub definition: EffectDefinition,
	pub spawn: VfxSpawn,
}

impl Scene for Vfx {
	fn resolve(
		self,
		context: &mut ResolveContext,
		scene: &mut bevy::scene::ResolvedScene,
	) -> Result<(), ResolveSceneError> {
		let prepared = PreparedVfx::new(&self.definition, self.spawn);
		let name = prepared.name.clone();
		let instance = prepared.instance.clone();
		let pending = prepared.pending_component.clone();
		let spawn = prepared.spawn.clone();
		let immediate = prepared.immediate.clone();
		bsn! {
			template(move |_ctx| Ok(name.clone()))
			Visibility::Visible
			template(move |ctx| {
				let parent = ctx.entity.id();
				let spawn = spawn.clone();
				let immediate = immediate.clone();
				ctx.entity.world_scope(|world| {
					let mut commands = world.commands();
					for layer in &immediate {
						realize_layer(&mut commands, parent, layer, &spawn);
					}
					world.flush();
				});
				Ok(instance.clone())
			})
			template(move |_ctx| Ok(pending.clone()))
			NotShadowCaster
		}
		.resolve(context, scene)
	}
}

/// Root of one spawned instance.
#[derive(Clone, Component, Debug)]
#[require(Transform, crate::membership::VfxInstanceMembers)]
pub struct VfxInstance {
	pub name: String,
	pub age: f32,
	pub duration: f32,
	pub playback: f32,
	pub seed: u64,
	/// False until immediate particle emitters are ready to burst.
	pub armed: bool,
}

/// Layers waiting for their start delay relative to the root.
#[derive(Clone, Component, Debug)]
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
	/// Authored range, scaled from the propagated layer transform.
	pub range: f32,
	pub playback: f32,
}

/// Particle layer waiting for [`bevy_hanabi::CompiledParticleEffect::is_ready`].
#[derive(Component, Debug)]
pub struct VfxEmitter {
	pub count: f32,
}

#[derive(Component, Debug)]
pub struct VfxEmitterArmed;

/// Spawner has been reset for this instance's start gate.
#[derive(Component, Debug)]
pub struct VfxEmitterBurst;

/// Actual start clock for one realized layer. Cleanup uses this, not planned delay.
#[derive(Clone, Component, Debug)]
pub struct VfxLayerLife {
	pub age: f32,
	pub duration: f32,
	pub playback: f32,
	pub waiting_for_emitter: bool,
}

struct PreparedVfx {
	name: Name,
	instance: VfxInstance,
	pending_component: VfxPendingLayers,
	spawn: VfxSpawn,
	immediate: Vec<EffectLayer>,
}

impl PreparedVfx {
	fn new(definition: &EffectDefinition, spawn: VfxSpawn) -> Self {
		let spawn = spawn.resolved();
		let mut pending = Vec::new();
		let mut immediate = Vec::new();
		for layer in &definition.layers {
			if layer.delay <= 0.0 {
				immediate.push(layer.clone());
			} else {
				pending.push(layer.clone());
			}
		}
		Self {
			name: Name::new(format!("vfx-{}", definition.name)),
			instance: VfxInstance {
				name: definition.name.clone(),
				age: 0.0,
				duration: definition.duration(),
				playback: spawn.clamped_playback(),
				seed: spawn.resolved_seed(),
				armed: false,
			},
			pending_component: VfxPendingLayers { layers: pending, spawn: spawn.clone() },
			spawn,
			immediate,
		}
	}
}

/// Spawn a definition at identity. Insert a `Transform` on the returned root to place it.
pub fn spawn_vfx(
	commands: &mut Commands,
	definition: &EffectDefinition,
	spawn: VfxSpawn,
) -> Entity {
	let prepared = PreparedVfx::new(definition, spawn);
	let spawn = prepared.spawn.clone();
	let root = commands
		.spawn((
			prepared.name,
			Visibility::Visible,
			prepared.instance,
			prepared.pending_component,
			NotShadowCaster,
		))
		.id();
	for layer in prepared.immediate {
		realize_layer(commands, root, &layer, &spawn);
	}
	root
}

/// `Commands` extension for flattened definitions. Place the returned root with `Transform`.
pub trait SpawnVfxExt {
	fn spawn_vfx(&mut self, definition: &EffectDefinition, spawn: VfxSpawn) -> Entity;
}

impl SpawnVfxExt for Commands<'_, '_> {
	fn spawn_vfx(&mut self, definition: &EffectDefinition, spawn: VfxSpawn) -> Entity {
		spawn_vfx(self, definition, spawn)
	}
}

pub fn realize_layer(
	commands: &mut Commands,
	parent: Entity,
	layer: &EffectLayer,
	spawn: &VfxSpawn,
) {
	let transform = Transform {
		translation: layer.transform.translation,
		rotation: layer.transform.rotation,
		scale: layer.transform.scale * layer.scale,
	};
	match &layer.part {
		EffectPart::Flipbook(part) => part.spawn(commands, parent, transform, spawn),
		EffectPart::Particle(part) => part.spawn(commands, parent, transform, spawn),
		EffectPart::Light(pulse) => {
			let peak = pulse.peak_intensity * spawn.clamped_intensity();
			let color = ExplosionPalette::color(spawn.palette().with_tint(spawn.tint).flash);
			let range = pulse.range;
			commands.spawn((
				Name::new("vfx-layer-flash"),
				ChildOf(parent),
				VfxMemberOf(parent),
				transform,
				PointLight {
					color,
					intensity: 0.0,
					range,
					shadow_maps_enabled: false,
					..default()
				},
				VfxFlash {
					age: 0.0,
					fade: pulse.fade,
					peak,
					playback: spawn.clamped_playback(),
					range,
				},
				VfxLayerLife {
					age: 0.0,
					duration: pulse.fade,
					playback: spawn.clamped_playback(),
					waiting_for_emitter: false,
				},
			));
		}
		EffectPart::Mesh(part) => part.spawn(commands, parent, transform, spawn),
		EffectPart::Soundwave(part) => part.spawn(commands, parent, transform, spawn),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::composition::{EffectLayer, LightPulse, ParticleShade};
	use crate::layers::flash::FLASH_FADE;
	use crate::parts::particle::{
		effect_properties, PROP_COLOR0, PROP_PLAYBACK, PROP_SEED, PROP_TINT,
	};

	#[test]
	fn spawn_clamps_intensity_and_playback() {
		let spawn = VfxSpawn { intensity: 0.01, playback: 0.01, ..default() };
		assert_eq!(spawn.clamped_intensity(), 0.25);
		assert_eq!(spawn.clamped_playback(), MIN_PLAYBACK);
		let fast = VfxSpawn { playback: 99.0, ..default() };
		assert_eq!(fast.clamped_playback(), MAX_PLAYBACK);
	}

	#[test]
	fn omitted_seed_is_filled_once() {
		let a = VfxSpawn::default().resolved();
		let b = VfxSpawn::default().resolved();
		assert!(a.seed.is_some());
		assert_ne!(a.seed, b.seed);
		let same = VfxSpawn { seed: Some(11), ..default() }.resolved();
		assert_eq!(same.seed, Some(11));
	}

	#[test]
	fn playback_preserves_layer_ratios() {
		let authored: f32 = 0.05 + 2.4;
		let playback: f32 = 2.0;
		assert!(((authored / playback) - 1.225).abs() < 1e-4);
		assert!((0.05 / authored - (0.05 / playback) / (authored / playback)).abs() < 1e-6);
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

	#[test]
	fn delayed_layer_extends_cleanup_from_actual_start() {
		let planned: f32 = 0.05 + 2.4;
		let actual_start = 0.08;
		let duration = planned.max(actual_start + 2.4);
		assert!((duration - 2.48).abs() < 1e-4);
	}

	#[test]
	fn tint_and_palette_reach_particle_properties() {
		let spawn = VfxSpawn {
			tint: Some(Color::srgb(0.2, 0.8, 1.0)),
			seed: Some(7),
			palette: Some(ExplosionPalette {
				fire_hot: LinearRgba::new(0.1, 0.3, 0.9, 1.0),
				..default()
			}),
			playback: 2.0,
			..default()
		}
		.resolved();
		let props = effect_properties(&spawn, 1.0, ParticleShade::Fire);
		assert!(props.get_stored(PROP_TINT).is_some());
		assert!(props.get_stored(PROP_SEED).is_some());
		assert!(props.get_stored(PROP_PLAYBACK).is_some());
		assert_eq!(props.get_stored(PROP_COLOR0), Some(Vec4::new(0.1, 0.3, 0.9, 1.0).into()));
	}
}

#[cfg(test)]
mod placement_tests {
	use crate::composition::{
		EffectDefinition, EffectLayer, LightPulse, ParticlePart, ParticleShade,
	};
	use crate::lifecycle::sync_flash_range;
	use crate::parts::particle::{sync_particle_scale, PROP_SCALE};
	use crate::spawn::{Vfx, VfxEmitter, VfxFlash};
	use bevy::prelude::*;
	use bevy::scene::{prelude::bsn, ScenePlugin};
	use bevy_hanabi::EffectProperties;

	#[test]
	fn particle_size_and_light_range_follow_composed_scale_without_changing_placement(
	) -> Result<(), Box<dyn std::error::Error>> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin, TransformPlugin))
			.add_systems(
				PostUpdate,
				(sync_particle_scale, sync_flash_range)
					.after(bevy::transform::TransformSystems::Propagate),
			);
		let particle = ParticlePart {
			name: "sizing".into(),
			effect: default(),
			images: Vec::new(),
			count: 1.0,
			capacity: 1,
			max_lifetime: 1.0,
			shade: ParticleShade::Fire,
		};
		let local = Transform::from_xyz(1.0, 0.0, 0.0).with_scale(Vec3::splat(2.0));
		let definition = EffectDefinition::new(
			"sizing",
			[
				EffectLayer::particle(particle).with_transform(local).with_scale(0.5),
				EffectLayer::light(LightPulse {
					color: Color::WHITE,
					peak_intensity: 1.0,
					range: 4.0,
					fade: 1.0,
				})
				.with_transform(local)
				.with_scale(0.5),
			],
		);
		let placement = Transform::from_xyz(2.0, 3.0, 4.0).with_scale(Vec3::splat(13.0));
		let root = app
			.world_mut()
			.commands()
			.spawn_scene(bsn! {
				~{placement}
				@{Vfx { definition, spawn: default() }}
			})
			.id();
		app.world_mut().flush();
		app.update();
		assert_eq!(app.world().get::<Transform>(root), Some(&placement));
		let properties = app
			.world_mut()
			.query_filtered::<&EffectProperties, With<VfxEmitter>>()
			.single(app.world())?;
		assert_eq!(properties.get_stored(PROP_SCALE), Some(13.0.into()));
		let light = app
			.world_mut()
			.query_filtered::<&PointLight, With<VfxFlash>>()
			.single(app.world())?;
		assert_eq!(light.range, 52.0);
		// A later ECS placement edit is reflected without mutating the authored root.
		app.world_mut()
			.entity_mut(root)
			.get_mut::<Transform>()
			.ok_or("missing placement")?
			.scale = Vec3::splat(3.0);
		app.update();
		let properties = app
			.world_mut()
			.query_filtered::<&EffectProperties, With<VfxEmitter>>()
			.single(app.world())?;
		assert_eq!(properties.get_stored(PROP_SCALE), Some(3.0.into()));
		let light = app
			.world_mut()
			.query_filtered::<&PointLight, With<VfxFlash>>()
			.single(app.world())?;
		assert_eq!(light.range, 12.0);
		Ok(())
	}
}
