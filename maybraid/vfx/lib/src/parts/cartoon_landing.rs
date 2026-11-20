//! Reusable true-3D landing primitive; event time belongs to the shared host.
pub mod motion;
use crate::{
	composition::CartoonLandingPart,
	membership::VfxMemberOf,
	spawn::{VfxInstance, VfxLayerLife, VfxSpawn},
};
use bevy::{
	asset::embedded_asset, prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef,
};
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LandingMaterial {
	/// Landing: dissolve threshold in X. Wave: four deformation coefficients.
	#[uniform(0)]
	pub cutoff: Vec4,
	/// Landing hash seed in X; W=1 tags a wave (no dissolution).
	#[uniform(1)]
	pub seed: UVec4,
	#[uniform(2)]
	pub environment: crate::parts::cartoon_flow::SurfaceEnvironment,
}
impl LandingMaterial {
	/// Preserve the legacy selector and local coefficients independently of the environment.
	pub(crate) fn wave(coefficients: Vec4) -> Self {
		Self { cutoff: coefficients, seed: UVec4::new(0, 0, 0, 1), environment: default() }
	}
	pub(crate) fn set_wave_coefficients(&mut self, coefficients: Vec4) {
		self.cutoff = coefficients;
	}
}
impl Material for LandingMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/parts/cartoon_landing/surface.wesl")
			.into()
	}
	fn fragment_shader() -> ShaderRef {
		Self::vertex_shader()
	}
	fn enable_prepass() -> bool {
		false
	}
	fn enable_shadows() -> bool {
		false
	}
}
#[derive(Component)]
pub struct Landing {
	part: CartoonLandingPart,
	initialized: bool,
}
#[derive(Component)]
pub(crate) struct Body {
	spec: motion::Puff,
	material: Handle<LandingMaterial>,
}
impl CartoonLandingPart {
	pub fn load(server: &AssetServer) -> Self {
		Self {
			meshes: ["Shoulder", "Column", "Swept"]
				.map(|n| server.load(format!("vfx/dust/bodies/{n}.glb#Mesh0/Primitive0"))),
		}
	}
	pub(crate) fn spawn(
		&self,
		commands: &mut Commands,
		parent: Entity,
		transform: Transform,
		spawn: &VfxSpawn,
	) {
		commands.spawn((
			ChildOf(parent),
			VfxMemberOf(parent),
			transform,
			Visibility::Visible,
			Landing { part: self.clone(), initialized: false },
			VfxLayerLife {
				age: 0.,
				duration: 1.2,
				playback: spawn.clamped_playback(),
				waiting_for_emitter: true,
			},
		));
	}
}
pub struct LandingPlugin;
impl Plugin for LandingPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "cartoon_landing/surface.wesl");
		app.add_plugins(MaterialPlugin::<LandingMaterial>::default())
			.add_systems(Update, tick.after(crate::lifecycle::tick_vfx_instances));
	}
}
pub(crate) fn tick(
	mut commands: Commands,
	server: Res<AssetServer>,
	meshes: Res<Assets<Mesh>>,
	mut materials: ResMut<Assets<LandingMaterial>>,
	instances: Query<&VfxInstance>,
	mut layers: Query<(Entity, &VfxMemberOf, &mut Landing, &mut VfxLayerLife)>,
	mut bodies: Query<(Entity, &VfxMemberOf, &Body, &mut Transform, &mut Visibility)>,
) {
	// No eviction: reject excess events explicitly, before constructing their resources.
	let mut admitted = layers.iter().filter(|(_, _, l, _)| l.initialized).count();
	let mut order = layers.iter().map(|(e, _, _, _)| e).collect::<Vec<_>>();
	order.sort();
	for id in order {
		let Ok((entity, member, mut layer, mut life)) = layers.get_mut(id) else {
			continue;
		};
		let Ok(instance) = instances.get(member.root()) else {
			continue;
		};
		if !layer.initialized {
			if layer.part.meshes.iter().any(|h| {
				server
					.get_load_states(h.id())
					.is_some_and(|(state, _, _)| matches!(state, bevy::asset::LoadState::Failed(_)))
			}) {
				warn!("Cartoon landing mesh failed to load; cancelling event");
				commands.entity(member.root()).try_despawn();
				continue;
			}

			if admitted >= 16 {
				warn!("Cartoon landing event rejected: sixteen concurrent events already active");
				commands.entity(member.root()).try_despawn();
				continue;
			}
			if !layer.part.meshes.iter().all(|h| meshes.contains(h)) {
				continue;
			}
			for spec in motion::Puff::table(instance.seed as u32) {
				let material = materials.add(LandingMaterial {
					cutoff: Vec4::splat(-0.02),
					seed: UVec4::new(spec.seed, 0, 0, 0),
					environment: default(),
				});
				commands.spawn((
					ChildOf(entity),
					VfxMemberOf(member.root()),
					Mesh3d(layer.part.meshes[spec.variant].clone()),
					MeshMaterial3d(material.clone()),
					Transform::IDENTITY,
					Visibility::Hidden,
					Body { spec, material },
				));
			}
			layer.initialized = true;
			admitted += 1;
			life.waiting_for_emitter = false;
		}
		life.age = instance.age;
	}
	for (_, member, body, mut transform, mut visible) in &mut bodies {
		let Ok(instance) = instances.get(member.root()) else {
			continue;
		};
		if !instance.armed {
			*visible = Visibility::Hidden;
			continue;
		}
		if let Some(state) = body.spec.state(instance.age as f64) {
			*transform = state.transform;
			*visible = Visibility::Visible;
			if let Some(mut m) = materials.get_mut(&body.material) {
				m.cutoff.x = state.threshold;
			}
		} else {
			*visible = Visibility::Hidden;
		}
	}
}
#[cfg(test)]
mod tests {
	use crate::{
		composition::{CartoonLandingPart, EffectDefinition, EffectLayer, EffectPart},
		lifecycle::{gate_vfx_instances, tick_vfx_instances},
		parts::cartoon_landing::{tick, Body, LandingMaterial},
		spawn::{spawn_vfx, VfxInstance, VfxSpawn},
	};
	use bevy::{asset::AssetPlugin, prelude::*, time::TimeUpdateStrategy};
	use std::time::Duration;
	pub(crate) fn fixture() -> (App, EffectDefinition) {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<LandingMaterial>()
			.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(10)))
			.add_systems(Update, (tick_vfx_instances, tick).chain())
			.add_systems(PostUpdate, gate_vfx_instances);
		let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Cuboid::default());
		let part = CartoonLandingPart { meshes: [mesh.clone(), mesh.clone(), mesh] };
		let def = EffectDefinition::new(
			"cartoon-landing",
			[EffectLayer {
				part: EffectPart::CartoonLanding(part),
				delay: 0.,
				transform: Transform::IDENTITY,
				scale: 1.,
			}],
		);
		(app, def)
	}
	pub(crate) fn spawn(app: &mut App, def: &EffectDefinition, seed: u64, speed: f32) -> Entity {
		let mut commands = app.world_mut().commands();
		let root = spawn_vfx(
			&mut commands,
			def,
			VfxSpawn { seed: Some(seed), playback: speed, ..default() },
		);
		app.world_mut().flush();
		root
	}
	#[test]
	fn pause_speed_reset_concurrency_cancel_completion() -> Result<(), Box<dyn std::error::Error>> {
		let (mut app, def) = fixture();
		let a = spawn(&mut app, &def, 13, 1.);
		let b = spawn(&mut app, &def, 37, 2.);
		for _ in 0..12 {
			app.update();
		}
		let age_a = app.world().get::<VfxInstance>(a).ok_or("a")?.age;
		let age_b = app.world().get::<VfxInstance>(b).ok_or("b")?.age;
		assert!((age_b - 2. * age_a).abs() < 1e-5);
		assert_eq!(app.world_mut().query::<&Body>().iter(app.world()).count(), 32);
		let handles = app
			.world_mut()
			.query::<&Body>()
			.iter(app.world())
			.map(|p| p.material.id())
			.collect::<std::collections::HashSet<_>>();
		assert_eq!(handles.len(), 32);
		app.world_mut().resource_mut::<Time<Virtual>>().pause();
		for _ in 0..20 {
			app.update();
		}
		assert_eq!(app.world().get::<VfxInstance>(a).ok_or("a")?.age, age_a);
		app.world_mut().despawn(a);
		app.update();
		assert_eq!(app.world_mut().query::<&Body>().iter(app.world()).count(), 16);
		app.world_mut().resource_mut::<Time<Virtual>>().unpause();
		let reset = spawn(&mut app, &def, 13, 1.);
		for _ in 0..12 {
			app.update();
		}
		assert!(app.world().get::<VfxInstance>(reset).ok_or("reset")?.age < 0.13);
		for _ in 0..150 {
			app.update();
		}
		assert!(app.world().get::<VfxInstance>(reset).is_none());
		assert!(app.world().get::<VfxInstance>(b).is_none());
		assert_eq!(app.world_mut().query::<&Body>().iter(app.world()).count(), 0);
		for _ in 0..5 {
			app.update();
		}
		assert_eq!(app.world().resource::<Assets<LandingMaterial>>().len(), 0);
		Ok(())
	}
}
#[cfg(test)]
mod budget_tests {
	use crate::parts::cartoon_landing::tests::{fixture, spawn};
	use crate::{parts::cartoon_landing::Body, spawn::VfxInstance};
	#[test]
	fn bounded_events_reject_without_eviction() {
		let (mut app, def) = fixture();
		for seed in 0..20 {
			spawn(&mut app, &def, seed, 1.);
		}
		app.update();
		app.update();
		assert_eq!(app.world_mut().query::<&VfxInstance>().iter(app.world()).count(), 16);
		assert_eq!(app.world_mut().query::<&Body>().iter(app.world()).count(), 256);
	}
}
