//! Shared instance ownership and finite lifecycle for body, dynamic clouds and grains.
pub mod assets;
pub mod cache;
pub mod flow;
pub mod interaction;
pub mod interactive;

pub mod render;
pub mod simulation;
use crate::{
	composition::{CombinedLayers, CombinedOptions, CombinedPart, CombinedRole},
	membership::VfxMemberOf,
	parts::{
		cartoon_wave::{motion::Wave, Geometry},
		combined::{assets::CombinedAsset, simulation::Simulation},
		packaged::StaticVolume,
	},
	spawn::{VfxInstance, VfxLayerLife, VfxSpawn},
};
use bevy::{
	asset::{LoadState, RecursiveDependencyLoadState},
	prelude::*,
	transform::TransformSystems,
};
#[derive(Component)]
pub struct CombinedLayerState {
	pub part: CombinedPart,
}
#[derive(Component)]
pub struct CombinedState {
	pub assets: Handle<CombinedAsset>,
	pub simulation: Simulation,
	pub options: CombinedOptions,
	pub body_time: f32,
	pub backlog: f64,
	pub ready: bool,
	roles: u8,
}
impl CombinedState {
	pub fn configure(&mut self, options: CombinedOptions) {
		self.options = options;
		self.simulation.scenario = options.scenario;
	}
	pub fn enabled(&self, role: CombinedRole) -> bool {
		let selected = match self.options.layers {
			CombinedLayers::All => true,
			CombinedLayers::Body => role == CombinedRole::Body,
			CombinedLayers::Clouds => role == CombinedRole::Clouds,
			CombinedLayers::Grains => role == CombinedRole::Grains,
		};
		selected && self.roles & role.mask() != 0
	}
}
impl CombinedRole {
	fn mask(self) -> u8 {
		match self {
			Self::Body => 1,
			Self::Clouds => 2,
			Self::Grains => 4,
		}
	}
}
impl CombinedPart {
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
			CombinedLayerState { part: self.clone() },
			VfxLayerLife {
				age: 0.,
				duration: self.options.motion.duration(),
				playback: spawn.clamped_playback(),
				waiting_for_emitter: true,
			},
		));
	}
}
pub struct CombinedPlugin;
impl Plugin for CombinedPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<interaction::InteractionScene>()
			.init_asset::<CombinedAsset>()
			.init_asset_loader::<assets::CombinedLoader>()
			.init_resource::<Geometry>();
		render::plugin(app);
		app.add_systems(Update, tick.after(crate::lifecycle::tick_vfx_instances));
		app.add_systems(
			PostUpdate,
			prepare
				.after(TransformSystems::Propagate)
				.before(crate::lifecycle::gate_vfx_instances),
		);
		app.add_systems(PostUpdate, render::compose.after(prepare));
	}
}
fn prepare(
	cartoon: Query<&crate::parts::cartoon_flow::CartoonFlow>,
	mut interaction: Option<ResMut<interaction::InteractionScene>>,
	mut commands: Commands,
	server: Res<AssetServer>,
	assets: Res<Assets<CombinedAsset>>,
	volumes: Res<Assets<StaticVolume>>,
	images: Res<Assets<Image>>,
	meshes: Res<Assets<Mesh>>,
	mut geometry: ResMut<Geometry>,
	instances: Query<(&VfxInstance, &GlobalTransform)>,
	mut states: Query<&mut CombinedState>,
	mut layers: Query<(&VfxMemberOf, &CombinedLayerState, &mut VfxLayerLife)>,
) {
	let mut roots = layers.iter().map(|(m, _, _)| m.root()).collect::<Vec<_>>();
	roots.sort();
	roots.dedup();
	// Existing admissions keep their slots even when the entity allocator reuses a
	// lower index for a new request. Reject overflow requests, never live owners.
	roots.sort_by_key(|root| (!states.contains(*root), *root));
	let bundle = roots.first().and_then(|root| {
		layers
			.iter()
			.find(|(member, _, _)| member.root() == *root)
			.map(|(_, l, _)| l.part.assets.id())
	});
	let mut interactive_admitted =
		cartoon.iter().any(|s| s.ready && s.contact && s.region.is_none());
	for (index, root) in roots.into_iter().enumerate() {
		if index >= 4 {
			warn!("Combined event rejected: four concurrent events already admitted");
			commands.entity(root).try_despawn();
			continue;
		}
		let Some(part) =
			layers.iter().find(|(m, _, _)| m.root() == root).map(|(_, l, _)| l.part.clone())
		else {
			continue;
		};
		if part.options.motion.interactive() {
			if interactive_admitted {
				warn!("Interactive dust rejected: one proxy validation event at a time");
				commands.entity(root).try_despawn();
				continue;
			}
			interactive_admitted = true;
		}
		if Some(part.assets.id()) != bundle {
			warn!("Combined event rejected: only one shared density bundle per scene is supported");
			commands.entity(root).try_despawn();
			continue;
		}
		let roles = layers
			.iter()
			.filter(|(m, _, _)| m.root() == root)
			.fold(0, |mask, (_, l, _)| mask | l.part.role.mask());
		if let Ok(mut state) = states.get_mut(root) {
			state.roles = roles;
			continue;
		}
		if server.get_load_states(part.assets.id()).is_some_and(|(s, _, d)| {
			matches!(s, LoadState::Failed(_))
				|| matches!(d, RecursiveDependencyLoadState::Failed(_))
		}) {
			warn!("Combined dependency failed; cancelling event");
			commands.entity(root).try_despawn();
			continue;
		}
		let Some(asset) = assets.get(&part.assets) else {
			continue;
		};
		if !asset
			.bodies
			.iter()
			.chain(asset.clouds.iter())
			.all(|h| volumes.get(h).is_some_and(|v| images.contains(&v.image)))
		{
			continue;
		}
		if !asset.meshes.iter().all(|h| meshes.contains(h)) {
			continue;
		}
		let Some(rest) = geometry.load(&asset.meshes, &meshes) else {
			warn!("Combined body geometry invalid; cancelling event");
			commands.entity(root).try_despawn();
			continue;
		};
		let Ok((instance, global)) = instances.get(root) else {
			continue;
		};
		let mut config = asset.config;
		config.seed = instance.seed as u32;
		let mut simulation =
			Simulation::new(Wave::new(rest), config, asset.manifest.anchors, part.options.scenario);
		simulation.placement = global.to_matrix().as_dmat4();
		if part.options.motion.interactive() {
			if let Some(scene) = interaction.as_deref_mut() {
				scene.rebase_manual();
				scene.compact_landing = false;
				scene.cartoon_wave_timing = false;
			}
			simulation.interactive = Some(interactive::Interactive::new(
				part.options.motion == crate::composition::CombinedMotion::InteractiveContinuous,
			));
		}
		if part.options.motion == crate::composition::CombinedMotion::FlowGuided {
			if let Some(i) = &mut simulation.interactive {
				i.flow = Some(flow::Flow::new(
					interaction.as_deref().map_or(bevy::math::DVec3::ZERO, |s| s.wind),
				));
			}
		}
		commands.entity(root).insert(CombinedState {
			assets: part.assets,
			simulation,
			options: part.options,
			body_time: 0.,
			backlog: 0.,
			ready: false,
			roles,
		});
		for (member, _, mut life) in &mut layers {
			if member.root() == root {
				life.waiting_for_emitter = false;
			}
		}
	}
}
pub fn tick(
	clock: Option<Res<Time<Virtual>>>,
	scene: Option<Res<interaction::InteractionScene>>,
	mut states: Query<(Entity, &VfxInstance, &GlobalTransform, &mut CombinedState)>,
	mut layers: Query<(&VfxMemberOf, &mut VfxLayerLife), With<CombinedLayerState>>,
) {
	for (root, instance, global, mut state) in &mut states {
		if !instance.armed {
			continue;
		}
		state.ready = true;
		if state.options.motion.interactive() && clock.as_ref().is_some_and(|t| t.is_paused()) {
			continue;
		}
		state.simulation.placement = global.to_matrix().as_dmat4();
		let duration = state.options.motion.duration() as f64;
		state.backlog = if state.simulation.interactive.is_some() {
			let fallback = interaction::InteractionScene::default();
			state.simulation.advance_interactive(
				(instance.age as f64).min(duration),
				scene.as_deref().unwrap_or(&fallback),
			)
		} else {
			state.simulation.advance_to((instance.age as f64).min(4.7))
		};
		// At most one tick of pose/particle phase difference when caught up. A retained
		// backlog instead displays one coherent simulation time until catch-up finishes.
		state.body_time = if state.simulation.interactive.is_some() || state.backlog > 0. {
			state.simulation.time() as f32
		} else {
			instance.age.min(4.)
		};
		let age = state.simulation.time() as f32;
		for (member, mut life) in &mut layers {
			if member.root() == root {
				life.age = age;
			}
		}
	}
}
#[cfg(test)]
mod tests;
