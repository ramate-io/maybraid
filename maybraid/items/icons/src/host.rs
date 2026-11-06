//! Realize posed, painted inventory subjects into isolated capture roots.

use crate::fisticuffs::{isolate_arm_joints, FisticuffsArmMask, FisticuffsGuardPose};
use bevy::camera::primitives::MeshAabb;
use bevy::camera::visibility::RenderLayers;
use bevy::ecs::entity_disabling::Disabled;
use bevy::ecs::query::{Allow, QueryFilter};
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use bevy::scene::prelude::bsn;
use bevy::world_serialization::WorldAssetRoot;
use character_items::{ClothingHost, ClothingMesh, InventoryItem, ItemColor};
use character_shaders::{CharacterShadersPlugin, ClothingShaderMaterial, FaceShaderMaterial};
use characters::material_lib::CharacterMaterialRefPlugin;
use characters::species::braidman::{Braidman, BraidmanConfig};
use characters::{
	add_character_components_host, character_bounds, drain_character_assembly_now,
	hide_socketed_parts, spawn_fixed_character_assembly, AnimClip, AnimMailbox, AnimProgress,
	AnimRef, AnimRefRoot, AnimateBones, AnimateEffects, AssetNormalization, BoneMap,
	CharacterComponents, CharacterComponentsPlugin, CharacterHostsPlugin, CharacterMembers,
	CharacterMotionSystems, CharacterPartSlot, CharacterRecipe, CharacterRig, CharacterRigRole,
	ClothingLayer, ComponentsOnly, HiddenUntilSceneReady, Layers, PartNode,
	PendingCharacterAssembly, RigNode,
};
use firearm_user::GeneratedFirearm;
use firearms_components::{
	add_firearm_components_host, firearm_bounds, spawn_firearm_components, ActiveRigPose,
	FirearmComponentsPlugin, PoseApplied,
};
use grenades::{grenade_material, grenade_mesh, GrenadeMaterial, GrenadeMaterialPlugin};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel};
use lod::{
	add_lod_refresh_chunk_for, LodLevelRootPending, LodLevelSpawnRequest, LodRefreshCorePlugin,
	LodRefreshDomain, LodRefreshMembership,
};
use material_ref::MaterialRefApplied;
use scene_ref::SceneRefRoot;
use screen_capture::{
	CaptureHostReady, CaptureHostRoot, CaptureId, CaptureSilhouetteRoot, CAPTURE_RENDER_LAYER,
	HOST_REPRESENTATIVE_POSE,
};
use std::collections::HashMap;

pub const HOST_FIREARM: &str = "generated-firearm";
pub const HOST_CLOTHING: &str = "clothing-preview";
pub const HOST_GRENADE: &str = "grenade-preview";
pub const HOST_SKILL_MAP: &str = "skill-map-preview";
pub const HOST_SELF_LUBIKAN: &str = "self-lubikan-preview";
pub const HOST_FISTICUFFS_ARMS: &str = "fisticuffs-arms-preview";

/// Clip + progress for a [`HOST_REPRESENTATIVE_POSE`] (or Fisticuffs) capture host.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseCaptureSpec {
	pub clip: AnimClip,
	pub progress: f32,
}

#[derive(Resource, Default, Debug)]
pub struct PoseCaptureSubjects {
	poses: HashMap<CaptureId, PoseCaptureSpec>,
}

impl PoseCaptureSubjects {
	pub fn remember(&mut self, id: CaptureId, spec: PoseCaptureSpec) {
		self.poses.insert(id, spec);
	}

	pub fn get(&self, id: &CaptureId) -> Option<PoseCaptureSpec> {
		self.poses.get(id).copied()
	}
}

/// Stamp the live clip onto a realized representative-pose host.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CapturePoseSpec {
	pub clip: AnimClip,
	pub progress: f32,
}

/// Fixed-assembly visual under a pose capture host. Drained without the world
/// streaming budget so isolated stills do not wait behind nearby mobs.
#[derive(Component)]
struct CapturePoseAssembly;

#[derive(Resource, Default, Debug)]
pub struct ItemCaptureSubjects {
	items: HashMap<CaptureId, InventoryItem>,
}

impl ItemCaptureSubjects {
	pub fn remember(&mut self, id: CaptureId, item: InventoryItem) {
		self.items.insert(id, item);
	}

	pub fn get(&self, id: &CaptureId) -> Option<&InventoryItem> {
		self.items.get(id)
	}

	pub fn items(&self) -> impl Iterator<Item = &InventoryItem> {
		self.items.values()
	}
}

#[derive(Component)]
pub struct CaptureHostRealized;

/// Bind-pose garment with catalog material — same inspect path as the menu preview.
#[derive(Clone, PartialEq, Component)]
struct ClothingIconPreview {
	layer: ClothingLayer,
}

impl Default for ClothingIconPreview {
	fn default() -> Self {
		Self {
			layer: ClothingLayer::new(
				ClothingMesh::TankTop,
				ItemColor::Natural,
				ClothingHost::HUMANOID,
			),
		}
	}
}

impl CharacterComponents for ClothingIconPreview {
	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("clothing", vec![self.layer.preview_part_node()])
	}
}

/// Bind-pose Braidman body — same isolated-preview path as clothing icons.
/// The live recipe skins this GLB to the body rig; the icon drops that so
/// a capture without a posed armature can still see the mesh.
#[derive(Clone, PartialEq, Component)]
#[cfg_attr(not(test), allow(dead_code))]
struct BraidmanIconPreview {
	body: PartNode,
}

impl Default for BraidmanIconPreview {
	fn default() -> Self {
		Self { body: braidman_icon_body() }
	}
}

impl CharacterComponents for BraidmanIconPreview {
	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("body", vec![self.body.clone()])
	}
}

fn braidman_icon_body() -> PartNode {
	let braidman = Braidman::from_config(&BraidmanConfig::default_preview());
	PartNode::glb(
		CharacterPartSlot::BodyMesh,
		braidman.body.label(),
		braidman.body.path().as_str(),
		AssetNormalization::IDENTITY,
	)
	.with_base_color(braidman.colors.color_for_slot(CharacterPartSlot::BodyMesh))
}

pub fn install_capture_hosts(app: &mut App) {
	if !app.is_plugin_added::<LodRefreshCorePlugin>() {
		app.add_plugins(LodRefreshCorePlugin);
	}
	if !app.is_plugin_added::<CharacterShadersPlugin>() {
		app.add_plugins(CharacterShadersPlugin);
	}
	if !app.is_plugin_added::<CharacterMaterialRefPlugin>() {
		app.add_plugins(CharacterMaterialRefPlugin);
	}
	if !app.is_plugin_added::<FirearmComponentsPlugin>() {
		app.add_plugins(FirearmComponentsPlugin);
	}
	add_firearm_components_host::<GeneratedFirearm>(app);
	if !app.is_plugin_added::<CharacterHostsPlugin>() {
		app.add_plugins(CharacterHostsPlugin);
	} else if !app.is_plugin_added::<CharacterComponentsPlugin>() {
		app.add_plugins(CharacterComponentsPlugin);
	}
	add_lod_refresh_chunk_for::<RigNode>(app);
	add_lod_refresh_chunk_for::<PartNode>(app);
	add_character_components_host::<ClothingIconPreview>(app);
	if !app.is_plugin_added::<GrenadeMaterialPlugin>() {
		app.add_plugins(GrenadeMaterialPlugin);
	}
	app.add_systems(
		Update,
		(
			drain_capture_pose_assemblies,
			pose_representative_hosts.before(CharacterMotionSystems::Anim),
			log_pose_host_wait,
		),
	);
}

pub fn remember_item(subjects: &mut ItemCaptureSubjects, item: &InventoryItem) {
	let Some(label) = item.capture_id_label() else {
		return;
	};
	subjects.remember(CaptureId::new(label), item.clone());
}

pub fn realize_capture_hosts(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut grenade_materials: Option<ResMut<Assets<GrenadeMaterial>>>,
	mut standard_materials: Option<ResMut<Assets<StandardMaterial>>>,
	subjects: Res<ItemCaptureSubjects>,
	poses: Res<PoseCaptureSubjects>,
	hosts: Query<(Entity, &CaptureHostRoot), Without<CaptureHostRealized>>,
) {
	for (root, host) in &hosts {
		if host.kind == HOST_SELF_LUBIKAN {
			let Some(materials) = standard_materials.as_mut() else {
				continue;
			};
			spawn_self_lubikan_host(&mut commands, &mut meshes, materials, root);
			commands.entity(root).try_insert(CaptureHostRealized);
			continue;
		}
		if host.kind == HOST_FISTICUFFS_ARMS {
			if spawn_fisticuffs_arms_host(&mut commands, root) {
				commands.entity(root).try_insert((
					CaptureHostRealized,
					CapturePoseSpec {
						clip: FisticuffsGuardPose::clip(),
						progress: FisticuffsGuardPose::progress(),
					},
				));
			}
			continue;
		}
		if let Some(item) = subjects.get(&host.id) {
			if spawn_item_host(
				&mut commands,
				&mut meshes,
				grenade_materials.as_deref_mut(),
				root,
				item,
			) {
				commands.entity(root).try_insert(CaptureHostRealized);
			}
			continue;
		}
		if host.kind != HOST_REPRESENTATIVE_POSE {
			continue;
		}
		let Some(spec) = poses.get(&host.id) else {
			continue;
		};
		if spawn_pose_host(&mut commands, root) {
			commands.entity(root).try_insert((
				CaptureHostRealized,
				CapturePoseSpec { clip: spec.clip, progress: spec.progress },
			));
		}
	}
}

/// Stamp the live clip onto a realized representative-pose (or Fisticuffs) host.
pub fn pose_representative_hosts(
	mut commands: Commands,
	hosts: Query<(Entity, &CapturePoseSpec), With<CaptureHostRealized>>,
	children: Query<&Children>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	for (root, spec) in &hosts {
		visit_descendants(root, &children, &mut |entity| {
			let Ok(rig) = rigs.get(entity) else {
				return;
			};
			if rig.role != CharacterRigRole::Body {
				return;
			}
			let desired = AnimRef::new(spec.clip);
			let needs = match anims.get(entity) {
				Ok(root) => root.0 != desired,
				Err(_) => true,
			};
			if needs {
				commands.entity(entity).try_insert(AnimRefRoot(desired));
			}
			commands.entity(entity).try_insert((
				AnimProgress(spec.progress),
				AnimateBones,
				AnimateEffects,
			));
		});
	}
}

fn spawn_pose_host(commands: &mut Commands, root: Entity) -> bool {
	let preview = BraidmanConfig::default_preview().clothed();
	let entity = spawn_fixed_character_assembly(commands, &preview, Transform::IDENTITY);
	commands.entity(entity).try_insert(CapturePoseAssembly);
	pin_capture_host(commands, entity, root);
	true
}

fn log_pose_host_wait(
	hosts: Query<
		(Entity, &CaptureHostRoot),
		(With<CapturePoseSpec>, With<CaptureHostRealized>, Without<CaptureHostReady>),
	>,
	children: Query<&Children>,
	pending_assembly: Query<(), With<PendingCharacterAssembly>>,
	hidden_until: Query<(), With<HiddenUntilSceneReady>>,
	meshes: Query<Entity, (With<Mesh3d>, Allow<Disabled>)>,
	mailboxes: Query<&AnimMailbox>,
	mut frames: Local<u32>,
) {
	if hosts.is_empty() {
		return;
	}
	*frames += 1;
	if *frames % 60 != 1 {
		return;
	}
	for (root, host) in &hosts {
		let mut pending = 0_u32;
		let mut hidden = 0_u32;
		let mut mesh_count = 0_u32;
		let mut mailbox_settled = false;
		visit_descendants(root, &children, &mut |entity| {
			if pending_assembly.get(entity).is_ok() {
				pending += 1;
			}
			if hidden_until.get(entity).is_ok() {
				hidden += 1;
			}
			if meshes.get(entity).is_ok() {
				mesh_count += 1;
			}
			if mailboxes.get(entity).is_ok_and(AnimMailbox::settled) {
				mailbox_settled = true;
			}
		});
		info!(
			id = %host.id.as_str(),
			pending,
			hidden,
			meshes = mesh_count,
			mailbox_settled,
			"pose capture host waiting"
		);
	}
}

fn drain_capture_pose_assemblies(world: &mut World) {
	let roots: Vec<Entity> = world
		.query_filtered::<Entity, (With<PendingCharacterAssembly>, With<CapturePoseAssembly>)>()
		.iter(world)
		.collect();
	if roots.is_empty() {
		return;
	}
	drain_character_assembly_now(world, roots);
}

pub fn mark_capture_hosts_ready(
	mut commands: Commands,
	hosts: Query<
		(Entity, &CaptureHostRoot),
		(With<CaptureHostRealized>, Without<CaptureHostReady>),
	>,
	children: Query<&Children>,
	pending: Query<
		(),
		Or<(
			With<LodLevelSpawnRequest>,
			With<LodLevelRootPending>,
			With<PendingCharacterAssembly>,
			With<HiddenUntilSceneReady>,
		)>,
	>,
	meshes: Query<Entity, (With<Mesh3d>, Allow<Disabled>)>,
	applied: Query<(), With<MaterialRefApplied>>,
	standard: Query<(), With<MeshMaterial3d<StandardMaterial>>>,
	clothing: Query<(), With<MeshMaterial3d<ClothingShaderMaterial>>>,
	faces: Query<(), With<MeshMaterial3d<FaceShaderMaterial>>>,
	grenades: Query<(), With<MeshMaterial3d<GrenadeMaterial>>>,
	poses: Query<(), With<ActiveRigPose>>,
	posed: Query<(), With<PoseApplied>>,
	pose_specs: Query<(), With<CapturePoseSpec>>,
	mailboxes: Query<&AnimMailbox>,
	pending_scene: Query<(), (With<SceneRefRoot>, Without<WorldAssetRoot>)>,
	bind_pose: Query<(), Or<(With<SelfLubikanBindPose>, With<FisticuffsArmsBindPose>)>>,
) {
	for (root, host) in &hosts {
		if (host.kind == HOST_SELF_LUBIKAN || host.kind == HOST_FISTICUFFS_ARMS)
			&& bind_pose.get(root).is_err()
		{
			continue;
		}
		if host_tree_ready(
			root,
			&children,
			&pending,
			&meshes,
			&applied,
			&standard,
			&clothing,
			&faces,
			&grenades,
			&poses,
			&posed,
			pose_specs.get(root).is_ok(),
			&mailboxes,
			&pending_scene,
		) {
			commands.entity(root).try_insert(CaptureHostReady);
		}
	}
}

/// Strip clothing/face/grenade shaders and keep the tree extractable. Clothing
/// and face shaders restamp in Update (`MaterialRefApplied` still counts as
/// painted). Grenades do not, so they get a [`StandardMaterial`] stand-in;
/// capture restamps unlit gray through settle/readback.
pub fn prepare_silhouette_hosts(
	mut commands: Commands,
	mut standard_materials: Option<ResMut<Assets<StandardMaterial>>>,
	roots: Query<Entity, With<CaptureSilhouetteRoot>>,
	children: Query<&Children>,
	clothing: Query<Entity, With<MeshMaterial3d<ClothingShaderMaterial>>>,
	faces: Query<Entity, With<MeshMaterial3d<FaceShaderMaterial>>>,
	grenades: Query<Entity, With<MeshMaterial3d<GrenadeMaterial>>>,
) {
	for root in &roots {
		visit_descendants(root, &children, &mut |entity| {
			if clothing.get(entity).is_ok() {
				commands.entity(entity).try_remove::<MeshMaterial3d<ClothingShaderMaterial>>();
			}
			if faces.get(entity).is_ok() {
				commands.entity(entity).try_remove::<MeshMaterial3d<FaceShaderMaterial>>();
			}
			if grenades.get(entity).is_ok() {
				commands.entity(entity).try_remove::<MeshMaterial3d<GrenadeMaterial>>();
				if let Some(materials) = standard_materials.as_mut() {
					commands.entity(entity).try_insert(MeshMaterial3d(materials.add(
						StandardMaterial { perceptual_roughness: 0.6, ..Default::default() },
					)));
				}
			}
		});
	}
}

pub fn keep_capture_hosts_visible(
	mut commands: Commands,
	roots: Query<Entity, (With<CaptureHostRoot>, Allow<Disabled>)>,
	children: Query<&Children, Allow<Disabled>>,
	hidden: Query<Option<&Visibility>, Or<(With<Disabled>, With<Visibility>)>>,
	arm_hidden: Query<(), (With<FisticuffsArmsHidden>, Allow<Disabled>)>,
) {
	for root in &roots {
		visit_descendants(root, &children, &mut |entity| {
			let Ok(visibility) = hidden.get(entity) else {
				return;
			};
			commands.entity(entity).try_remove::<Disabled>();
			// Only nodes explicitly hidden by arm isolation retain Hidden.
			// Other scene parts still receive generic visibility repair.
			if arm_hidden.contains(entity) && visibility == Some(&Visibility::Hidden) {
				return;
			}
			commands.entity(entity).try_insert(Visibility::Inherited);
		});
	}
}

fn spawn_item_host(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	grenade_materials: Option<&mut Assets<GrenadeMaterial>>,
	root: Entity,
	item: &InventoryItem,
) -> bool {
	match item {
		InventoryItem::Firearm { spec, .. } => {
			let preview = GeneratedFirearm::from_spec(*spec);
			for entity in spawn_firearm_components(
				commands,
				&preview,
				Transform::IDENTITY,
				firearm_bounds(&preview),
			) {
				pin_capture_host(commands, entity, root);
				commands
					.entity(entity)
					.try_insert(ActiveRigPose { pose: preview.icon_pose().to_resolved() });
			}
			true
		}
		InventoryItem::Clothing { mesh, material, .. } => {
			let preview = ClothingIconPreview {
				layer: ClothingLayer::new(*mesh, material.color, ClothingHost::HUMANOID)
					.with_material(material.id),
			};
			let entity = spawn_clothing_icon(commands, &preview);
			pin_capture_host(commands, entity, root);
			true
		}
		InventoryItem::Grenade { .. } => {
			let Some(materials) = grenade_materials else {
				return false;
			};
			let entity = commands
				.spawn((
					Name::new("capture-grenade"),
					Mesh3d(meshes.add(grenade_mesh())),
					MeshMaterial3d(materials.add(grenade_material())),
					Transform::IDENTITY,
					Visibility::default(),
				))
				.id();
			pin_capture_host(commands, entity, root);
			true
		}
		InventoryItem::SkillMap { .. } | InventoryItem::Tanz { .. } => false,
		InventoryItem::Fisticuffs => spawn_fisticuffs_arms_host(commands, root),
	}
}

fn spawn_fisticuffs_arms_host(commands: &mut Commands, root: Entity) -> bool {
	spawn_pose_host(commands, root)
}

fn spawn_self_lubikan_host(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	root: Entity,
) {
	// Scene lives on a child so WorldAssetRoot spawn stays off the host root.
	commands.spawn((
		Name::new("self-lubikan-body"),
		SceneRefRoot(braidman_icon_body().scene),
		Transform::IDENTITY,
		Visibility::default(),
		RenderLayers::layer(CAPTURE_RENDER_LAYER),
		ChildOf(root),
	));
	spawn_medical_cross(commands, meshes, materials, root);
}

/// Finished destemming the Self Lubikan body so bind-pose vertices draw.
#[derive(Component)]
pub(crate) struct SelfLubikanBindPose;

/// Finished destemming, posing, and isolating the Fisticuffs arms mesh.
#[derive(Component)]
pub(crate) struct FisticuffsArmsBindPose;

/// Visibility owned by the Fisticuffs arm mask, not scene readiness.
#[derive(Component)]
pub(crate) struct FisticuffsArmsHidden;

/// Isolated captures have no posed body rig. The GLB still instantiates
/// `SkinnedMesh` plus JOINTS/WEIGHTS; Bevy's PBR path skips those meshes
/// without a live armature, and dropping only `SkinnedMesh` leaves them
/// invisible. Clone the real Braidman mesh, strip the skin attributes, and
/// draw the authored bind pose as a regular `Mesh3d`.
pub fn rest_pose_self_lubikan_skins(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut standard_materials: Option<ResMut<Assets<StandardMaterial>>>,
	hosts: Query<
		(Entity, &CaptureHostRoot),
		(With<CaptureHostRealized>, Without<SelfLubikanBindPose>),
	>,
	children: Query<&Children>,
	pending_scene: Query<(), (With<SceneRefRoot>, Without<WorldAssetRoot>)>,
	mesh_3d: Query<&Mesh3d>,
	skinned: Query<Entity, With<SkinnedMesh>>,
	crosses: Query<(), With<MedicalCross>>,
) {
	for (root, host) in &hosts {
		if host.kind != HOST_SELF_LUBIKAN {
			continue;
		}
		let mut pending = false;
		visit_descendants(root, &children, &mut |entity| {
			if pending_scene.get(entity).is_ok() {
				pending = true;
			}
		});
		if pending {
			continue;
		}
		let mut destemmed_any = false;
		let mut saw_loaded_body = false;
		let mut pending_mesh = false;
		visit_descendants(root, &children, &mut |entity| {
			if entity == root {
				return;
			}
			commands.entity(entity).try_insert(RenderLayers::layer(CAPTURE_RENDER_LAYER));
			if crosses.get(entity).is_ok() {
				return;
			}
			let Ok(handle) = mesh_3d.get(entity) else {
				return;
			};
			let Some(mesh) = meshes.get(&handle.0) else {
				pending_mesh = true;
				return;
			};
			saw_loaded_body = true;
			if let Some(destemmed) = destem_bind_pose_mesh(mesh) {
				commands.entity(entity).try_insert(Mesh3d(meshes.add(destemmed)));
				destemmed_any = true;
			}
			// Opaque gray-ready paint — no GLB texture / alpha for silhouette.
			if let Some(materials) = standard_materials.as_mut() {
				commands.entity(entity).try_insert(MeshMaterial3d(materials.add(
					StandardMaterial {
						perceptual_roughness: 0.6,
						metallic: 0.0,
						alpha_mode: AlphaMode::Opaque,
						..Default::default()
					},
				)));
			}
			if skinned.get(entity).is_ok() {
				commands.entity(entity).try_remove::<SkinnedMesh>();
			}
		});
		if pending_mesh || !(saw_loaded_body || destemmed_any) {
			continue;
		}
		place_cross_in_corner(&mut commands, &meshes, root, &children, &mesh_3d, &crosses);
		commands.entity(root).try_insert(SelfLubikanBindPose);
	}
}

/// Pose has settled: hide non-arm parts and keep body triangles skinned to arm joints.
///
/// The live [`pose_representative_hosts`] clip supplies the guard. FitExtents
/// crops the remaining posed AABB; this filter does not invent a fold.
pub fn rest_pose_fisticuffs_arms_skins(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut standard_materials: Option<ResMut<Assets<StandardMaterial>>>,
	hosts: Query<
		(Entity, &CaptureHostRoot),
		(With<CaptureHostRealized>, Without<FisticuffsArmsBindPose>),
	>,
	children: Query<&Children>,
	pending_scene: Query<(), (With<SceneRefRoot>, Without<WorldAssetRoot>)>,
	pending_assembly: Query<(), With<PendingCharacterAssembly>>,
	hidden_until: Query<(), With<HiddenUntilSceneReady>>,
	mesh_3d: Query<&Mesh3d>,
	skins: Query<&SkinnedMesh>,
	maps: Query<&BoneMap>,
	members: Query<&CharacterMembers>,
	parts: Query<&PartNode>,
	mut visibilities: Query<&mut Visibility>,
	mailboxes: Query<&AnimMailbox>,
) {
	for (root, host) in &hosts {
		if host.kind != HOST_FISTICUFFS_ARMS {
			continue;
		}
		let mut pending = false;
		let mut mailbox_settled = false;
		let mut bone_map: Option<&BoneMap> = None;
		visit_descendants(root, &children, &mut |entity| {
			if pending_scene.get(entity).is_ok()
				|| pending_assembly.get(entity).is_ok()
				|| hidden_until.get(entity).is_ok()
			{
				pending = true;
			}
			if mailboxes.get(entity).is_ok_and(AnimMailbox::settled) {
				mailbox_settled = true;
			}
			if bone_map.is_none() {
				if let Ok(map) = maps.get(entity) {
					bone_map = Some(map);
				}
			}
		});
		if pending || !mailbox_settled {
			continue;
		}
		let Some(map) = bone_map else {
			continue;
		};
		visit_descendants(root, &children, &mut |entity| {
			if let Ok(list) = members.get(entity) {
				hide_socketed_parts(
					list,
					&parts,
					&mut visibilities,
					FisticuffsArmMask::hides_part,
					true,
				);
				for member in list.iter() {
					if parts.get(member).is_ok_and(|part| FisticuffsArmMask::hides_part(part.slot))
					{
						commands.entity(member).try_insert(FisticuffsArmsHidden);
					}
				}
			}
		});
		let mut isolated_any = false;
		let mut saw_loaded_body = false;
		let mut pending_mesh = false;
		visit_descendants(root, &children, &mut |entity| {
			if entity == root {
				return;
			}
			commands.entity(entity).try_insert(RenderLayers::layer(CAPTURE_RENDER_LAYER));
			let Ok(handle) = mesh_3d.get(entity) else {
				return;
			};
			let Some(mesh) = meshes.get(&handle.0) else {
				pending_mesh = true;
				return;
			};
			saw_loaded_body = true;
			let Ok(skin) = skins.get(entity) else {
				commands.entity(entity).try_insert((Visibility::Hidden, FisticuffsArmsHidden));
				return;
			};
			let joint_is_arm: Vec<bool> = skin
				.joints
				.iter()
				.map(|joint| {
					map.by_name
						.iter()
						.any(|(name, &bone)| bone == *joint && FisticuffsArmMask::shows_joint(name))
				})
				.collect();
			let Some(isolated) = isolate_arm_joints(mesh, &joint_is_arm) else {
				commands.entity(entity).try_insert((Visibility::Hidden, FisticuffsArmsHidden));
				return;
			};
			commands.entity(entity).try_insert(Mesh3d(meshes.add(isolated)));
			isolated_any = true;
			if let Some(materials) = standard_materials.as_mut() {
				commands.entity(entity).try_insert(MeshMaterial3d(materials.add(
					StandardMaterial {
						perceptual_roughness: 0.6,
						metallic: 0.0,
						alpha_mode: AlphaMode::Opaque,
						..Default::default()
					},
				)));
			}
		});
		if pending_mesh || !(saw_loaded_body && isolated_any) {
			continue;
		}
		commands.entity(root).try_insert(FisticuffsArmsBindPose);
	}
}

fn place_cross_in_corner(
	commands: &mut Commands,
	meshes: &Assets<Mesh>,
	root: Entity,
	children: &Query<&Children>,
	mesh_3d: &Query<&Mesh3d>,
	crosses: &Query<(), With<MedicalCross>>,
) {
	let mut best: Option<(Entity, Vec3, Vec3)> = None;
	visit_descendants(root, children, &mut |entity| {
		if entity == root || crosses.get(entity).is_ok() {
			return;
		}
		let Ok(handle) = mesh_3d.get(entity) else {
			return;
		};
		let Some(mesh) = meshes.get(&handle.0) else {
			return;
		};
		let Some(local) = mesh.get_aabb() else {
			return;
		};
		let min = Vec3::from(local.min());
		let max = Vec3::from(local.max());
		let extent = (max - min).max_element();
		if best
			.as_ref()
			.is_none_or(|(_, bmin, bmax)| extent > (*bmax - *bmin).max_element())
		{
			best = Some((entity, min, max));
		}
	});
	let Some((body, min, max)) = best else {
		return;
	};
	let size = max - min;
	if size.max_element() < 1e-3 {
		return;
	}
	// Mesh-local top-right, so FitExtents puts the mark in the icon corner.
	let corner = Vec3::new(max.x + size.x * 0.08, max.y + size.y * 0.08, max.z + size.z * 0.04);
	let scale = (size.max_element() / 1.8).clamp(0.4, 12.0);
	visit_descendants(root, children, &mut |entity| {
		if crosses.get(entity).is_ok() {
			commands.entity(entity).try_insert((
				ChildOf(body),
				Transform { translation: corner, scale: Vec3::splat(scale), ..Default::default() },
			));
		}
	});
}

/// Bind-pose vertices are already in `ATTRIBUTE_POSITION`. Joint channels
/// only keep the mesh on the skinned extract path.
fn destem_bind_pose_mesh(mesh: &Mesh) -> Option<Mesh> {
	if mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_none()
		&& mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_none()
	{
		return None;
	}
	let mut destemmed = mesh.clone();
	destemmed.remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
	destemmed.remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
	Some(destemmed)
}

/// Scene-space badge so destem can place it after the body AABB is known.
#[derive(Component)]
pub(crate) struct MedicalCross;

fn spawn_medical_cross(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	root: Entity,
) {
	let material =
		materials.add(StandardMaterial { perceptual_roughness: 0.6, ..Default::default() });
	// Corner badge: two cuboids so the cross is a real scene object.
	// Silhouette restamp turns it unlit gray with the body.
	for (name, size) in [
		("medical-cross-vertical", Vec3::new(0.05, 0.22, 0.05)),
		("medical-cross-horizontal", Vec3::new(0.16, 0.05, 0.05)),
	] {
		let entity = commands
			.spawn((
				Name::new(name),
				MedicalCross,
				Mesh3d(meshes.add(Cuboid::from_size(size))),
				MeshMaterial3d(material.clone()),
				Transform::from_xyz(1.2, 1.2, 0.18),
				Visibility::default(),
				RenderLayers::layer(CAPTURE_RENDER_LAYER),
			))
			.id();
		// No LodRefreshMembership: isolated captures have no LodViewer, and
		// membership disables the cuboids before readback.
		commands.entity(entity).try_insert(ChildOf(root));
	}
}

fn pin_capture_host(commands: &mut Commands, entity: Entity, root: Entity) {
	commands.entity(entity).try_insert((
		ChildOf(root),
		LodRefreshMembership(LodRefreshDomain::of::<CaptureHostRoot>()),
	));
}

fn spawn_clothing_icon(commands: &mut Commands, preview: &ClothingIconPreview) -> Entity {
	spawn_character_icon(commands, preview)
}

fn spawn_character_icon<T: CharacterComponents + Component + Clone + Default + Unpin>(
	commands: &mut Commands,
	preview: &T,
) -> Entity {
	let bounds = character_bounds(preview);
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let host = ComponentsOnly(preview.clone());
	commands
		.spawn_scene((
			host.host(&lod_ref),
			bsn! {
				Transform::IDENTITY
				Visibility::default()
			},
		))
		.id()
}

fn host_tree_ready(
	root: Entity,
	children: &Query<&Children>,
	pending: &Query<
		(),
		Or<(
			With<LodLevelSpawnRequest>,
			With<LodLevelRootPending>,
			With<PendingCharacterAssembly>,
			With<HiddenUntilSceneReady>,
		)>,
	>,
	meshes: &Query<Entity, (With<Mesh3d>, Allow<Disabled>)>,
	applied: &Query<(), With<MaterialRefApplied>>,
	standard: &Query<(), With<MeshMaterial3d<StandardMaterial>>>,
	clothing: &Query<(), With<MeshMaterial3d<ClothingShaderMaterial>>>,
	faces: &Query<(), With<MeshMaterial3d<FaceShaderMaterial>>>,
	grenades: &Query<(), With<MeshMaterial3d<GrenadeMaterial>>>,
	poses: &Query<(), With<ActiveRigPose>>,
	posed: &Query<(), With<PoseApplied>>,
	require_mailbox: bool,
	mailboxes: &Query<&AnimMailbox>,
	pending_scene: &Query<(), (With<SceneRefRoot>, Without<WorldAssetRoot>)>,
) -> bool {
	let mut found_mesh = false;
	let mut is_pending = false;
	let mut unpainted = false;
	let mut needs_pose = false;
	let mut pose_ready = false;
	let mut mailbox_ready = !require_mailbox;
	visit_descendants(root, children, &mut |entity| {
		if pending.get(entity).is_ok() || pending_scene.get(entity).is_ok() {
			is_pending = true;
		}
		if poses.get(entity).is_ok() {
			needs_pose = true;
		}
		if posed.get(entity).is_ok() {
			pose_ready = true;
		}
		if require_mailbox && mailboxes.get(entity).is_ok_and(AnimMailbox::settled) {
			mailbox_ready = true;
		}
		if meshes.get(entity).is_err() {
			return;
		}
		found_mesh = true;
		if applied.get(entity).is_err()
			&& standard.get(entity).is_err()
			&& clothing.get(entity).is_err()
			&& faces.get(entity).is_err()
			&& grenades.get(entity).is_err()
		{
			unpainted = true;
		}
	});
	found_mesh && !is_pending && !unpainted && (!needs_pose || pose_ready) && mailbox_ready
}

fn visit_descendants<F: QueryFilter>(
	root: Entity,
	children: &Query<&Children, F>,
	visit: &mut impl FnMut(Entity),
) {
	visit(root);
	let Ok(kids) = children.get(root) else {
		return;
	};
	for child in kids.iter() {
		visit_descendants(child, children, visit);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::{FirearmMesh, FirearmSpec};

	#[test]
	fn clothing_preview_uses_the_humanoid_fitted_mesh() {
		let layer = ClothingLayer::new(ClothingMesh::Pants, ItemColor::Red, ClothingHost::HUMANOID)
			.with_material(character_items::ClothingMaterial::Cloth);
		let preview = ClothingIconPreview { layer: layer.clone() };
		let nodes = preview.part_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(nodes.len(), 1);
		assert_eq!(layer.preview_part_node().scene, nodes[0].scene);
	}

	#[test]
	fn generated_firearm_is_the_capture_host() {
		let spec = FirearmSpec::from_mesh(FirearmMesh::Silopup);
		let item = InventoryItem::from_firearm_spec(spec);
		assert!(item.capture_id_label().is_some());
		let _ = GeneratedFirearm::from_spec(spec);
	}

	#[test]
	fn grenade_host_uses_the_thrown_mesh() {
		let item = InventoryItem::standard_grenade();
		assert!(item.capture_id_label().is_some());
		assert_eq!(HOST_GRENADE, "grenade-preview");
	}

	#[test]
	fn self_lubikan_host_uses_the_preview_braidman() {
		let config = BraidmanConfig::default_preview();
		assert!(config.clothing.is_empty());
		let braidman = Braidman::from_config(&config);
		let preview = BraidmanIconPreview::default();
		let nodes = preview.part_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(nodes.len(), 1);
		assert_eq!(nodes[0].slot, CharacterPartSlot::BodyMesh);
		assert_eq!(nodes[0].scene, braidman.body.path().scene_ref());
		assert!(nodes[0].skin.is_none(), "icon body must be bind-pose, like clothing");
		assert_eq!(HOST_SELF_LUBIKAN, "self-lubikan-preview");
	}

	#[test]
	fn silhouette_prep_keeps_the_grenade_mesh_painted() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.init_asset::<GrenadeMaterial>()
			.add_systems(Update, prepare_silhouette_hosts);
		let grenade = app
			.world_mut()
			.resource_mut::<Assets<GrenadeMaterial>>()
			.add(grenade_material());
		let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Sphere::new(0.5));
		let root = app.world_mut().spawn(CaptureSilhouetteRoot).id();
		let child = app
			.world_mut()
			.spawn((Mesh3d(mesh), MeshMaterial3d(grenade), ChildOf(root)))
			.id();
		app.update();
		assert!(app.world().get::<MeshMaterial3d<GrenadeMaterial>>(child).is_none());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(child).is_some());
	}

	#[test]
	fn capture_hosts_use_an_isolated_lod_domain() {
		let domain = LodRefreshDomain::of::<CaptureHostRoot>();
		assert_eq!(domain, LodRefreshDomain::of::<CaptureHostRoot>());
		assert_ne!(domain, LodRefreshDomain::of::<GeneratedFirearm>());
	}

	#[test]
	fn destem_bind_pose_strips_joint_channels() {
		use bevy::asset::RenderAssetUsages;
		use bevy::mesh::{PrimitiveTopology, VertexAttributeValues};

		let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
			.with_inserted_attribute(
				Mesh::ATTRIBUTE_POSITION,
				vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
			)
			.with_inserted_attribute(
				Mesh::ATTRIBUTE_JOINT_INDEX,
				VertexAttributeValues::Uint16x4(vec![[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]),
			)
			.with_inserted_attribute(
				Mesh::ATTRIBUTE_JOINT_WEIGHT,
				vec![[1.0_f32, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]],
			);
		let destemmed = destem_bind_pose_mesh(&mesh).expect("jointed mesh destems");
		assert!(destemmed.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_none());
		assert!(destemmed.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_none());
		assert!(destemmed.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
		assert!(destem_bind_pose_mesh(&destemmed).is_none());
	}

	#[test]
	fn self_lubikan_waits_for_loaded_body_mesh() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.add_systems(Update, rest_pose_self_lubikan_skins);
		let root = app
			.world_mut()
			.spawn((
				CaptureHostRoot {
					kind: HOST_SELF_LUBIKAN.to_string(),
					id: CaptureId::new("self-lubikan"),
				},
				CaptureHostRealized,
			))
			.id();
		app.world_mut().spawn((Mesh3d(Handle::default()), ChildOf(root)));
		app.update();
		assert!(
			app.world().get::<SelfLubikanBindPose>(root).is_none(),
			"Mesh3d without an asset must not ready the host"
		);
	}

	#[test]
	fn self_lubikan_does_not_ready_on_cross_only() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.add_systems(Update, rest_pose_self_lubikan_skins);
		let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Cuboid::from_length(0.1));
		let root = app
			.world_mut()
			.spawn((
				CaptureHostRoot {
					kind: HOST_SELF_LUBIKAN.to_string(),
					id: CaptureId::new("self-lubikan"),
				},
				CaptureHostRealized,
			))
			.id();
		app.world_mut().spawn((MedicalCross, Mesh3d(mesh), ChildOf(root)));
		app.update();
		assert!(app.world().get::<SelfLubikanBindPose>(root).is_none());
	}

	#[test]
	fn self_lubikan_rest_pose_drops_skinned_mesh() {
		use bevy::asset::RenderAssetUsages;
		use bevy::mesh::{PrimitiveTopology, VertexAttributeValues};

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>()
			.add_systems(Update, rest_pose_self_lubikan_skins);
		let inverse = app
			.world_mut()
			.resource_mut::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
			.add(bevy::mesh::skinning::SkinnedMeshInverseBindposes::from(vec![Mat4::IDENTITY]));
		let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(
			Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
				.with_inserted_attribute(
					Mesh::ATTRIBUTE_POSITION,
					vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
				)
				.with_inserted_attribute(
					Mesh::ATTRIBUTE_JOINT_INDEX,
					VertexAttributeValues::Uint16x4(vec![[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]),
				)
				.with_inserted_attribute(
					Mesh::ATTRIBUTE_JOINT_WEIGHT,
					vec![[1.0_f32, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]],
				),
		);
		let root = app
			.world_mut()
			.spawn((
				CaptureHostRoot {
					kind: HOST_SELF_LUBIKAN.to_string(),
					id: CaptureId::new("self-lubikan"),
				},
				CaptureHostRealized,
			))
			.id();
		let child = app
			.world_mut()
			.spawn((
				Mesh3d(mesh),
				SkinnedMesh { inverse_bindposes: inverse, joints: vec![root] },
				ChildOf(root),
			))
			.id();
		let cross_mesh =
			app.world_mut().resource_mut::<Assets<Mesh>>().add(Cuboid::from_length(0.1));
		let cross = app
			.world_mut()
			.spawn((
				MedicalCross,
				Mesh3d(cross_mesh),
				Transform::from_xyz(1.2, 1.2, 0.18),
				ChildOf(root),
			))
			.id();
		app.update();
		assert!(app.world().get::<SkinnedMesh>(child).is_none());
		assert!(app.world().get::<SelfLubikanBindPose>(root).is_some());
		let handle = app.world().get::<Mesh3d>(child).expect("destemmed mesh");
		let meshes = app.world().resource::<Assets<Mesh>>();
		let destemmed = meshes.get(&handle.0).expect("mesh asset");
		assert!(destemmed.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_none());
		assert!(destemmed.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_none());
		assert_eq!(app.world().get::<ChildOf>(cross).map(|parent| parent.0), Some(child));
		let placed = app.world().get::<Transform>(cross).expect("corner");
		assert!(placed.translation.x > 0.0 && placed.translation.y > 0.0);
	}

	#[test]
	fn fisticuffs_waits_for_loaded_body_mesh() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.add_systems(Update, rest_pose_fisticuffs_arms_skins);
		let root = app
			.world_mut()
			.spawn((
				CaptureHostRoot {
					kind: HOST_FISTICUFFS_ARMS.to_string(),
					id: CaptureId::new("fisticuffs:braidman-arms"),
				},
				CaptureHostRealized,
			))
			.id();
		app.world_mut().spawn((Mesh3d(Handle::default()), ChildOf(root)));
		app.update();
		assert!(
			app.world().get::<FisticuffsArmsBindPose>(root).is_none(),
			"Mesh3d without an asset must not ready the host"
		);
	}

	#[test]
	fn fisticuffs_arm_mask_survives_visibility_repair_after_asset_readiness() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, keep_capture_hosts_visible);
		let root = app
			.world_mut()
			.spawn(CaptureHostRoot {
				kind: HOST_FISTICUFFS_ARMS.to_string(),
				id: CaptureId::new("fisticuffs:braidman-arms"),
			})
			.id();
		let arm = app.world_mut().spawn((Visibility::Hidden, ChildOf(root))).id();
		let face = app.world_mut().spawn((Visibility::Hidden, ChildOf(root))).id();
		let hidden_child = app.world_mut().spawn((Visibility::Hidden, ChildOf(face))).id();
		app.update();
		assert_eq!(app.world().get::<Visibility>(arm), Some(&Visibility::Inherited));
		assert_eq!(app.world().get::<Visibility>(face), Some(&Visibility::Inherited));

		// Once the asset and arm isolation are ready, the masked face stays
		// hidden; unrelated Hidden nodes are still repaired.
		app.world_mut().entity_mut(root).insert(FisticuffsArmsBindPose);
		app.world_mut()
			.entity_mut(face)
			.insert((Visibility::Hidden, FisticuffsArmsHidden));
		app.world_mut().entity_mut(hidden_child).insert(Visibility::Hidden);
		let late_part = app.world_mut().spawn((Visibility::Hidden, ChildOf(root))).id();
		for _ in 0..3 {
			app.update();
			assert_eq!(app.world().get::<Visibility>(face), Some(&Visibility::Hidden));
			assert_eq!(app.world().get::<Visibility>(hidden_child), Some(&Visibility::Inherited));
			assert_eq!(app.world().get::<Visibility>(late_part), Some(&Visibility::Inherited));
		}
		assert_eq!(app.world().get::<Visibility>(arm), Some(&Visibility::Inherited));
	}

	#[test]
	fn visibility_repair_traverses_disabled_hierarchies_and_preserves_arm_masks() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, keep_capture_hosts_visible);
		let root = app
			.world_mut()
			.spawn((
				CaptureHostRoot {
					kind: HOST_FISTICUFFS_ARMS.to_string(),
					id: CaptureId::new("fisticuffs:disabled-arms"),
				},
				Disabled,
				Visibility::Hidden,
			))
			.id();
		// The intermediate entity has no Visibility; it must still be traversed
		// and enabled so its visible arm descendants can render.
		let branch = app.world_mut().spawn((Disabled, ChildOf(root))).id();
		let arm = app.world_mut().spawn((Disabled, Visibility::Hidden, ChildOf(branch))).id();
		let masked = app
			.world_mut()
			.spawn((Disabled, Visibility::Hidden, FisticuffsArmsHidden, ChildOf(branch)))
			.id();
		let masked_child = app
			.world_mut()
			.spawn((Disabled, Visibility::Hidden, FisticuffsArmsHidden, ChildOf(masked)))
			.id();
		let ordinary_child =
			app.world_mut().spawn((Disabled, Visibility::Hidden, ChildOf(masked))).id();
		for _ in 0..3 {
			app.update();
			for entity in [root, branch, arm, masked, masked_child, ordinary_child] {
				assert!(
					app.world().get::<Disabled>(entity).is_none(),
					"capture descendants must be enabled"
				);
			}
			assert_eq!(app.world().get::<Visibility>(root), Some(&Visibility::Inherited));
			assert_eq!(app.world().get::<Visibility>(arm), Some(&Visibility::Inherited));
			assert_eq!(app.world().get::<Visibility>(ordinary_child), Some(&Visibility::Inherited));
			assert_eq!(app.world().get::<Visibility>(masked), Some(&Visibility::Hidden));
			assert_eq!(app.world().get::<Visibility>(masked_child), Some(&Visibility::Hidden));
		}
	}

	#[test]
	fn fisticuffs_host_uses_the_guard_pose_spec() {
		assert_eq!(FisticuffsGuardPose::clip(), AnimClip::jab());
		assert_eq!(FisticuffsGuardPose::progress(), AnimClip::fisticuffs_guard_progress());
		assert_eq!(HOST_FISTICUFFS_ARMS, "fisticuffs-arms-preview");
		assert_eq!(HOST_REPRESENTATIVE_POSE, "representative-pose");
	}

	#[test]
	fn pose_representative_hosts_stamps_the_guard_clip() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, pose_representative_hosts);
		let root = app
			.world_mut()
			.spawn((
				CaptureHostRealized,
				CapturePoseSpec {
					clip: FisticuffsGuardPose::clip(),
					progress: FisticuffsGuardPose::progress(),
				},
			))
			.id();
		let rig = app
			.world_mut()
			.spawn((CharacterRig { role: CharacterRigRole::Body, ..default() }, ChildOf(root)))
			.id();
		app.update();
		let stamped = app.world().get::<AnimRefRoot>(rig).expect("anim");
		assert_eq!(stamped.0, AnimRef::new(AnimClip::jab()));
		assert_eq!(app.world().get::<AnimProgress>(rig).map(|progress| progress.0), Some(0.0));
		assert!(app.world().get::<AnimateBones>(rig).is_some());
	}
}
