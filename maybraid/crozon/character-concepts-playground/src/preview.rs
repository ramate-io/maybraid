//! Preview configuration and spawning.
//!
//! Commands update [`ConceptPreviewConfig`]. This module resolves that config via
//! `crozon-characters` and spawns Bevy scenes from the resulting assembly.

use bevy::prelude::*;
use crozon_characters::{
	assembly::{CharacterPartSlot, ResolvedCharacterAssembly},
	species::{
		braidman::{BraidmanColor, BraidmanConfig},
		SpeciesConfig,
	},
	ResolvedCharacterPart, SkinTarget, SocketRig,
};

use crate::animation::{AnimatedBodyRig, BodyRigBindTransform, ConceptAnimation};
use crate::skinning::{
	ActiveRigPose, BoneMap, CharacterPart, CharacterRig, CharacterRigRole, NeedsSkinRemap,
	NeedsSocketPlacement, PartRigRef, RigBindScales,
};
use crate::ui::UiAssetTarget;

#[derive(Resource, Debug, Clone, PartialEq)]
pub enum ConceptPreviewConfig {
	Braidman { config: BraidmanConfig, animation: ConceptAnimation },
}

impl Default for ConceptPreviewConfig {
	fn default() -> Self {
		Self::braidman(BraidmanConfig::default_preview())
	}
}

impl ConceptPreviewConfig {
	pub fn braidman(config: BraidmanConfig) -> Self {
		Self::Braidman { config, animation: ConceptAnimation::default() }
	}

	pub fn braidman_with_animation(config: BraidmanConfig, animation: ConceptAnimation) -> Self {
		Self::Braidman { config, animation }
	}

	pub fn resolve(&self) -> ResolvedCharacterAssembly {
		match self {
			Self::Braidman { config, .. } => config.resolve(),
		}
	}

	pub fn status_label(&self) -> String {
		match self {
			Self::Braidman { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
		}
	}

	pub fn sync_key(&self) -> String {
		match self {
			Self::Braidman { config, animation } => {
				format!("{} animation={animation:?}", config.sync_key())
			}
		}
	}

	pub const fn animation(&self) -> ConceptAnimation {
		match self {
			Self::Braidman { animation, .. } => *animation,
		}
	}
}

#[derive(Resource, Default)]
pub struct ConceptPreviewSyncState {
	key: String,
}

/// Skips preview mutation systems for one frame after a full respawn so queued
/// despawn commands are not racing inserts on the outgoing entities.
#[derive(Resource, Default)]
pub struct PreviewRespawnCooldown {
	pub frames_remaining: u8,
}

pub fn tick_preview_respawn_cooldown(mut cooldown: ResMut<PreviewRespawnCooldown>) {
	if cooldown.frames_remaining > 0 {
		cooldown.frames_remaining -= 1;
	}
}

pub fn preview_pass_ready(cooldown: Res<PreviewRespawnCooldown>) -> bool {
	cooldown.frames_remaining == 0
}

#[derive(Component)]
pub struct ConceptPreviewRoot;

#[derive(Component, Clone, Copy)]
pub struct PreviewAssetTarget {
	pub target: UiAssetTarget,
	pub color: BraidmanColor,
}

pub fn sync_preview(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	config: Res<ConceptPreviewConfig>,
	mut sync_state: ResMut<ConceptPreviewSyncState>,
	mut respawn_cooldown: ResMut<PreviewRespawnCooldown>,
	roots: Query<Entity, With<ConceptPreviewRoot>>,
) {
	let key = config.sync_key();
	if sync_state.key == key {
		return;
	}
	sync_state.key.clone_from(&key);
	respawn_cooldown.frames_remaining = 1;

	// Full respawn on any config change; fine for command-driven preview scale.
	for entity in &roots {
		commands.entity(entity).try_despawn();
	}

	let assembly = config.resolve();
	PreviewSpawner::new(&mut commands, &asset_server, assembly, config.clone()).spawn();
}

struct PreviewSpawner<'w, 's, 'a> {
	commands: &'a mut Commands<'w, 's>,
	asset_server: &'a AssetServer,
	assembly: ResolvedCharacterAssembly,
	config: ConceptPreviewConfig,
}

impl<'w, 's, 'a> PreviewSpawner<'w, 's, 'a> {
	fn new(
		commands: &'a mut Commands<'w, 's>,
		asset_server: &'a AssetServer,
		assembly: ResolvedCharacterAssembly,
		config: ConceptPreviewConfig,
	) -> Self {
		Self { commands, asset_server, assembly, config }
	}

	fn spawn(mut self) {
		let body_rig = self.spawn_body_rig();
		let mut head_rig = None;

		// Head rig must exist before features that skin or socket to it.
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
				AnimatedBodyRig,
				BoneMap::default(),
				// Pose maintenance runs on the body rig only in this pass.
				ActiveRigPose { pose: self.assembly.pose.clone() },
				RigBindScales::default(),
				BodyRigBindTransform(Transform::IDENTITY),
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
				self.preview_target(part),
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
		/*if part.slot == CharacterPartSlot::HeadMesh {
			return;
		}*/

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
				self.preview_target(part),
				Name::new(format!("character_{:?}_{}", part.slot, part.asset.label)),
			))
			.id();

		if let Some(rig_root) = self.skin_target_rig(body_rig, head_rig, part.skin_target) {
			// Deferred until bone map is populated after GLTF load.
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

	fn preview_target(&self, part: &ResolvedCharacterPart) -> PreviewAssetTarget {
		let ConceptPreviewConfig::Braidman { config, .. } = &self.config;
		match part.slot {
			CharacterPartSlot::BodyMesh => PreviewAssetTarget {
				target: UiAssetTarget::Body(config.body),
				color: config.colors.body,
			},
			CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => PreviewAssetTarget {
				target: UiAssetTarget::Head(config.head),
				color: config.colors.head,
			},
			CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => PreviewAssetTarget {
				target: UiAssetTarget::Eye(config.eye),
				color: config.colors.eyes,
			},
			CharacterPartSlot::Nose => PreviewAssetTarget {
				target: UiAssetTarget::Nose(config.nose),
				color: config.colors.nose,
			},
			CharacterPartSlot::Mouth => PreviewAssetTarget {
				target: UiAssetTarget::Mouth(config.mouth),
				color: config.colors.mouth,
			},
			CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => PreviewAssetTarget {
				target: UiAssetTarget::Ear(config.ear),
				color: config.colors.ears,
			},
			CharacterPartSlot::Hair => PreviewAssetTarget {
				target: UiAssetTarget::Hair(config.hair),
				color: config.colors.hair,
			},
			CharacterPartSlot::Clothing => match config
				.clothing
				.iter()
				.copied()
				.find(|clothing| clothing.label() == part.asset.label)
			{
				Some(clothing) => PreviewAssetTarget {
					target: UiAssetTarget::Clothing(clothing),
					color: config.colors.clothing_color(clothing),
				},
				None => PreviewAssetTarget {
					target: UiAssetTarget::Head(config.head),
					color: config.colors.head,
				},
			},
		}
	}
}
