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
	bind_scales_ready, bone_map_ready, ActiveRigPose, BoneMap, CharacterPart, CharacterRig,
	CharacterRigRole, NeedsSkinRemap, NeedsSocketPlacement, PartRigRef, RigBindScales,
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

	/// Full config fingerprint — used for status text and command sync.
	pub fn sync_key(&self) -> String {
		match self {
			Self::Braidman { config, animation } => {
				format!("{} animation={animation:?}", config.sync_key())
			}
		}
	}

	/// Asset topology only. When this changes the preview GLTF scenes are respawned.
	pub fn spawn_key(&self) -> String {
		match self {
			Self::Braidman { config, .. } => format!(
				"body={:?} head={:?} eye={:?} nose={:?} mouth={:?} ear={:?} hair={:?} clothing={:?}",
				config.body,
				config.head,
				config.eye,
				config.nose,
				config.mouth,
				config.ear,
				config.hair,
				config.clothing,
			),
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
	live_key: String,
	spawn_key: String,
}

/// Skips part attachment/remap for one frame after a GLTF respawn so queued
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

/// Spawned hidden until the body rig bone map and bind scales are ready.
#[derive(Component)]
pub struct PreviewAwaitingReveal;

#[derive(Component, Clone, Copy)]
pub struct PreviewPartBaseTransform {
	normalization: Transform,
	socket: Option<Transform>,
}

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
	mut body_poses: Query<&mut ActiveRigPose, With<AnimatedBodyRig>>,
	mut parts: Query<(
		&CharacterPart,
		&mut PreviewAssetTarget,
		Option<&PreviewPartBaseTransform>,
		Option<&mut Transform>,
	)>,
	roots: Query<Entity, With<ConceptPreviewRoot>>,
) {
	let live_key = config.sync_key();
	let spawn_key = config.spawn_key();
	if sync_state.live_key == live_key {
		return;
	}

	let assembly = config.resolve();
	if sync_state.spawn_key == spawn_key {
		sync_state.live_key = live_key;
		sync_live_preview(&config, &assembly, &mut body_poses, &mut parts);
		return;
	}

	sync_state.live_key = live_key;
	sync_state.spawn_key.clone_from(&spawn_key);
	respawn_cooldown.frames_remaining = 1;

	for entity in &roots {
		commands.entity(entity).try_despawn();
	}

	PreviewSpawner::new(&mut commands, &asset_server, assembly, config.clone()).spawn();
}

fn sync_live_preview(
	config: &ConceptPreviewConfig,
	assembly: &ResolvedCharacterAssembly,
	body_poses: &mut Query<&mut ActiveRigPose, With<AnimatedBodyRig>>,
	parts: &mut Query<(
		&CharacterPart,
		&mut PreviewAssetTarget,
		Option<&PreviewPartBaseTransform>,
		Option<&mut Transform>,
	)>,
) {
	for mut pose in body_poses {
		pose.pose = assembly.pose.clone();
	}

	let ConceptPreviewConfig::Braidman { config: braidman, .. } = config;
	let sliders = braidman.sliders.clamped();
	for (part, mut target, base, transform) in parts {
		target.color = preview_color(braidman, target.target);
		let Some(base) = base else {
			continue;
		};
		let Some(mut transform) = transform else {
			continue;
		};
		if !has_feature_transform(part.slot) {
			continue;
		}
		let authored = base
			.normalization
			.mul_transform(sliders.feature_transform(part.slot));
		match base.socket {
			Some(socket) => {
				*transform = socket;
				transform.scale *= authored.scale;
				transform.rotation *= authored.rotation;
			}
			None => *transform = authored,
		}
	}
}

/// Reveal a respawned preview only after proportions have been applied once.
pub fn reveal_ready_preview(
	mut commands: Commands,
	pending: Query<Entity, With<PreviewAwaitingReveal>>,
	body_rigs: Query<(&BoneMap, &RigBindScales), With<AnimatedBodyRig>>,
) {
	let Ok((bone_map, bind_scales)) = body_rigs.single() else {
		return;
	};
	if !bone_map_ready(bone_map) || !bind_scales_ready(bind_scales, bone_map) {
		return;
	}
	for entity in &pending {
		commands.entity(entity).try_insert(Visibility::Inherited);
		commands.entity(entity).try_remove::<PreviewAwaitingReveal>();
	}
}

fn has_feature_transform(slot: CharacterPartSlot) -> bool {
	matches!(
		slot,
		CharacterPartSlot::EyeLeft
			| CharacterPartSlot::EyeRight
			| CharacterPartSlot::Nose
			| CharacterPartSlot::Mouth
			| CharacterPartSlot::EarLeft
			| CharacterPartSlot::EarRight
	)
}

fn preview_color(config: &BraidmanConfig, target: UiAssetTarget) -> BraidmanColor {
	let skin = config.colors.skin_color();
	match target {
		UiAssetTarget::Body(_) => config.colors.body,
		UiAssetTarget::Head(_) | UiAssetTarget::Nose(_) | UiAssetTarget::Ear(_) => skin,
		UiAssetTarget::Eye(_) => config.colors.eyes,
		UiAssetTarget::Mouth(_) => config.colors.mouth,
		UiAssetTarget::Hair(_) => config.colors.hair,
		UiAssetTarget::Clothing(clothing) => config.colors.clothing_color(clothing),
		UiAssetTarget::Animation(_) => BraidmanColor::Natural,
	}
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

		let parts = self.assembly.parts.clone();
		for part in parts {
			if part.slot == CharacterPartSlot::HeadRig {
				head_rig = self.spawn_head_rig(body_rig, &part);
				continue;
			}
			self.spawn_part(body_rig, head_rig, &part);
		}
	}

	fn part_transform(&self, part: &ResolvedCharacterPart) -> Transform {
		let ConceptPreviewConfig::Braidman { config, .. } = &self.config;
		let sliders = config.sliders.clamped();
		part.asset
			.normalization
			.transform()
			.mul_transform(sliders.feature_transform(part.slot))
	}

	fn part_base_transform(&self, part: &ResolvedCharacterPart) -> PreviewPartBaseTransform {
		PreviewPartBaseTransform {
			normalization: part.asset.normalization.transform(),
			socket: part.socket.map(|socket| socket.local_transform),
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
				ActiveRigPose { pose: self.assembly.pose.clone() },
				RigBindScales::default(),
				BodyRigBindTransform(Transform::IDENTITY),
				ConceptPreviewRoot,
				PreviewAwaitingReveal,
				Visibility::Hidden,
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
				PreviewAwaitingReveal,
				Visibility::Hidden,
				self.part_base_transform(part),
				self.part_transform(part),
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
		let entity = self
			.commands
			.spawn((
				SceneRoot(
					self.asset_server
						.load(GltfAssetLabel::Scene(0).from_asset(part.asset.path.as_str())),
				),
				CharacterPart { slot: part.slot },
				ConceptPreviewRoot,
				PreviewAwaitingReveal,
				Visibility::Hidden,
				self.part_base_transform(part),
				self.part_transform(part),
				self.preview_target(part),
				Name::new(format!("character_{:?}_{}", part.slot, part.asset.label)),
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

	fn preview_target(&self, part: &ResolvedCharacterPart) -> PreviewAssetTarget {
		let ConceptPreviewConfig::Braidman { config, .. } = &self.config;
		PreviewAssetTarget {
			target: match part.slot {
				CharacterPartSlot::BodyMesh => UiAssetTarget::Body(config.body),
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					UiAssetTarget::Head(config.head)
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					UiAssetTarget::Eye(config.eye)
				}
				CharacterPartSlot::Nose => UiAssetTarget::Nose(config.nose),
				CharacterPartSlot::Mouth => UiAssetTarget::Mouth(config.mouth),
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
					UiAssetTarget::Ear(config.ear)
				}
				CharacterPartSlot::Hair => UiAssetTarget::Hair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == part.asset.label)
					.map(UiAssetTarget::Clothing)
					.unwrap_or(UiAssetTarget::Head(config.head)),
			},
			color: preview_color(
				config,
				match part.slot {
					CharacterPartSlot::BodyMesh => UiAssetTarget::Body(config.body),
					CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
						UiAssetTarget::Head(config.head)
					}
					CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
						UiAssetTarget::Eye(config.eye)
					}
					CharacterPartSlot::Nose => UiAssetTarget::Nose(config.nose),
					CharacterPartSlot::Mouth => UiAssetTarget::Mouth(config.mouth),
					CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
						UiAssetTarget::Ear(config.ear)
					}
					CharacterPartSlot::Hair => UiAssetTarget::Hair(config.hair),
					CharacterPartSlot::Clothing => config
						.clothing
						.iter()
						.copied()
						.find(|clothing| clothing.label() == part.asset.label)
						.map(UiAssetTarget::Clothing)
						.unwrap_or(UiAssetTarget::Head(config.head)),
				},
			),
		}
	}
}
