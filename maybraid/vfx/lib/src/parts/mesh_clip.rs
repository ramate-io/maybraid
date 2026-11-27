//! Shared offline-authored mesh pose clips with opaque palette/dissolve surfaces.
//! No effect-specific motion, contacts or random simulation runs here.
pub mod asset;
mod material;
use crate::{
	composition::MeshClipPart,
	membership::VfxMemberOf,
	spawn::{VfxInstance, VfxLayerLife, VfxSpawn},
	VfxLoadFailure,
};
pub use asset::{MeshClip, MeshClipManifest};
use bevy::{
	asset::{LoadState, RecursiveDependencyLoadState},
	prelude::*,
};
pub use material::{ClipAppearance, ClipMaterial};
#[derive(Component)]
pub struct MeshClipPlayer {
	pub path: String,
	pub appearance: ClipAppearance,
	pub clip: Option<Handle<MeshClip>>,
	initialized: bool,
	started: bool,
}
#[derive(Component)]
pub struct ClipBody {
	pub track: usize,
	pub player: Entity,
}
impl MeshClipPart {
	pub(crate) fn spawn(
		&self,
		commands: &mut Commands,
		parent: Entity,
		transform: Transform,
		spawn: &VfxSpawn,
	) -> Entity {
		commands
			.spawn((
				ChildOf(parent),
				VfxMemberOf(parent),
				transform,
				Visibility::Inherited,
				MeshClipPlayer {
					path: self.path.clone(),
					appearance: ClipAppearance::from_spawn(spawn),
					clip: None,
					initialized: false,
					started: false,
				},
				VfxLayerLife {
					age: 0.,
					duration: 0.,
					playback: spawn.clamped_playback(),
					waiting_for_emitter: true,
				},
			))
			.id()
	}
}
pub struct MeshClipPlugin;
impl Plugin for MeshClipPlugin {
	fn build(&self, app: &mut App) {
		app.init_asset::<MeshClip>()
			.init_asset_loader::<asset::MeshClipLoader>()
			.add_plugins(material::ClipMaterialPlugin)
			.add_systems(
				Update,
				(tick_players, tick_bodies).chain().before(crate::lifecycle::tick_vfx_instances),
			);
	}
}
fn tick_players(
	mut commands: Commands,
	time: Res<Time>,
	server: Res<AssetServer>,
	assets: Res<Assets<MeshClip>>,
	meshes: Res<Assets<Mesh>>,
	mut materials: ResMut<Assets<ClipMaterial>>,
	instances: Query<&VfxInstance>,
	mut failures: MessageWriter<VfxLoadFailure>,
	mut players: Query<(Entity, &VfxMemberOf, &mut MeshClipPlayer, &mut VfxLayerLife)>,
) {
	for (entity, member, mut player, mut life) in &mut players {
		let path = player.path.clone();
		let handle = player.clip.get_or_insert_with(|| server.load(path)).clone();
		let error = match server.get_load_states(handle.id()) {
			Some((LoadState::Failed(e), _, _)) => Some(e.to_string()),
			Some((_, _, RecursiveDependencyLoadState::Failed(e))) => Some(e.to_string()),
			_ => None,
		};
		if let Some(error) = error {
			let f = VfxLoadFailure {
				root: member.root(),
				effect: instances.get(member.root()).map(|i| i.name.clone()).unwrap_or_default(),
				layer: "mesh clip".into(),
				path: player.path.clone(),
				error,
			};
			warn!("{f}");
			failures.write(f);
			commands.entity(member.root()).try_despawn();
			continue;
		}
		let Some(clip) = assets.get(&handle) else { continue };
		if !server.is_loaded_with_dependencies(handle.id())
			|| !clip.meshes.iter().all(|m| meshes.contains(m))
		{
			continue;
		}
		life.duration = clip.manifest.duration;
		if !player.initialized {
			for (track, t) in clip.manifest.tracks.iter().enumerate() {
				commands.spawn((
					ChildOf(entity),
					VfxMemberOf(member.root()),
					Mesh3d(clip.meshes[track].clone()),
					MeshMaterial3d(materials.add(ClipMaterial::with_appearance(
						t.seed,
						&clip.manifest.surface,
						player.appearance,
					))),
					Transform::IDENTITY,
					Visibility::Hidden,
					ClipBody { track, player: entity },
				));
			}
			player.initialized = true;
			life.waiting_for_emitter = false;
		}
		if !member.instance_armed(&instances) {
			continue;
		}
		if player.started {
			life.age += time.delta_secs() * life.playback;
		}
		player.started = true;
	}
}
fn tick_bodies(
	players: Query<(&MeshClipPlayer, &VfxLayerLife, &VfxMemberOf)>,
	instances: Query<&VfxInstance>,
	assets: Res<Assets<MeshClip>>,
	mut materials: ResMut<Assets<ClipMaterial>>,
	mut bodies: Query<(&ClipBody, &MeshMaterial3d<ClipMaterial>, &mut Transform, &mut Visibility)>,
) {
	for (body, handle, mut transform, mut visible) in &mut bodies {
		let Ok((player, life, member)) = players.get(body.player) else {
			continue;
		};
		let Some(clip) = player.clip.as_ref().and_then(|h| assets.get(h)) else {
			continue;
		};
		let state = if member.instance_armed(&instances) {
			clip.manifest.sample(body.track, life.age)
		} else {
			None
		};
		if let Some(state) = state {
			let pose = state.transform();
			if *transform != pose {
				*transform = pose;
			}
			if *visible != Visibility::Inherited {
				*visible = Visibility::Inherited;
			}
			if materials.get(&handle.0).is_some_and(|m| m.cutoff.x != state.cutoff) {
				if let Some(mut material) = materials.get_mut(&handle.0) {
					material.cutoff.x = state.cutoff;
				}
			}
		} else {
			if *visible != Visibility::Hidden {
				*visible = Visibility::Hidden;
			}
		}
	}
}

#[cfg(test)]
mod tests;
