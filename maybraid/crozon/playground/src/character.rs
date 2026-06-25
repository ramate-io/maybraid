use bevy::prelude::*;

use crate::animation::AnimationMode;
use crate::skinning::{
	BoneMap, CharacterRig, DumpBonesRequest, ModularPart, ModularPartKind, NeedsSkinRemap,
	NeedsSocketPlacement, PartRigRef, HEAD_SCALE, HEAD_SOCKET_BONE,
};

pub const DEFAULT_RIG: &str = "characters/bodies/humanoid_rig.glb";
pub const DEFAULT_BODY: &str = "characters/bodies/humanoid_playground.glb";

#[derive(Component)]
pub struct CharacterRoot;

#[derive(Resource, Clone)]
pub struct CharacterConfig {
	pub rig: String,
	pub body: Option<String>,
	pub head: Option<String>,
	pub mouth: Option<String>,
	pub nose: Option<String>,
	pub animation: AnimationMode,
	pub transform: Transform,
}

impl Default for CharacterConfig {
	fn default() -> Self {
		Self {
			rig: DEFAULT_RIG.into(),
			body: Some(DEFAULT_BODY.into()),
			head: None,
			mouth: None,
			nose: None,
			animation: AnimationMode::default(),
			transform: Transform::IDENTITY,
		}
	}
}

impl CharacterConfig {
	pub fn status_label(&self) -> String {
		let mut parts = vec![format!("rig={}", self.rig)];
		if let Some(body) = &self.body {
			parts.push(format!("body={body}"));
		}
		if let Some(head) = &self.head {
			parts.push(format!("head={head}"));
		}
		if let Some(mouth) = &self.mouth {
			parts.push(format!("mouth={mouth}"));
		}
		if let Some(nose) = &self.nose {
			parts.push(format!("nose={nose}"));
		}
		parts.push(format!("animation={:?}", self.animation));
		format!("character {}", parts.join(" "))
	}

	pub fn sync_key(&self) -> String {
		format!(
			"{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
			self.rig,
			self.body,
			self.head,
			self.mouth,
			self.nose,
			self.transform.translation,
			self.transform.rotation,
		)
	}

	fn part_specs(&self) -> Vec<(&'static str, &str, ModularPartKind)> {
		let mut parts = Vec::new();
		if let Some(body) = &self.body {
			parts.push(("body", body.as_str(), ModularPartKind::Body));
		}
		if let Some(head) = &self.head {
			parts.push(("head", head.as_str(), ModularPartKind::Head));
		}
		if let Some(mouth) = &self.mouth {
			parts.push(("mouth", mouth.as_str(), ModularPartKind::Mouth));
		}
		if let Some(nose) = &self.nose {
			parts.push(("nose", nose.as_str(), ModularPartKind::Nose));
		}
		parts
	}
}

#[derive(Resource, Default)]
pub(crate) struct CharacterSyncState {
	key: String,
}

pub(crate) fn sync_character(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	config: Res<CharacterConfig>,
	mut sync_state: ResMut<CharacterSyncState>,
	roots: Query<Entity, With<CharacterRoot>>,
) {
	let key = config.sync_key();
	if sync_state.key == key {
		return;
	}
	sync_state.key.clone_from(&key);

	for entity in &roots {
		commands.entity(entity).despawn();
	}

	let rig_path = config.rig.clone();
	let transform = config.transform;
	let parts: Vec<(String, String, ModularPartKind)> = config
		.part_specs()
		.into_iter()
		.map(|(label, path, kind)| (label.to_string(), path.to_string(), kind))
		.collect();

	let rig_entity = commands
		.spawn((
			SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(rig_path))),
			CharacterRig,
			BoneMap::default(),
			CharacterRoot,
			transform,
			Name::new("character_rig"),
		))
		.id();

	for (label, path, kind) in parts {
		let mut part = commands.spawn((
			SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path))),
			ModularPart,
			kind,
			PartRigRef { rig_root: rig_entity },
			NeedsSkinRemap,
			CharacterRoot,
			transform,
			Name::new(format!("character_{label}")),
		));

		if kind == ModularPartKind::Head {
			part.insert(NeedsSocketPlacement { socket_bone: HEAD_SOCKET_BONE, scale: HEAD_SCALE });
		}
	}
}

pub fn request_dump_bones(commands: &mut Commands) {
	commands.queue(|world: &mut World| {
		world.resource_mut::<DumpBonesRequest>().0 = true;
	});
}
