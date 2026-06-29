//! Preview configuration and spawning.
//!
//! Commands update [`ConceptPreviewConfig`]. This module resolves that config via
//! `crozon-characters` and spawns Bevy scenes from the resulting assembly.

use bevy::prelude::*;
use crozon_characters::{
	assembly::{CharacterPartSlot, ResolvedCharacterAssembly},
	species::{braidman::BraidmanConfig, SpeciesConfig},
	ResolvedCharacterPart, SkinTarget, SocketRig,
};

use crate::skinning::{
	BoneMap, CharacterPart, CharacterRig, CharacterRigRole, NeedsPoseApply, NeedsSkinRemap,
	NeedsSocketPlacement, PartRigRef,
};

#[derive(Resource, Debug, Clone, PartialEq)]
pub enum ConceptPreviewConfig {
	Braidman(BraidmanConfig),
}

impl Default for ConceptPreviewConfig {
	fn default() -> Self {
		Self::braidman(BraidmanConfig::default_preview())
	}
}

impl ConceptPreviewConfig {
	pub fn braidman(config: BraidmanConfig) -> Self {
		Self::Braidman(config)
	}

	pub fn resolve(&self) -> ResolvedCharacterAssembly {
		match self {
			Self::Braidman(config) => config.resolve(),
		}
	}

	pub fn status_label(&self) -> String {
		match self {
			Self::Braidman(config) => config.status_label(),
		}
	}

	pub fn sync_key(&self) -> String {
		match self {
			Self::Braidman(config) => config.sync_key(),
		}
	}
}

#[derive(Resource, Default)]
pub struct ConceptPreviewSyncState {
	key: String,
}

#[derive(Component)]
pub struct ConceptPreviewRoot;

pub fn sync_preview(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	config: Res<ConceptPreviewConfig>,
	mut sync_state: ResMut<ConceptPreviewSyncState>,
	roots: Query<Entity, With<ConceptPreviewRoot>>,
) {
	let key = config.sync_key();
	if sync_state.key == key {
		return;
	}
	sync_state.key.clone_from(&key);

	for entity in &roots {
		commands.entity(entity).despawn();
	}

	let assembly = config.resolve();
	PreviewSpawner::new(&mut commands, &asset_server, assembly).spawn();
}

struct PreviewSpawner<'w, 's, 'a> {
	commands: &'a mut Commands<'w, 's>,
	asset_server: &'a AssetServer,
	assembly: ResolvedCharacterAssembly,
}

impl<'w, 's, 'a> PreviewSpawner<'w, 's, 'a> {
	fn new(
		commands: &'a mut Commands<'w, 's>,
		asset_server: &'a AssetServer,
		assembly: ResolvedCharacterAssembly,
	) -> Self {
		Self { commands, asset_server, assembly }
	}

	fn spawn(mut self) {
		let body_rig = self.spawn_body_rig();
		let mut head_rig = None;

		let parts = self.assembly.parts.clone();
		for part in parts {
			if part.slot == CharacterPartSlot::HeadRig {
				head_rig = self.spawn_head_rig(body_rig, &part);
				continue;
			}
			self.spawn_part(body_rig, head_rig, &part);
		}
	}

	fn spawn_body_rig(&mut self) -> Entity {
		self.commands
			.spawn((
				SceneRoot(self.asset_server.load(
					GltfAssetLabel::Scene(0).from_asset(self.assembly.body_rig.path.as_str()),
				)),
				CharacterRig { role: CharacterRigRole::Body },
				BoneMap::default(),
				NeedsPoseApply { pose: self.assembly.pose.clone() },
				ConceptPreviewRoot,
				Transform::IDENTITY,
				Name::new(format!("{}_body_rig", self.assembly.label)),
			))
			.id()
	}

	fn spawn_head_rig(&mut self, body_rig: Entity, part: &ResolvedCharacterPart) -> Option<Entity> {
		let entity = self
			.commands
			.spawn((
				SceneRoot(
					self.asset_server
						.load(GltfAssetLabel::Scene(0).from_asset(part.asset.path.as_str())),
				),
				CharacterRig { role: CharacterRigRole::Head },
				CharacterPart { slot: part.slot },
				BoneMap::default(),
				ConceptPreviewRoot,
				part.asset.normalization.transform(),
				Name::new(format!("character_{:?}", part.slot)),
			))
			.id();

		if let Some(socket) = part.socket {
			self.commands.entity(entity).insert(NeedsSocketPlacement {
				rig_root: body_rig,
				socket_bone: socket.bone,
				local_transform: socket.local_transform,
			});
		}

		Some(entity)
	}

	fn spawn_part(
		&mut self,
		body_rig: Entity,
		head_rig: Option<Entity>,
		part: &ResolvedCharacterPart,
	) {
		// debug: disable head mesh for now
		if part.slot == CharacterPartSlot::HeadMesh {
			return;
		}

		let entity = self
			.commands
			.spawn((
				SceneRoot(
					self.asset_server
						.load(GltfAssetLabel::Scene(0).from_asset(part.asset.path.as_str())),
				),
				CharacterPart { slot: part.slot },
				ConceptPreviewRoot,
				part.asset.normalization.transform(),
				Name::new(format!("character_{:?}", part.slot)),
			))
			.id();

		if let Some(rig_root) = self.skin_target_rig(body_rig, head_rig, part.skin_target) {
			self.commands.entity(entity).insert((PartRigRef { rig_root }, NeedsSkinRemap));
		}

		if let Some(socket) = part.socket {
			if let Some(rig_root) = self.socket_rig(body_rig, head_rig, socket.rig) {
				self.commands.entity(entity).insert(NeedsSocketPlacement {
					rig_root,
					socket_bone: socket.bone,
					local_transform: socket.local_transform,
				});
			}
		}
	}

	fn skin_target_rig(
		&self,
		body_rig: Entity,
		head_rig: Option<Entity>,
		target: SkinTarget,
	) -> Option<Entity> {
		match target {
			SkinTarget::BodyRig => Some(body_rig),
			SkinTarget::HeadRig => head_rig,
			SkinTarget::OwnRig | SkinTarget::None => None,
		}
	}

	fn socket_rig(
		&self,
		body_rig: Entity,
		head_rig: Option<Entity>,
		target: SocketRig,
	) -> Option<Entity> {
		match target {
			SocketRig::Body => Some(body_rig),
			SocketRig::Head => head_rig,
		}
	}
}
