//! Hidden "shadow" rigs that hold the character's proportional pose for
//! camera-focus socket resolution.
//!
//! # Why a shadow rig?
//!
//! The visible preview rig is animated: its bones move every frame, so camera
//! framing derived from it would chase the walk/run/gallop cycle. The shadow
//! rigs load the same body and head-rig armatures and apply the same resolved
//! proportion pose ([`ActiveRigPose`]), but are never animated and never
//! rendered ([`Visibility::Hidden`]). Socket lookups against them therefore
//! reflect the character's proportions without any animation transforms.
//!
//! # Lifecycle
//!
//! Readiness is signalled by imperative state changes, never approximated from
//! bone transforms:
//!
//! 1. [`sync_focus_reference`] spawns a shadow body rig (plus a head rig when
//!    the assembly has a `HeadRig` part) whenever the spawn key — body-rig and
//!    head-rig asset paths — changes. Old shadow roots are despawned.
//! 2. `build_rig_bone_map` (skinning) fills [`BoneMap`] as the GLTF scenes
//!    spawn bones.
//! 3. `maintain_resolved_pose` (skinning) applies the proportional pose and
//!    inserts [`ResolvedPoseApplied`](crate::skinning::ResolvedPoseApplied)
//!    on the body rig once the pose is fully written. Camera focus gates on
//!    that marker.
//! 4. `attach_focus_reference_to_sockets` (skinning) parents the head rig to
//!    its socket bone and removes [`NeedsSocketPlacement`] — the readiness
//!    signal for head-socket focuses.
//! 5. Config tweaks that keep the same armatures (sliders, colors) only update
//!    [`ActiveRigPose`] in place; pose maintenance re-applies it every frame,
//!    so no respawn and no readiness reset is needed.

use bevy::prelude::*;
use crozon_characters::assembly::{CharacterPartSlot, ResolvedCharacterAssembly};

use crate::preview::ConceptPreviewConfig;
use crate::skinning::{
	ActiveRigPose, BoneMap, CharacterRig, CharacterRigRole, NeedsSocketPlacement, RigBindScales,
	RigSkeletonKind,
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

/// Any config change that can affect proportions refreshes the applied pose.
fn focus_live_key(config: &ConceptPreviewConfig) -> String {
	match config {
		ConceptPreviewConfig::Braidman { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Brenal { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Brodler { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Mygr { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Dui { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Wumbus { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Lero { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Spibmom { config, .. } => config.sync_key(),
	}
}

/// Shadow rigs must respawn when the underlying armature assets change: a new
/// body rig or head rig carries different socket bones. Cosmetic part swaps on
/// the same armatures only refresh the live pose.
fn focus_spawn_key(assembly: &ResolvedCharacterAssembly) -> String {
	let head_rig_path = assembly
		.parts
		.iter()
		.find(|part| part.slot == CharacterPartSlot::HeadRig)
		.map(|part| part.asset.path.as_str())
		.unwrap_or("");
	format!("body={} head={head_rig_path}", assembly.body_rig.path.as_str())
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
	if sync_state.live_key == live_key {
		return;
	}

	let assembly = config.resolve();
	let spawn_key = focus_spawn_key(&assembly);
	if sync_state.spawn_key == spawn_key {
		// Same armatures: update the pose in place. `maintain_resolved_pose`
		// re-applies it every frame, so nothing else needs to be invalidated.
		sync_state.live_key = live_key;
		for mut pose in &mut body_poses {
			pose.pose = assembly.pose.clone();
		}
		return;
	}

	sync_state.live_key = live_key;
	sync_state.spawn_key = spawn_key;

	// Respawning from scratch resets all readiness markers imperatively: the
	// fresh rigs carry no `ResolvedPoseApplied` and the head rig starts with
	// `NeedsSocketPlacement`, so camera focus waits for the new pose.
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
	let skeleton = RigSkeletonKind::from_body_rig_label(assembly.body_rig.label);
	let body_rig = commands
		.spawn((
			WorldAssetRoot(
				asset_server
					.load(GltfAssetLabel::Scene(0).from_asset(assembly.body_rig.path.as_str())),
			),
			CharacterRig { role: CharacterRigRole::Body, skeleton },
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
			CharacterRig { role: CharacterRigRole::Head, skeleton: RigSkeletonKind::Humanoid },
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
