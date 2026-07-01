//! Preview configuration and spawning.
//!
//! Commands update [`ConceptPreviewConfig`]. This module resolves that config via
//! `crozon-characters` and spawns Bevy scenes from the resulting assembly.

use bevy::prelude::*;
use crozon_characters::{
	assembly::{CharacterPartSlot, ResolvedCharacterAssembly},
	species::{
		braidman::BraidmanConfig,
		brodler::BrodlerConfig,
		SpeciesConfig,
	},
	ResolvedCharacterPart, SkinTarget, SocketRig,
};

use crate::animation::{AnimatedBodyRig, BodyRigBindTransform, ConceptAnimation};
use crate::preview_color::PreviewColor;
use crate::skinning::{
	bind_scales_ready, bone_map_ready, ActiveRigPose, BoneMap, CharacterPart, CharacterRig,
	CharacterRigRole, NeedsSkinRemap, NeedsSocketPlacement, PartRigRef, RigBindScales,
};
use crate::ui::UiAssetTarget;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ConceptSpecies {
	#[default]
	Braidman,
	Brodler,
}

impl ConceptSpecies {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Braidman => "braidman",
			Self::Brodler => "brodler",
		}
	}
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub enum ConceptPreviewConfig {
	Braidman { config: BraidmanConfig, animation: ConceptAnimation },
	Brodler { config: BrodlerConfig, animation: ConceptAnimation },
}

impl Default for ConceptPreviewConfig {
	fn default() -> Self {
		Self::default_for(ConceptSpecies::Braidman)
	}
}

impl ConceptPreviewConfig {
	pub fn default_for(species: ConceptSpecies) -> Self {
		match species {
			ConceptSpecies::Braidman => Self::braidman(BraidmanConfig::default_preview()),
			ConceptSpecies::Brodler => Self::brodler(BrodlerConfig::default_preview()),
		}
	}

	pub fn species(&self) -> ConceptSpecies {
		match self {
			Self::Braidman { .. } => ConceptSpecies::Braidman,
			Self::Brodler { .. } => ConceptSpecies::Brodler,
		}
	}

	pub fn braidman(config: BraidmanConfig) -> Self {
		Self::Braidman { config, animation: ConceptAnimation::default() }
	}

	pub fn braidman_with_animation(config: BraidmanConfig, animation: ConceptAnimation) -> Self {
		Self::Braidman { config, animation }
	}

	pub fn brodler(config: BrodlerConfig) -> Self {
		Self::Brodler { config, animation: ConceptAnimation::default() }
	}

	pub fn brodler_with_animation(config: BrodlerConfig, animation: ConceptAnimation) -> Self {
		Self::Brodler { config, animation }
	}

	pub fn resolve(&self) -> ResolvedCharacterAssembly {
		match self {
			Self::Braidman { config, .. } => config.resolve(),
			Self::Brodler { config, .. } => config.resolve(),
		}
	}

	pub fn status_label(&self) -> String {
		match self {
			Self::Braidman { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Brodler { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
		}
	}

	pub fn sync_key(&self) -> String {
		match self {
			Self::Braidman { config, animation } => {
				format!("species=braidman {} animation={animation:?}", config.sync_key())
			}
			Self::Brodler { config, animation } => {
				format!("species=brodler {} animation={animation:?}", config.sync_key())
			}
		}
	}

	pub fn spawn_key(&self) -> String {
		match self {
			Self::Braidman { config, .. } => format!(
				"species=braidman body={:?} head={:?} eye={:?} nose={:?} mouth={:?} ear={:?} hair={:?} clothing={:?}",
				config.body,
				config.head,
				config.eye,
				config.nose,
				config.mouth,
				config.ear,
				config.hair,
				config.clothing,
			),
			Self::Brodler { config, .. } => format!(
				"species=brodler head={:?} horns={:?} eye={:?} nose={:?} mouth={:?} ear={:?} hair={:?} clothing={:?}",
				config.head,
				config.horns,
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
			Self::Braidman { animation, .. } | Self::Brodler { animation, .. } => *animation,
		}
	}

	pub fn set_animation(&mut self, animation: ConceptAnimation) {
		match self {
			Self::Braidman { animation: current, .. } | Self::Brodler { animation: current, .. } => {
				*current = animation;
			}
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
	pub color: PreviewColor,
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

	match config {
		ConceptPreviewConfig::Braidman { config: braidman, .. } => {
			let sliders = braidman.sliders.clamped();
			for (part, mut target, base, transform) in parts {
				target.color = preview_color_braidman(braidman, target.target);
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
		ConceptPreviewConfig::Brodler { config: brodler, .. } => {
			for (_, mut target, ..) in parts {
				target.color = preview_color_brodler(brodler, target.target);
			}
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

fn preview_color_braidman(
	config: &BraidmanConfig,
	target: UiAssetTarget,
) -> PreviewColor {
	use crate::ui::braidman::UiAssetTarget as BraidmanTarget;
	use crozon_characters::species::braidman::BraidmanColor;

	let UiAssetTarget::Braidman(target) = target else {
		return PreviewColor::Braidman(BraidmanColor::Natural);
	};
	let skin = config.colors.skin_color();
	PreviewColor::Braidman(match target {
		BraidmanTarget::Body(_) => config.colors.body,
		BraidmanTarget::Head(_) | BraidmanTarget::Nose(_) | BraidmanTarget::Ear(_) => skin,
		BraidmanTarget::Eye(_) => config.colors.eyes,
		BraidmanTarget::Mouth(_) => config.colors.mouth,
		BraidmanTarget::Hair(_) => config.colors.hair,
		BraidmanTarget::Clothing(clothing) => config.colors.clothing_color(clothing),
		BraidmanTarget::Animation(_) => BraidmanColor::Natural,
	})
}

fn preview_color_brodler(config: &BrodlerConfig, target: UiAssetTarget) -> PreviewColor {
	use crate::ui::brodler::UiAssetTarget as BrodlerTarget;

	let UiAssetTarget::Brodler(target) = target else {
		return PreviewColor::BrodlerSkin(config.colors.skin);
	};
	match target {
		BrodlerTarget::Head(_) | BrodlerTarget::Body | BrodlerTarget::Nose(_)
		| BrodlerTarget::Ear(_) => PreviewColor::BrodlerSkin(config.colors.skin),
		BrodlerTarget::Horns(_) => PreviewColor::BrodlerHorn(config.colors.horns),
		BrodlerTarget::Eye(_) => PreviewColor::BrodlerEye(config.colors.eyes),
		BrodlerTarget::Mouth(_) => PreviewColor::Braidman(config.colors.mouth),
		BrodlerTarget::Hair(_) => PreviewColor::Braidman(config.colors.hair),
		BrodlerTarget::Clothing(clothing) => {
			PreviewColor::Braidman(config.colors.clothing_color(clothing))
		}
		BrodlerTarget::Animation(_) => PreviewColor::BrodlerSkin(config.colors.skin),
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
		match &self.config {
			ConceptPreviewConfig::Braidman { config, .. } => {
				let sliders = config.sliders.clamped();
				part.asset
					.normalization
					.transform()
					.mul_transform(sliders.feature_transform(part.slot))
			}
			ConceptPreviewConfig::Brodler { .. } => part.asset.normalization.transform(),
		}
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
		match &self.config {
			ConceptPreviewConfig::Braidman { config, .. } => {
				use crate::ui::braidman::UiAssetTarget as BraidmanTarget;
				let target = match part.slot {
					CharacterPartSlot::BodyMesh => BraidmanTarget::Body(config.body),
					CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
						BraidmanTarget::Head(config.head)
					}
					CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
						BraidmanTarget::Eye(config.eye)
					}
					CharacterPartSlot::Nose => BraidmanTarget::Nose(config.nose),
					CharacterPartSlot::Mouth => BraidmanTarget::Mouth(config.mouth),
					CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
						BraidmanTarget::Ear(config.ear)
					}
					CharacterPartSlot::Hair => BraidmanTarget::Hair(config.hair),
					CharacterPartSlot::Horns => BraidmanTarget::Head(config.head),
					CharacterPartSlot::Clothing => config
						.clothing
						.iter()
						.copied()
						.find(|clothing| clothing.label() == part.asset.label)
						.map(BraidmanTarget::Clothing)
						.unwrap_or(BraidmanTarget::Head(config.head)),
				};
				let ui_target = UiAssetTarget::Braidman(target);
				PreviewAssetTarget {
					target: ui_target,
					color: preview_color_braidman(config, ui_target),
				}
			}
			ConceptPreviewConfig::Brodler { config, .. } => {
				use crate::ui::brodler::UiAssetTarget as BrodlerTarget;
				let target = match part.slot {
					CharacterPartSlot::BodyMesh => BrodlerTarget::Body,
					CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
						BrodlerTarget::Head(config.head)
					}
					CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
						BrodlerTarget::Eye(config.eye)
					}
					CharacterPartSlot::Nose => BrodlerTarget::Nose(config.nose),
					CharacterPartSlot::Mouth => BrodlerTarget::Mouth(config.mouth),
					CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
						BrodlerTarget::Ear(config.ear)
					}
					CharacterPartSlot::Horns => BrodlerTarget::Horns(config.horns),
					CharacterPartSlot::Hair => BrodlerTarget::Hair(config.hair),
					CharacterPartSlot::Clothing => config
						.clothing
						.iter()
						.copied()
						.find(|clothing| clothing.label() == part.asset.label)
						.map(BrodlerTarget::Clothing)
						.unwrap_or(BrodlerTarget::Head(config.head)),
				};
				let ui_target = UiAssetTarget::Brodler(target);
				PreviewAssetTarget {
					target: ui_target,
					color: preview_color_brodler(config, ui_target),
				}
			}
		}
	}
}
