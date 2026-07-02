//! Preview configuration and spawning.
//!
//! Commands update [`ConceptPreviewConfig`]. This module resolves that config via
//! `crozon-characters` and spawns Bevy scenes from the resulting assembly.

use bevy::prelude::*;
use crozon_characters::{
	assembly::{CharacterPartSlot, ResolvedCharacterAssembly},
	species::{
		braidman::BraidmanConfig,
		brodler::{BrodlerConfig, BrodlerHeadMesh, HornMesh},
		mygr::MygrConfig,
		dui::{DuiConfig, DuiNoseMesh},
		wumbus::{WumbusConfig, WumbusHornMesh},
		lero::{LeroConfig, LeroMouthMesh},
		common::{
			BodyMesh, ClothingMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh,
		},
		SpeciesConfig,
	},
	ResolvedCharacterPart, SkinTarget, SocketRig,
};

use crate::animation::{AnimatedBodyRig, BodyRigBindTransform, ConceptAnimation};
use crate::preview_color::PreviewColor;
use crate::skinning::{
	bind_scales_ready, bone_map_ready, ActiveRigPose, BoneMap, CharacterPart, CharacterRig,
	CharacterRigRole, NeedsDuplicateScenePrune, NeedsSkinRemap, NeedsSocketPlacement,
	NoMatchingArmature, PartRigRef, RigBindScales,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ConceptSpecies {
	#[default]
	Braidman,
	Brodler,
	Mygr,
	Dui,
	Wumbus,
	Lero,
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub enum ConceptPreviewConfig {
	Braidman { config: BraidmanConfig, animation: ConceptAnimation },
	Brodler { config: BrodlerConfig, animation: ConceptAnimation },
	Mygr { config: MygrConfig, animation: ConceptAnimation },
	Dui { config: DuiConfig, animation: ConceptAnimation },
	Wumbus { config: WumbusConfig, animation: ConceptAnimation },
	Lero { config: LeroConfig, animation: ConceptAnimation },
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
			ConceptSpecies::Mygr => Self::mygr(MygrConfig::default_preview()),
			ConceptSpecies::Dui => Self::dui(DuiConfig::default_preview()),
			ConceptSpecies::Wumbus => Self::wumbus(WumbusConfig::default_preview()),
			ConceptSpecies::Lero => Self::lero(LeroConfig::default_preview()),
		}
	}

	pub fn species(&self) -> ConceptSpecies {
		match self {
			Self::Braidman { .. } => ConceptSpecies::Braidman,
			Self::Brodler { .. } => ConceptSpecies::Brodler,
			Self::Mygr { .. } => ConceptSpecies::Mygr,
			Self::Dui { .. } => ConceptSpecies::Dui,
			Self::Wumbus { .. } => ConceptSpecies::Wumbus,
			Self::Lero { .. } => ConceptSpecies::Lero,
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

	pub fn mygr(config: MygrConfig) -> Self {
		Self::Mygr { config, animation: ConceptAnimation::default() }
	}

	pub fn mygr_with_animation(config: MygrConfig, animation: ConceptAnimation) -> Self {
		Self::Mygr { config, animation }
	}

	pub fn dui(config: DuiConfig) -> Self {
		Self::Dui { config, animation: ConceptAnimation::default() }
	}

	pub fn dui_with_animation(config: DuiConfig, animation: ConceptAnimation) -> Self {
		Self::Dui { config, animation }
	}

	pub fn wumbus(config: WumbusConfig) -> Self {
		Self::Wumbus { config, animation: ConceptAnimation::default() }
	}

	pub fn wumbus_with_animation(config: WumbusConfig, animation: ConceptAnimation) -> Self {
		Self::Wumbus { config, animation }
	}

	pub fn lero(config: LeroConfig) -> Self {
		Self::Lero { config, animation: ConceptAnimation::default() }
	}

	pub fn lero_with_animation(config: LeroConfig, animation: ConceptAnimation) -> Self {
		Self::Lero { config, animation }
	}

	pub fn resolve(&self) -> ResolvedCharacterAssembly {
		match self {
			Self::Braidman { config, .. } => config.resolve(),
			Self::Brodler { config, .. } => config.resolve(),
			Self::Mygr { config, .. } => config.resolve(),
			Self::Dui { config, .. } => config.resolve(),
			Self::Wumbus { config, .. } => config.resolve(),
			Self::Lero { config, .. } => config.resolve(),
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
			Self::Mygr { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Dui { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Wumbus { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Lero { config, animation } => {
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
			Self::Mygr { config, animation } => {
				format!("species=mygr {} animation={animation:?}", config.sync_key())
			}
			Self::Dui { config, animation } => {
				format!("species=dui {} animation={animation:?}", config.sync_key())
			}
			Self::Wumbus { config, animation } => {
				format!("species=wumbus {} animation={animation:?}", config.sync_key())
			}
			Self::Lero { config, animation } => {
				format!("species=lero {} animation={animation:?}", config.sync_key())
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
			Self::Mygr { config, .. } => format!(
				"species=mygr eye={:?} hair={:?} clothing={:?}",
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Dui { config, .. } => format!(
				"species=dui nose={:?} hair={:?} clothing={:?}",
				config.nose,
				config.hair,
				config.clothing,
			),
			Self::Wumbus { config, .. } => format!(
				"species=wumbus horns={:?} eye={:?} hair={:?} clothing={:?}",
				config.horns,
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Lero { config, .. } => format!(
				"species=lero mouth={:?} hair={:?} clothing={:?}",
				config.mouth,
				config.hair,
				config.clothing,
			),
		}
	}

	pub const fn animation(&self) -> ConceptAnimation {
		match self {
			Self::Braidman { animation, .. }
			| Self::Brodler { animation, .. }
			| Self::Mygr { animation, .. }
			| Self::Dui { animation, .. }
			| Self::Wumbus { animation, .. }
			| Self::Lero { animation, .. } => *animation,
		}
	}
}

#[derive(Resource, Default)]
pub struct ConceptPreviewSyncState {
	live_key: String,
	spawn_key: String,
}

impl ConceptPreviewSyncState {
	/// Drop cached pose/config so the next sync can live-update or respawn parts.
	pub(crate) fn invalidate_live(&mut self) {
		self.live_key.clear();
	}

	/// Force a full preview respawn (species switch).
	pub(crate) fn invalidate(&mut self) {
		self.live_key.clear();
		self.spawn_key.clear();
	}
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
	pub target: PreviewTarget,
	pub color: PreviewColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreviewTarget {
	BraidmanBody(BodyMesh),
	BraidmanHead(HeadMesh),
	BraidmanEye(EyeMesh),
	BraidmanNose(NoseMesh),
	BraidmanMouth(MouthMesh),
	BraidmanEar(EarMesh),
	BraidmanHair(HairMesh),
	BraidmanClothing(ClothingMesh),
	BrodlerBody,
	BrodlerHead(BrodlerHeadMesh),
	BrodlerHorns(HornMesh),
	BrodlerEye(EyeMesh),
	BrodlerNose(NoseMesh),
	BrodlerMouth(MouthMesh),
	BrodlerEar(EarMesh),
	BrodlerHair(HairMesh),
	BrodlerClothing(ClothingMesh),
	MygrBody,
	MygrHead,
	MygrEye(EyeMesh),
	MygrMouth,
	MygrEar,
	MygrTail,
	MygrHair(HairMesh),
	MygrClothing(ClothingMesh),
	DuiBody,
	DuiHead,
	DuiEye,
	DuiNose(DuiNoseMesh),
	DuiMouth,
	DuiHair(HairMesh),
	DuiClothing(ClothingMesh),
	WumbusBody,
	WumbusHead,
	WumbusHorns(WumbusHornMesh),
	WumbusSpine,
	WumbusEye(EyeMesh),
	WumbusMouth,
	WumbusEar,
	WumbusHair(HairMesh),
	WumbusClothing(ClothingMesh),
	LeroBody,
	LeroHead,
	LeroEye,
	LeroMouth(LeroMouthMesh),
	LeroTail,
	LeroSpine,
	LeroHair(HairMesh),
	LeroClothing(ClothingMesh),
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
				let authored =
					base.normalization.mul_transform(sliders.feature_transform(part.slot));
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
		ConceptPreviewConfig::Mygr { config: mygr, .. } => {
			for (_, mut target, ..) in parts {
				target.color = preview_color_mygr(mygr, target.target);
			}
		}
		ConceptPreviewConfig::Dui { config: dui, .. } => {
			for (_, mut target, ..) in parts {
				target.color = preview_color_dui(dui, target.target);
			}
		}
		ConceptPreviewConfig::Wumbus { config: wumbus, .. } => {
			for (_, mut target, ..) in parts {
				target.color = preview_color_wumbus(wumbus, target.target);
			}
		}
		ConceptPreviewConfig::Lero { config: lero, .. } => {
			for (_, mut target, ..) in parts {
				target.color = preview_color_lero(lero, target.target);
			}
		}
	}
}

/// Reveal a respawned preview only after the body pose, socket attach, and skin
/// remap passes have settled.
pub fn reveal_ready_preview(
	mut commands: Commands,
	pending: Query<Entity, With<PreviewAwaitingReveal>>,
	body_rigs: Query<(&BoneMap, &RigBindScales), With<AnimatedBodyRig>>,
	awaiting_socket: Query<(), (With<NeedsSocketPlacement>, With<ConceptPreviewRoot>)>,
	awaiting_remap: Query<
		(),
		(
			With<NeedsSkinRemap>,
			With<CharacterPart>,
			With<ConceptPreviewRoot>,
			Without<NoMatchingArmature>,
		),
	>,
	awaiting_prune: Query<
		(),
		(With<NeedsDuplicateScenePrune>, With<CharacterPart>, With<ConceptPreviewRoot>),
	>,
) {
	let Ok((bone_map, bind_scales)) = body_rigs.single() else {
		return;
	};
	if !bone_map_ready(bone_map) || !bind_scales_ready(bind_scales, bone_map) {
		return;
	}
	if !awaiting_socket.is_empty() || !awaiting_remap.is_empty() || !awaiting_prune.is_empty() {
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

fn preview_color_braidman(config: &BraidmanConfig, target: PreviewTarget) -> PreviewColor {
	use crozon_characters::species::braidman::BraidmanColor;

	let skin = config.colors.skin_color();
	PreviewColor::Braidman(match target {
		PreviewTarget::BraidmanBody(_) => config.colors.body,
		PreviewTarget::BraidmanHead(_)
		| PreviewTarget::BraidmanNose(_)
		| PreviewTarget::BraidmanEar(_) => skin,
		PreviewTarget::BraidmanEye(_) => config.colors.eyes,
		PreviewTarget::BraidmanMouth(_) => config.colors.mouth,
		PreviewTarget::BraidmanHair(_) => config.colors.hair,
		PreviewTarget::BraidmanClothing(clothing) => config.colors.clothing_color(clothing),
		_ => BraidmanColor::Natural,
	})
}

fn preview_color_brodler(config: &BrodlerConfig, target: PreviewTarget) -> PreviewColor {
	match target {
		PreviewTarget::BrodlerHead(_)
		| PreviewTarget::BrodlerBody
		| PreviewTarget::BrodlerNose(_)
		| PreviewTarget::BrodlerEar(_) => PreviewColor::BrodlerSkin(config.colors.skin),
		PreviewTarget::BrodlerHorns(_) => PreviewColor::BrodlerHorn(config.colors.horns),
		PreviewTarget::BrodlerEye(_) => PreviewColor::BrodlerEye(config.colors.eyes),
		PreviewTarget::BrodlerMouth(_) => PreviewColor::Braidman(config.colors.mouth),
		PreviewTarget::BrodlerHair(_) => PreviewColor::Braidman(config.colors.hair),
		PreviewTarget::BrodlerClothing(clothing) => {
			PreviewColor::Braidman(config.colors.clothing_color(clothing))
		}
		_ => PreviewColor::BrodlerSkin(config.colors.skin),
	}
}

fn preview_color_mygr(config: &MygrConfig, target: PreviewTarget) -> PreviewColor {
	match target {
		PreviewTarget::MygrHead
		| PreviewTarget::MygrBody
		| PreviewTarget::MygrEar
		| PreviewTarget::MygrTail => PreviewColor::MygrSkin(config.colors.skin),
		PreviewTarget::MygrEye(_) => PreviewColor::MygrEye(config.colors.eyes),
		PreviewTarget::MygrMouth => PreviewColor::Braidman(config.colors.mouth),
		PreviewTarget::MygrHair(_) => PreviewColor::Braidman(config.colors.hair),
		PreviewTarget::MygrClothing(clothing) => {
			PreviewColor::Braidman(config.colors.clothing_color(clothing))
		}
		_ => PreviewColor::MygrSkin(config.colors.skin),
	}
}

fn preview_color_dui(config: &DuiConfig, target: PreviewTarget) -> PreviewColor {
	match target {
		PreviewTarget::DuiHead
		| PreviewTarget::DuiBody => PreviewColor::DuiSkin(config.colors.skin),
		PreviewTarget::DuiNose(_) => PreviewColor::DuiNose(config.colors.nose_color),
		PreviewTarget::DuiEye => PreviewColor::DuiEye(config.colors.eyes),
		PreviewTarget::DuiMouth => PreviewColor::DuiMouth(config.colors.mouth),
		PreviewTarget::DuiHair(_) => PreviewColor::Braidman(config.colors.hair),
		PreviewTarget::DuiClothing(clothing) => {
			PreviewColor::Braidman(config.colors.clothing_color(clothing))
		}
		_ => PreviewColor::DuiSkin(config.colors.skin),
	}
}

fn preview_color_wumbus(config: &WumbusConfig, target: PreviewTarget) -> PreviewColor {
	match target {
		PreviewTarget::WumbusHead | PreviewTarget::WumbusBody => {
			PreviewColor::WumbusSkin(config.colors.skin)
		}
		PreviewTarget::WumbusHorns(_) => PreviewColor::WumbusHorn(config.colors.horns),
		PreviewTarget::WumbusSpine => PreviewColor::WumbusSpine(config.colors.spine),
		PreviewTarget::WumbusEye(_) => PreviewColor::WumbusEye(config.colors.eyes),
		PreviewTarget::WumbusEar => PreviewColor::WumbusEar(config.colors.ears),
		PreviewTarget::WumbusMouth => PreviewColor::WumbusMouth(config.colors.mouth),
		PreviewTarget::WumbusHair(_) => PreviewColor::Braidman(config.colors.hair),
		PreviewTarget::WumbusClothing(clothing) => {
			PreviewColor::Braidman(config.colors.clothing_color(clothing))
		}
		_ => PreviewColor::WumbusSkin(config.colors.skin),
	}
}

fn preview_color_lero(config: &LeroConfig, target: PreviewTarget) -> PreviewColor {
	match target {
		PreviewTarget::LeroHead | PreviewTarget::LeroBody | PreviewTarget::LeroMouth(_) => {
			PreviewColor::LeroSkin(config.colors.skin)
		}
		PreviewTarget::LeroEye => PreviewColor::LeroEye(config.colors.eyes),
		PreviewTarget::LeroTail => PreviewColor::LeroTail(config.colors.tail),
		PreviewTarget::LeroSpine => PreviewColor::LeroSpine(config.colors.spine),
		PreviewTarget::LeroHair(_) => PreviewColor::Braidman(config.colors.hair),
		PreviewTarget::LeroClothing(clothing) => {
			PreviewColor::Braidman(config.colors.clothing_color(clothing))
		}
		_ => PreviewColor::LeroSkin(config.colors.skin),
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
			ConceptPreviewConfig::Mygr { .. } => part.asset.normalization.transform(),
			ConceptPreviewConfig::Dui { .. } => part.asset.normalization.transform(),
			ConceptPreviewConfig::Wumbus { .. } => part.asset.normalization.transform(),
			ConceptPreviewConfig::Lero { .. } => part.asset.normalization.transform(),
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
				let target = match part.slot {
					CharacterPartSlot::BodyMesh => PreviewTarget::BraidmanBody(config.body),
					CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
						PreviewTarget::BraidmanHead(config.head)
					}
					CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
						PreviewTarget::BraidmanEye(config.eye)
					}
					CharacterPartSlot::Nose => PreviewTarget::BraidmanNose(config.nose),
					CharacterPartSlot::Mouth => PreviewTarget::BraidmanMouth(config.mouth),
					CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
						PreviewTarget::BraidmanEar(config.ear)
					}
					CharacterPartSlot::Hair => PreviewTarget::BraidmanHair(config.hair),
					CharacterPartSlot::Horns => PreviewTarget::BraidmanHead(config.head),
					CharacterPartSlot::Clothing => config
						.clothing
						.iter()
						.copied()
						.find(|clothing| clothing.label() == part.asset.label)
						.map(PreviewTarget::BraidmanClothing)
						.unwrap_or(PreviewTarget::BraidmanHead(config.head)),
					CharacterPartSlot::Tail => PreviewTarget::BraidmanBody(config.body),
					CharacterPartSlot::Spine => PreviewTarget::BraidmanBody(config.body),
				};
				PreviewAssetTarget { target, color: preview_color_braidman(config, target) }
			}
			ConceptPreviewConfig::Brodler { config, .. } => {
				let target = match part.slot {
					CharacterPartSlot::BodyMesh => PreviewTarget::BrodlerBody,
					CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
						PreviewTarget::BrodlerHead(config.head)
					}
					CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
						PreviewTarget::BrodlerEye(config.eye)
					}
					CharacterPartSlot::Nose => PreviewTarget::BrodlerNose(config.nose),
					CharacterPartSlot::Mouth => PreviewTarget::BrodlerMouth(config.mouth),
					CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
						PreviewTarget::BrodlerEar(config.ear)
					}
					CharacterPartSlot::Horns => PreviewTarget::BrodlerHorns(config.horns),
					CharacterPartSlot::Hair => PreviewTarget::BrodlerHair(config.hair),
					CharacterPartSlot::Clothing => config
						.clothing
						.iter()
						.copied()
						.find(|clothing| clothing.label() == part.asset.label)
						.map(PreviewTarget::BrodlerClothing)
						.unwrap_or(PreviewTarget::BrodlerHead(config.head)),
					CharacterPartSlot::Tail => PreviewTarget::BrodlerBody,
					CharacterPartSlot::Spine => PreviewTarget::BrodlerBody,
				};
				PreviewAssetTarget { target, color: preview_color_brodler(config, target) }
			}
			ConceptPreviewConfig::Mygr { config, .. } => {
				let target = match part.slot {
					CharacterPartSlot::BodyMesh => PreviewTarget::MygrBody,
					CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => PreviewTarget::MygrHead,
					CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
						PreviewTarget::MygrEye(config.eye)
					}
					CharacterPartSlot::Mouth => PreviewTarget::MygrMouth,
					CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => PreviewTarget::MygrEar,
					CharacterPartSlot::Tail => PreviewTarget::MygrTail,
					CharacterPartSlot::Nose | CharacterPartSlot::Horns => PreviewTarget::MygrHead,
					CharacterPartSlot::Hair => PreviewTarget::MygrHair(config.hair),
					CharacterPartSlot::Clothing => config
						.clothing
						.iter()
						.copied()
						.find(|clothing| clothing.label() == part.asset.label)
						.map(PreviewTarget::MygrClothing)
						.unwrap_or(PreviewTarget::MygrHead),
					CharacterPartSlot::Spine => PreviewTarget::MygrBody,
				};
				PreviewAssetTarget { target, color: preview_color_mygr(config, target) }
			}
			ConceptPreviewConfig::Dui { config, .. } => {
				let target = match part.slot {
					CharacterPartSlot::BodyMesh => PreviewTarget::DuiBody,
					CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => PreviewTarget::DuiHead,
					CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => PreviewTarget::DuiEye,
					CharacterPartSlot::Nose => PreviewTarget::DuiNose(config.nose),
					CharacterPartSlot::Mouth => PreviewTarget::DuiMouth,
					CharacterPartSlot::EarLeft
					| CharacterPartSlot::EarRight
					| CharacterPartSlot::Horns
					| CharacterPartSlot::Tail
					| CharacterPartSlot::Spine => PreviewTarget::DuiHead,
					CharacterPartSlot::Hair => PreviewTarget::DuiHair(config.hair),
					CharacterPartSlot::Clothing => config
						.clothing
						.iter()
						.copied()
						.find(|clothing| clothing.label() == part.asset.label)
						.map(PreviewTarget::DuiClothing)
						.unwrap_or(PreviewTarget::DuiHead),
				};
				PreviewAssetTarget { target, color: preview_color_dui(config, target) }
			}
			ConceptPreviewConfig::Wumbus { config, .. } => {
				let target = match part.slot {
					CharacterPartSlot::BodyMesh => PreviewTarget::WumbusBody,
					CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => PreviewTarget::WumbusHead,
					CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
						PreviewTarget::WumbusEye(config.eye)
					}
					CharacterPartSlot::Mouth => PreviewTarget::WumbusMouth,
					CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => PreviewTarget::WumbusEar,
					CharacterPartSlot::Horns => PreviewTarget::WumbusHorns(config.horns),
					CharacterPartSlot::Spine => PreviewTarget::WumbusSpine,
					CharacterPartSlot::Nose | CharacterPartSlot::Tail => PreviewTarget::WumbusHead,
					CharacterPartSlot::Hair => PreviewTarget::WumbusHair(config.hair),
					CharacterPartSlot::Clothing => config
						.clothing
						.iter()
						.copied()
						.find(|clothing| clothing.label() == part.asset.label)
						.map(PreviewTarget::WumbusClothing)
						.unwrap_or(PreviewTarget::WumbusHead),
				};
				PreviewAssetTarget { target, color: preview_color_wumbus(config, target) }
			}
			ConceptPreviewConfig::Lero { config, .. } => {
				let target = match part.slot {
					CharacterPartSlot::BodyMesh => PreviewTarget::LeroBody,
					CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => PreviewTarget::LeroHead,
					CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => PreviewTarget::LeroEye,
					CharacterPartSlot::Mouth => PreviewTarget::LeroMouth(config.mouth),
					CharacterPartSlot::Tail => PreviewTarget::LeroTail,
					CharacterPartSlot::Spine => PreviewTarget::LeroSpine,
					CharacterPartSlot::Nose
					| CharacterPartSlot::EarLeft
					| CharacterPartSlot::EarRight
					| CharacterPartSlot::Horns => PreviewTarget::LeroHead,
					CharacterPartSlot::Hair => PreviewTarget::LeroHair(config.hair),
					CharacterPartSlot::Clothing => config
						.clothing
						.iter()
						.copied()
						.find(|clothing| clothing.label() == part.asset.label)
						.map(PreviewTarget::LeroClothing)
						.unwrap_or(PreviewTarget::LeroHead),
				};
				PreviewAssetTarget { target, color: preview_color_lero(config, target) }
			}
		}
	}
}
