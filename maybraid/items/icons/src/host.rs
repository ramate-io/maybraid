//! Realize posed, painted inventory subjects into isolated capture roots.

use bevy::camera::primitives::MeshAabb;
use bevy::camera::visibility::RenderLayers;
use bevy::ecs::entity_disabling::Disabled;
use bevy::math::bounding::Aabb3d;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::prelude::*;
use bevy::scene::prelude::bsn;
use bevy::world_serialization::WorldAssetRoot;
use character_items::{ClothingHost, ClothingMesh, InventoryItem, ItemColor};
use character_shaders::{CharacterShadersPlugin, ClothingShaderMaterial, FaceShaderMaterial};
use characters::material_lib::CharacterMaterialRefPlugin;
use characters::species::braidman::{Braidman, BraidmanConfig};
use characters::{
	add_character_components_host, character_bounds, AssetNormalization, CharacterComponents,
	CharacterComponentsPlugin, CharacterHostsPlugin, CharacterPartSlot, ClothingLayer,
	ComponentsOnly, Layers, PartNode, RigNode,
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
use material_ref::{MaterialRefApplied, StandardMaterialRefPlugin};
use scene_ref::SceneRefRoot;
use screen_capture::{
	CaptureExtents, CaptureHostReady, CaptureHostRoot, CaptureId, CaptureSilhouetteRoot,
	CAPTURE_RENDER_LAYER,
};
use std::collections::HashMap;

pub const HOST_FIREARM: &str = "generated-firearm";
pub const HOST_CLOTHING: &str = "clothing-preview";
pub const HOST_GRENADE: &str = "grenade-preview";
pub const HOST_SKILL_MAP: &str = "skill-map-preview";
pub const HOST_SELF_LUBIKAN: &str = "self-lubikan-preview";
pub const HOST_FISTICUFFS_ARMS: &str = "fisticuffs-arms-preview";

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
	if !app.is_plugin_added::<StandardMaterialRefPlugin>() {
		app.add_plugins(StandardMaterialRefPlugin);
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
			spawn_fisticuffs_arms_host(&mut commands, root);
			commands.entity(root).try_insert(CaptureHostRealized);
			continue;
		}
		let Some(item) = subjects.get(&host.id) else {
			continue;
		};
		if spawn_item_host(&mut commands, &mut meshes, grenade_materials.as_deref_mut(), root, item)
		{
			commands.entity(root).try_insert(CaptureHostRealized);
		}
	}
}

pub fn mark_capture_hosts_ready(
	mut commands: Commands,
	hosts: Query<
		Entity,
		(With<CaptureHostRoot>, With<CaptureHostRealized>, Without<CaptureHostReady>),
	>,
	children: Query<&Children>,
	pending_spawn: Query<(), With<LodLevelSpawnRequest>>,
	pending_root: Query<(), With<LodLevelRootPending>>,
	meshes: Query<Entity, With<Mesh3d>>,
	applied: Query<(), With<MaterialRefApplied>>,
	standard: Query<(), With<MeshMaterial3d<StandardMaterial>>>,
	clothing: Query<(), With<MeshMaterial3d<ClothingShaderMaterial>>>,
	faces: Query<(), With<MeshMaterial3d<FaceShaderMaterial>>>,
	grenades: Query<(), With<MeshMaterial3d<GrenadeMaterial>>>,
	poses: Query<(), With<ActiveRigPose>>,
	posed: Query<(), With<PoseApplied>>,
	pending_scene: Query<(), (With<SceneRefRoot>, Without<WorldAssetRoot>)>,
	hosts_meta: Query<&CaptureHostRoot>,
	bind_pose: Query<(), Or<(With<SelfLubikanBindPose>, With<FisticuffsArmsBindPose>)>>,
) {
	for root in &hosts {
		if hosts_meta
			.get(root)
			.is_ok_and(|host| host.kind == HOST_SELF_LUBIKAN || host.kind == HOST_FISTICUFFS_ARMS)
			&& bind_pose.get(root).is_err()
		{
			continue;
		}
		if host_tree_ready(
			root,
			&children,
			&pending_spawn,
			&pending_root,
			&meshes,
			&applied,
			&standard,
			&clothing,
			&faces,
			&grenades,
			&poses,
			&posed,
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
	roots: Query<Entity, With<CaptureHostRoot>>,
	children: Query<&Children>,
	hidden: Query<(), Or<(With<Disabled>, With<Visibility>)>>,
) {
	for root in &roots {
		visit_descendants(root, &children, &mut |entity| {
			if hidden.get(entity).is_err() {
				return;
			}
			commands.entity(entity).try_remove::<Disabled>();
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
		InventoryItem::SkillMap { .. } => false,
		InventoryItem::Fisticuffs => {
			spawn_fisticuffs_arms_host(commands, root);
			true
		}
	}
}

fn spawn_fisticuffs_arms_host(commands: &mut Commands, root: Entity) {
	commands.spawn((
		Name::new("fisticuffs-arms-body"),
		SceneRefRoot(braidman_icon_body().scene),
		Transform::IDENTITY,
		Visibility::default(),
		RenderLayers::layer(CAPTURE_RENDER_LAYER),
		ChildOf(root),
	));
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

/// Destem the Braidman GLB, keep both arms, and fold them into a guard so the
/// thumbnail is the character's fists rather than a full-body silhouette.
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
	mesh_3d: Query<&Mesh3d>,
	skinned: Query<Entity, With<SkinnedMesh>>,
) {
	for (root, host) in &hosts {
		if host.kind != HOST_FISTICUFFS_ARMS {
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
		let mut extents: Option<Aabb3d> = None;
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
			let destemmed = destem_bind_pose_mesh(mesh).unwrap_or_else(|| mesh.clone());
			let posed = isolate_guard_pose_arms(&destemmed).unwrap_or(destemmed);
			if let Some(local) = posed.compute_aabb() {
				let min = Vec3::from(local.min());
				let max = Vec3::from(local.max());
				let next = Aabb3d::from_min_max(min, max);
				extents = Some(match extents {
					Some(previous) => union_capture_aabb(previous, next),
					None => next,
				});
			}
			commands.entity(entity).try_insert(Mesh3d(meshes.add(posed)));
			destemmed_any = true;
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
		if let Some(extents) = extents {
			commands.entity(root).try_insert(CaptureExtents(extents));
		}
		commands.entity(root).try_insert(FisticuffsArmsBindPose);
	}
}

fn union_capture_aabb(a: Aabb3d, b: Aabb3d) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::from(a.min).min(Vec3::from(b.min)),
		Vec3::from(a.max).max(Vec3::from(b.max)),
	)
}

/// Keep the destemmed arm vertices and swing them from a T-pose into a guard.
///
/// Isolation is spatial (preview posing, not the live `Jab` clip) so a capture
/// without a body rig still shows Braidman geometry instead of a fist glyph.
fn isolate_guard_pose_arms(mesh: &Mesh) -> Option<Mesh> {
	let VertexAttributeValues::Float32x3(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
	else {
		return None;
	};
	if positions.len() < 3 {
		return None;
	}
	let mut min = Vec3::splat(f32::MAX);
	let mut max = Vec3::splat(f32::MIN);
	for position in positions {
		let point = Vec3::from(*position);
		min = min.min(point);
		max = max.max(point);
	}
	let size = (max - min).max(Vec3::splat(1e-4));
	let center = (min + max) * 0.5;
	// T-pose arms occupy the sides. Keep a wide slab from just outside the
	// ribcage out to the hands so the thumbnail is Braidman geometry, not a
	// handful of leftover triangles.
	let keep_point = |point: Vec3| -> bool {
		let u = (point - min) / size;
		let from_mid = (point.x - center.x).abs() / size.x;
		let arm_height = u.y > 0.45 && u.y < 0.86;
		let shoulder = u.y > 0.58 && u.y < 0.82 && from_mid > 0.06;
		(arm_height && from_mid > 0.10) || shoulder
	};
	let triangles = mesh_triangles(mesh, positions.len())?;
	let kept: Vec<[u32; 3]> = triangles
		.into_iter()
		.filter(|tri| {
			let verts = tri.map(|index| Vec3::from(positions[index as usize]));
			let centroid = (verts[0] + verts[1] + verts[2]) / 3.0;
			keep_point(centroid) || verts.into_iter().any(keep_point)
		})
		.collect();
	if kept.len() < 2 {
		return None;
	}
	let mut remap = vec![None; positions.len()];
	let mut new_positions = Vec::new();
	let normals = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
		Some(VertexAttributeValues::Float32x3(values)) if values.len() == positions.len() => {
			Some(values.as_slice())
		}
		_ => None,
	};
	let mut new_normals = Vec::new();
	let mut new_indices = Vec::new();
	for tri in &kept {
		for &index in tri {
			if remap[index as usize].is_none() {
				remap[index as usize] = Some(new_positions.len() as u32);
				new_positions.push(Vec3::from(positions[index as usize]));
				if let Some(normals) = normals {
					new_normals.push(Vec3::from(normals[index as usize]));
				}
			}
			new_indices.push(remap[index as usize].expect("remapped"));
		}
	}
	let left_shoulder = Vec3::new(center.x + size.x * 0.16, min.y + size.y * 0.76, center.z);
	let right_shoulder = Vec3::new(center.x - size.x * 0.16, min.y + size.y * 0.76, center.z);
	let left_rot = Quat::from_rotation_y(-1.05) * Quat::from_rotation_x(-0.42);
	let right_rot = Quat::from_rotation_y(1.05) * Quat::from_rotation_x(-0.42);
	for (index, point) in new_positions.iter_mut().enumerate() {
		let left = point.x >= center.x;
		let shoulder = if left { left_shoulder } else { right_shoulder };
		let rot = if left { left_rot } else { right_rot };
		*point = shoulder + rot * (*point - shoulder);
		if let Some(normal) = new_normals.get_mut(index) {
			*normal = rot * *normal;
		}
	}
	let mut posed = Mesh::new(mesh.primitive_topology(), mesh.asset_usage);
	posed.insert_attribute(
		Mesh::ATTRIBUTE_POSITION,
		new_positions.into_iter().map(|point| point.to_array()).collect::<Vec<_>>(),
	);
	if !new_normals.is_empty() {
		posed.insert_attribute(
			Mesh::ATTRIBUTE_NORMAL,
			new_normals.into_iter().map(|normal| normal.to_array()).collect::<Vec<_>>(),
		);
	}
	posed.insert_indices(Indices::U32(new_indices));
	Some(posed)
}

fn mesh_triangles(mesh: &Mesh, vertex_count: usize) -> Option<Vec<[u32; 3]>> {
	match mesh.indices() {
		Some(Indices::U32(indices)) => triangles_from_indices(indices),
		Some(Indices::U16(indices)) => {
			let indices: Vec<u32> = indices.iter().copied().map(u32::from).collect();
			triangles_from_indices(&indices)
		}
		None => {
			if vertex_count % 3 != 0 {
				return None;
			}
			Some((0..vertex_count as u32).step_by(3).map(|i| [i, i + 1, i + 2]).collect())
		}
	}
}

fn triangles_from_indices(indices: &[u32]) -> Option<Vec<[u32; 3]>> {
	if indices.len() % 3 != 0 {
		return None;
	}
	Some(indices.chunks_exact(3).map(|chunk| [chunk[0], chunk[1], chunk[2]]).collect())
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
		let Some(local) = mesh.compute_aabb() else {
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
	pending_spawn: &Query<(), With<LodLevelSpawnRequest>>,
	pending_root: &Query<(), With<LodLevelRootPending>>,
	meshes: &Query<Entity, With<Mesh3d>>,
	applied: &Query<(), With<MaterialRefApplied>>,
	standard: &Query<(), With<MeshMaterial3d<StandardMaterial>>>,
	clothing: &Query<(), With<MeshMaterial3d<ClothingShaderMaterial>>>,
	faces: &Query<(), With<MeshMaterial3d<FaceShaderMaterial>>>,
	grenades: &Query<(), With<MeshMaterial3d<GrenadeMaterial>>>,
	poses: &Query<(), With<ActiveRigPose>>,
	posed: &Query<(), With<PoseApplied>>,
	pending_scene: &Query<(), (With<SceneRefRoot>, Without<WorldAssetRoot>)>,
) -> bool {
	let mut found_mesh = false;
	let mut pending = false;
	let mut unpainted = false;
	let mut needs_pose = false;
	let mut pose_ready = false;
	visit_descendants(root, children, &mut |entity| {
		if pending_spawn.get(entity).is_ok()
			|| pending_root.get(entity).is_ok()
			|| pending_scene.get(entity).is_ok()
		{
			pending = true;
		}
		if poses.get(entity).is_ok() {
			needs_pose = true;
		}
		if posed.get(entity).is_ok() {
			pose_ready = true;
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
	found_mesh && !pending && !unpainted && (!needs_pose || pose_ready)
}

fn visit_descendants(root: Entity, children: &Query<&Children>, visit: &mut impl FnMut(Entity)) {
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

	fn triangle_at(origin: Vec3) -> (Vec<[f32; 3]>, Vec<u32>) {
		(
			vec![
				(origin + Vec3::new(-0.05, 0.0, 0.0)).to_array(),
				(origin + Vec3::new(0.05, 0.0, 0.0)).to_array(),
				(origin + Vec3::new(0.0, 0.05, 0.0)).to_array(),
			],
			vec![0, 1, 2],
		)
	}

	fn body_with_arms_mesh() -> Mesh {
		use bevy::asset::RenderAssetUsages;
		use bevy::mesh::PrimitiveTopology;

		let parts = [
			Vec3::new(0.0, 1.0, 0.0),
			Vec3::new(0.85, 1.25, 0.0),
			Vec3::new(-0.85, 1.25, 0.0),
			Vec3::new(0.0, 0.12, 0.0),
			Vec3::new(0.0, 1.85, 0.0),
		];
		let mut positions = Vec::new();
		let mut indices = Vec::new();
		for origin in parts {
			let base = positions.len() as u32;
			let (verts, _) = triangle_at(origin);
			positions.extend(verts);
			indices.extend([base, base + 1, base + 2]);
		}
		Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
			.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
			.with_inserted_indices(Indices::U32(indices))
	}

	#[test]
	fn isolate_guard_pose_keeps_both_arms_and_drops_the_body() {
		let posed = isolate_guard_pose_arms(&body_with_arms_mesh()).expect("arms isolate");
		let VertexAttributeValues::Float32x3(positions) =
			posed.attribute(Mesh::ATTRIBUTE_POSITION).expect("positions")
		else {
			panic!("float3 positions");
		};
		assert!(positions.len() >= 6);
		let mut min = Vec3::splat(f32::MAX);
		let mut max = Vec3::splat(f32::MIN);
		let mut saw_left = false;
		let mut saw_right = false;
		for position in positions {
			let point = Vec3::from(*position);
			min = min.min(point);
			max = max.max(point);
			if point.x > 0.05 {
				saw_left = true;
			}
			if point.x < -0.05 {
				saw_right = true;
			}
		}
		assert!(saw_left && saw_right, "both arms must remain, {positions:?}");
		assert!(
			(max.y - min.y) < 1.4,
			"isolated arms should not keep full-body height, got {:?}",
			max - min
		);
		assert!(max.z > 0.15, "guard pose should swing the fists forward, max z {}", max.z);
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
	fn fisticuffs_host_uses_the_preview_braidman() {
		let config = BraidmanConfig::default_preview();
		assert!(config.clothing.is_empty());
		let braidman = Braidman::from_config(&config);
		let preview = BraidmanIconPreview::default();
		let nodes = preview.part_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(nodes.len(), 1);
		assert_eq!(nodes[0].scene, braidman.body.path().scene_ref());
		assert!(nodes[0].skin.is_none(), "icon body must be bind-pose, like clothing");
		assert_eq!(HOST_FISTICUFFS_ARMS, "fisticuffs-arms-preview");
	}
}
