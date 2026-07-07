//! Hidden body + head rigs for stable camera-focus sockets.
//!
//! These armatures mirror the preview's proportional pose but are never animated,
//! so camera framing stays stable while the visible character runs cycles.

use bevy::prelude::*;
use crozon_characters::assembly::{CharacterPartSlot, ResolvedCharacterAssembly};

use crate::preview::{ConceptPreviewConfig, ConceptSpecies};
use crate::skinning::{
	ActiveRigPose, BoneMap, CharacterRig, CharacterRigRole, NeedsSocketPlacement, RigBindScales,
};

#[derive(Component)]
pub struct FocusReferenceRoot;

/// Marks a hidden rig used only for [`crate::camera_focus`] socket resolution.
#[derive(Component)]
pub struct FocusReferenceRig;

#[derive(Resource, Default)]
pub struct FocusReferenceSyncState {
	live_key: String,
	spawn_key: String,
}

impl FocusReferenceSyncState {
	/// Drop cached pose/config so the next sync re-runs without despawning rigs.
	pub(crate) fn invalidate_live(&mut self) {
		self.live_key.clear();
	}

	/// Force a full hidden-rig respawn (species switch).
	pub(crate) fn invalidate(&mut self) {
		self.live_key.clear();
		self.spawn_key.clear();
	}
}

fn focus_live_key(config: &ConceptPreviewConfig) -> String {
	match config {
		ConceptPreviewConfig::Braidman { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Brodler { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Mygr { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Dui { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Wumbus { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Lero { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Spibmom { config, .. } => config.sync_key(),
	}
}

/// Hidden focus rigs only mirror body + head armatures; cosmetic part swaps do not
/// change these assets.
fn focus_spawn_key(config: &ConceptPreviewConfig) -> String {
	match config.species() {
		ConceptSpecies::Braidman => "focus_rigs=braidman".into(),
		ConceptSpecies::Brodler => "focus_rigs=brodler".into(),
		ConceptSpecies::Mygr => "focus_rigs=mygr".into(),
		ConceptSpecies::Dui => "focus_rigs=dui".into(),
		ConceptSpecies::Wumbus => "focus_rigs=wumbus".into(),
		ConceptSpecies::Lero => "focus_rigs=lero".into(),
		ConceptSpecies::Spibmom => "focus_rigs=spibmom".into(),
	}
}

pub fn sync_focus_reference(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	config: Res<ConceptPreviewConfig>,
	mut sync_state: ResMut<FocusReferenceSyncState>,
	mut body_poses: Query<&mut ActiveRigPose, With<FocusReferenceRig>>,
	roots: Query<Entity, With<FocusReferenceRoot>>,
) {
	let live_key = focus_live_key(&config);
	let spawn_key = focus_spawn_key(&config);
	if sync_state.live_key == live_key {
		return;
	}

	let assembly = config.resolve();
	if sync_state.spawn_key == spawn_key {
		sync_state.live_key.clone_from(&live_key);
		for mut pose in &mut body_poses {
			pose.pose = assembly.pose.clone();
		}
		return;
	}

	sync_state.live_key = live_key;
	sync_state.spawn_key = spawn_key;

	for entity in &roots {
		commands.entity(entity).try_despawn();
	}

	spawn_focus_reference(&mut commands, &asset_server, &assembly);
}

fn spawn_focus_reference(
	commands: &mut Commands,
	asset_server: &AssetServer,
	assembly: &ResolvedCharacterAssembly,
) {
	let body_rig = commands
		.spawn((
			WorldAssetRoot(
				asset_server
					.load(GltfAssetLabel::Scene(0).from_asset(assembly.body_rig.path.as_str())),
			),
			CharacterRig { role: CharacterRigRole::Body },
			FocusReferenceRig,
			BoneMap::default(),
			ActiveRigPose { pose: assembly.pose.clone() },
			RigBindScales::default(),
			FocusReferenceRoot,
			Visibility::Hidden,
			Transform::IDENTITY,
			Name::new(format!("focus_{}_body_rig", assembly.label)),
		))
		.id();

	let Some(head_part) =
		assembly.parts.iter().find(|part| part.slot == CharacterPartSlot::HeadRig)
	else {
		return;
	};

	let head_rig = commands
		.spawn((
			WorldAssetRoot(
				asset_server
					.load(GltfAssetLabel::Scene(0).from_asset(head_part.asset.path.as_str())),
			),
			CharacterRig { role: CharacterRigRole::Head },
			FocusReferenceRig,
			BoneMap::default(),
			FocusReferenceRoot,
			Visibility::Hidden,
			Transform::IDENTITY,
			Name::new("focus_head_rig"),
		))
		.id();

	if let Some(socket) = head_part.socket {
		commands.entity(head_rig).insert(NeedsSocketPlacement {
			rig_root: body_rig,
			socket_bone: socket.bone,
			local_transform: socket.local_transform,
		});
	}
}
