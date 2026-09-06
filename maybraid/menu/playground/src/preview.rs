//! Live character preview driven by [`CharacterMenuState`].

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::scene::prelude::bsn;
use bevy::window::PrimaryWindow;
use character_ui_menu::{CameraFocus, FocusRig};
use crozon_character_items::{
	ClothingHost, ClothingMesh, FirearmBarrel, FirearmGrip, FirearmSpec, FirearmTriggerBox,
	InventoryItem, ItemColor, SlotLook,
};
use crozon_character_persist::SaveRoot;
use crozon_character_playground::CameraController;
use crozon_character_ui_menus::{
	spin_reveal_firearm_focus, spin_reveal_focus, CharacterField, CharacterMenu, ConceptSpecies,
	MenuEvent, BODY_FOCUS,
};
use crozon_characters::{
	add_character_components_host, character_bounds, AnimRef, AnimRefRoot, BoneMap,
	CharacterComponents, CharacterHostSystems, CharacterMembers, CharacterRecipe, CharacterRig,
	CharacterRigRole, ClothingLayer, ComponentsOnly, Layers, MaterialRef, PartNode,
};
use firearms_components::assets::guns;
use firearms_components::{
	add_firearm_components_host, firearm_bounds, spawn_firearm_components, ActiveRigPose,
	BoneScale, FirearmComponents, FirearmComponentsPlugin, FirearmHostSystems, FirearmMembers,
	FirearmRoot, Layers as FirearmLayers, PartNode as FirearmPartNode, ResolvedRigPose, RigNode,
	RigPoseLayer, RigRoot,
};
use lod::gen::LodScene;
use lod::gen::LodSceneLevel;
use lod::lod_ref::LodRef;
use maybraid_character_ui_menu_renderer::CharacterMenuEvent;
use menu_components::DESCRIPTION_PANE_LEFT_PERCENT;
use menu_screens::{
	GalleryScreen, HomeScreen, SpinRevealCurrent, SpinRevealScreen, SpinRevealSystems,
};

use crate::character::{CharacterMenuState, CharacterScreen};
use crate::session::ActiveCharacter;
use crate::weapon_gallery::{RequestShowWeapons, WeaponGalleryScreen};

#[derive(Component)]
pub struct CharacterPreviewRoot;

/// Menu-playground key / fill lights. Composed applications can put these on
/// the same render layer as [`CharacterPreviewRoot`].
#[derive(Component)]
pub struct CharacterPreviewLight;

#[derive(Resource, Default)]
struct PreviewSyncState {
	key: String,
	anim: Option<AnimRef>,
}

#[derive(Resource, Default)]
struct PendingCameraFocus {
	focus: Option<CameraFocus>,
	resolved: Option<Transform>,
	look_at: Option<Vec3>,
}

pub struct CharacterPreviewPlugin;

impl Plugin for CharacterPreviewPlugin {
	fn build(&self, app: &mut App) {
		add_character_components_host::<ClothingPreview>(app);
		if !app.is_plugin_added::<FirearmComponentsPlugin>() {
			app.add_plugins(FirearmComponentsPlugin);
		}
		add_firearm_components_host::<FirearmPreview>(app);
		app.init_resource::<PreviewSyncState>()
			.init_resource::<PendingCameraFocus>()
			.insert_resource(GlobalAmbientLight {
				color: Color::WHITE,
				brightness: 200.0,
				..default()
			})
			.add_systems(Startup, setup_lighting)
			.add_systems(
				Update,
				(
					sync_preview.after(SpinRevealSystems::Apply),
					stamp_preview_animation
						.after(sync_preview)
						.after(CharacterHostSystems::Membership)
						.before(crozon_characters::CharacterMotionSystems::Anim),
					apply_firearm_preview_pose.after(FirearmHostSystems::Membership),
					queue_preview_camera_focus,
				),
			)
			.add_systems(
				PostUpdate,
				apply_preview_camera_focus
					.after(TransformSystems::Propagate)
					.after(CharacterHostSystems::Pose),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	commands.spawn((
		CharacterPreviewLight,
		DirectionalLight { illuminance: 10000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		CharacterPreviewLight,
		DirectionalLight { illuminance: 500.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}

fn sync_preview(
	mut commands: Commands,
	screens: Query<Entity, With<CharacterScreen>>,
	spin_screens: Query<Entity, With<SpinRevealScreen>>,
	home: Query<Entity, With<HomeScreen>>,
	gallery: Query<Entity, With<GalleryScreen>>,
	weapons: Query<Entity, With<WeaponGalleryScreen>>,
	weapon_requests: Query<Entity, With<RequestShowWeapons>>,
	menu_state: Res<CharacterMenuState>,
	spin: Option<Res<SpinRevealCurrent>>,
	active: Option<Res<ActiveCharacter>>,
	save_root: Option<Res<SaveRoot>>,
	mut sync: ResMut<PreviewSyncState>,
	mut pending: ResMut<PendingCameraFocus>,
	roots: Query<Entity, With<CharacterPreviewRoot>>,
) {
	if !weapons.is_empty() || !weapon_requests.is_empty() {
		clear_preview(&mut commands, &mut sync, &mut pending, &roots);
		return;
	}

	if !screens.is_empty() {
		respawn_from_menu(
			&mut commands,
			&mut sync,
			&mut pending,
			&roots,
			&menu_state.0,
			format!("{:?}", menu_state.0),
			false,
		);
		return;
	}

	if !spin_screens.is_empty() {
		if let Some(spin) = spin {
			let key = format!("spin:{:?}", spin.item);
			if sync.key == key && !roots.is_empty() {
				return;
			}
			sync.key = key;
			sync.anim = None;
			for entity in &roots {
				commands.entity(entity).despawn();
			}
			spawn_from_item(&mut commands, &spin.item);
			pending.resolved = None;
			pending.focus = match spin.item.firearm_mesh() {
				Some(_) => Some(spin_reveal_firearm_focus()),
				None => {
					spin.item.mesh().map(|mesh| spin_reveal_focus(mesh.kind())).or(Some(BODY_FOCUS))
				}
			};
			return;
		}
	}

	if home.is_empty() && gallery.is_empty() {
		clear_preview(&mut commands, &mut sync, &mut pending, &roots);
		return;
	}

	let Some(active) = active else {
		clear_preview(&mut commands, &mut sync, &mut pending, &roots);
		return;
	};
	let Some(save_root) = save_root else {
		return;
	};
	let Some(menu) = menu_for_saved(&save_root, active.id) else {
		clear_preview(&mut commands, &mut sync, &mut pending, &roots);
		return;
	};
	respawn_from_menu(
		&mut commands,
		&mut sync,
		&mut pending,
		&roots,
		&menu,
		format!("active:{}", active.id.to_hex()),
		true,
	);
}

fn respawn_from_menu(
	commands: &mut Commands,
	sync: &mut PreviewSyncState,
	pending: &mut PendingCameraFocus,
	roots: &Query<Entity, With<CharacterPreviewRoot>>,
	menu: &CharacterMenu,
	key: String,
	force_focus: bool,
) {
	if sync.key == key && !roots.is_empty() {
		return;
	}
	sync.key = key;
	sync.anim = Some(AnimRef::from(menu.animation()));
	for entity in roots {
		commands.entity(entity).despawn();
	}
	spawn_from_menu(commands, menu);
	pending.resolved = None;
	if force_focus || pending.focus.is_none() {
		pending.focus = Some(default_body_focus(menu));
	}
}

fn clear_preview(
	commands: &mut Commands,
	sync: &mut PreviewSyncState,
	pending: &mut PendingCameraFocus,
	roots: &Query<Entity, With<CharacterPreviewRoot>>,
) {
	for entity in roots {
		commands.entity(entity).despawn();
	}
	sync.key.clear();
	sync.anim = None;
	pending.focus = None;
	pending.resolved = None;
	pending.look_at = None;
}

fn menu_for_saved(
	root: &SaveRoot,
	id: crozon_character_persist::CharacterId,
) -> Option<CharacterMenu> {
	let model = crozon_character_model_user::load(root, id).ok()?;
	let inventory = crozon_inventory_user::load(root, id).ok()?;
	Some(CharacterMenu::for_saved(model.name, &model.appearance, inventory))
}

pub(crate) fn spawn_firearm(
	commands: &mut Commands,
	spec: FirearmSpec,
	transform: Transform,
) -> Vec<Entity> {
	let preview = FirearmPreview { spec };
	spawn_firearm_components(commands, &preview, transform, firearm_bounds(&preview))
}

fn spawn_from_item(commands: &mut Commands, item: &InventoryItem) {
	if let Some(spec) = item.firearm_spec() {
		for entity in spawn_firearm(commands, spec, Transform::IDENTITY) {
			commands.entity(entity).insert(CharacterPreviewRoot);
		}
		return;
	}
	let Some(mesh) = item.mesh() else {
		return;
	};
	let Some(material) = item.material() else {
		return;
	};
	spawn_clothed(
		commands,
		&ClothingPreview {
			layer: ClothingLayer::new(mesh, material.color, ClothingHost::HUMANOID)
				.with_material(material.id),
		},
	);
}

/// Assembled catalog kit from inventory identity.
#[derive(Clone, Default, PartialEq)]
struct FirearmPreview {
	spec: FirearmSpec,
}

impl FirearmPreview {
	fn look_material(look: SlotLook) -> MaterialRef {
		MaterialRef::named(look.material.recipe_id()).with_palette([look.color.color()])
	}

	fn pose(&self) -> ResolvedRigPose {
		let mut layer = RigPoseLayer::new("kit");
		for (name, length, thickness) in self.spec.scales.bone_fits() {
			layer = layer
				.with_scale(BoneScale::length(name, length))
				.with_scale(BoneScale::thickness(name, thickness));
		}
		ResolvedRigPose::new().with_layer(layer)
	}
}

impl FirearmComponents for FirearmPreview {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> FirearmLayers<RigNode> {
		FirearmLayers::from_labeled(
			"receiver",
			vec![RigNode::receiver("firearm-rig", guns::FIREARM_RIG.as_str())],
		)
	}

	fn body_nodes_for_level(&self, _level: LodSceneLevel) -> FirearmLayers<FirearmPartNode> {
		let material = Self::look_material(self.spec.looks.body);
		let body = self.spec.kit.body;
		FirearmLayers::from_labeled(
			"body",
			vec![FirearmPartNode::body(body.label(), body.body_path()).with_material(material)],
		)
	}

	fn barrel_nodes_for_level(&self, _level: LodSceneLevel) -> FirearmLayers<FirearmPartNode> {
		let material = Self::look_material(self.spec.looks.barrel);
		match self.spec.kit.barrel {
			FirearmBarrel::None => FirearmLayers::new(),
			FirearmBarrel::Bullpup => FirearmLayers::from_labeled(
				"barrel",
				vec![FirearmPartNode::barrel("bullpup", guns::BULLPUP_BARREL.as_str())
					.with_material(material)],
			),
			FirearmBarrel::Laznard => FirearmLayers::from_labeled(
				"barrel",
				vec![FirearmPartNode::barrel("laznard", guns::LAZNARD_BARREL.as_str())
					.with_material(material)],
			),
		}
	}

	fn trigger_box_nodes_for_level(&self, _level: LodSceneLevel) -> FirearmLayers<FirearmPartNode> {
		let material = Self::look_material(self.spec.looks.trigger_box);
		match self.spec.kit.trigger_box {
			FirearmTriggerBox::None => FirearmLayers::new(),
			FirearmTriggerBox::Keelripe => FirearmLayers::from_labeled(
				"trigger_box",
				vec![FirearmPartNode::trigger_box("keelripe", guns::KEELRIPE_BOX.as_str())
					.with_material(material)],
			),
			FirearmTriggerBox::Paddle => FirearmLayers::from_labeled(
				"trigger_box",
				vec![FirearmPartNode::trigger_box("paddle", guns::PADDLE_BOX.as_str())
					.with_material(material)],
			),
			FirearmTriggerBox::Reltor => FirearmLayers::from_labeled(
				"trigger_box",
				vec![FirearmPartNode::trigger_box("reltor", guns::RELTOR_BOX.as_str())
					.with_material(material)],
			),
		}
	}

	fn grip_nodes_for_level(&self, _level: LodSceneLevel) -> FirearmLayers<FirearmPartNode> {
		match self.spec.kit.grip {
			FirearmGrip::None => FirearmLayers::new(),
			FirearmGrip::BumpHandle => FirearmLayers::from_labeled(
				"grip",
				vec![FirearmPartNode::grip("bump-handle", guns::BUMP_HANDLE.as_str())
					.with_material(Self::look_material(self.spec.looks.grip))],
			),
		}
	}
}

fn apply_firearm_preview_pose(
	hosts: Query<
		(&firearms_components::ComponentsOnly<FirearmPreview>, Option<&FirearmMembers>),
		With<FirearmRoot>,
	>,
	mut poses: Query<&mut ActiveRigPose, With<RigRoot>>,
) {
	for (preview, members) in &hosts {
		let Some(members) = members else {
			continue;
		};
		let resolved = preview.pose();
		for member in members.iter() {
			if let Ok(mut active) = poses.get_mut(member) {
				active.pose = resolved.clone();
			}
		}
	}
}

/// Unskinned garment in bind pose. Camera framing is per clothing kind.
#[derive(Clone, PartialEq)]
struct ClothingPreview {
	layer: ClothingLayer,
}

impl Default for ClothingPreview {
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

impl CharacterComponents for ClothingPreview {
	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("clothing", vec![self.layer.preview_part_node()])
	}
}

fn spawn_from_menu(commands: &mut Commands, menu: &CharacterMenu) {
	match menu.species.value {
		ConceptSpecies::Braidman => spawn_clothed(commands, &menu.braidman_config().clothed()),
		ConceptSpecies::Brenal => spawn_clothed(commands, &menu.brenal_config().clothed()),
		ConceptSpecies::Caole => spawn_clothed(commands, &menu.caole_config().clothed()),
		ConceptSpecies::Epiphant => spawn_clothed(commands, &menu.epiphant_config().clothed()),
		ConceptSpecies::Hars => spawn_clothed(commands, &menu.hars_config().clothed()),
		ConceptSpecies::Yilter => spawn_clothed(commands, &menu.ylter_config().clothed()),
		ConceptSpecies::Sonyak => spawn_clothed(commands, &menu.sonyak_config().clothed()),
		ConceptSpecies::Claber => spawn_clothed(commands, &menu.claber_config().clothed()),
		ConceptSpecies::Croconot => spawn_clothed(commands, &menu.croconot_config().clothed()),
		ConceptSpecies::Brodler => spawn_clothed(commands, &menu.brodler_config().clothed()),
		ConceptSpecies::Mygr => spawn_clothed(commands, &menu.mygr_config().clothed()),
		ConceptSpecies::Dui => spawn_clothed(commands, &menu.dui_config().clothed()),
		ConceptSpecies::Lidder => spawn_clothed(commands, &menu.lidder_config().clothed()),
		ConceptSpecies::Chupri => spawn_clothed(commands, &menu.chupri_config().clothed()),
		ConceptSpecies::Brokker => spawn_clothed(commands, &menu.brokker_config().clothed()),
		ConceptSpecies::Tipple => spawn_clothed(commands, &menu.tipple_config().clothed()),
		ConceptSpecies::Topple => spawn_clothed(commands, &menu.topple_config().clothed()),
		ConceptSpecies::Kispar => spawn_clothed(commands, &menu.kispar_config().clothed()),
		ConceptSpecies::Tapp => spawn_clothed(commands, &menu.tapp_config().clothed()),
		ConceptSpecies::Kaller => spawn_clothed(commands, &menu.kaller_config().clothed()),
		ConceptSpecies::Kappler => spawn_clothed(commands, &menu.kappler_config().clothed()),
		ConceptSpecies::Wumbus => spawn_clothed(commands, &menu.wumbus_config().clothed()),
		ConceptSpecies::Lero => spawn_clothed(commands, &menu.lero_config().clothed()),
		ConceptSpecies::Spibmom => spawn_clothed(commands, &menu.spibmom_config().clothed()),
		ConceptSpecies::Grener => spawn_clothed(commands, &menu.grener_config().clothed()),
		ConceptSpecies::Thumplus => spawn_clothed(commands, &menu.thumplus_config().clothed()),
		ConceptSpecies::Mistler => spawn_clothed(commands, &menu.mistler_config().clothed()),
		ConceptSpecies::Tuberwaber => spawn_clothed(commands, &menu.tuberwaber_config().clothed()),
	}
}

fn spawn_clothed<T>(commands: &mut Commands, character: &T)
where
	T: CharacterComponents + Clone + Default + Unpin + Send + Sync + 'static,
{
	let bounds = character_bounds(character);
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let host = ComponentsOnly(character.clone());
	let entity = commands
		.spawn_scene((
			host.host(&lod_ref),
			bsn! {
				Transform::IDENTITY
			},
		))
		.id();
	commands.entity(entity).insert(CharacterPreviewRoot);
}

fn stamp_preview_animation(
	mut commands: Commands,
	sync: Res<PreviewSyncState>,
	menu_state: Res<CharacterMenuState>,
	character_screens: Query<Entity, With<CharacterScreen>>,
	spin_screens: Query<Entity, With<SpinRevealScreen>>,
	roots: Query<&CharacterMembers, With<CharacterPreviewRoot>>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	if !spin_screens.is_empty() {
		return;
	}
	let desired = if character_screens.is_empty() {
		let Some(anim) = sync.anim else {
			return;
		};
		anim
	} else {
		AnimRef::from(menu_state.0.animation())
	};
	for members in &roots {
		for member in members.iter() {
			if !rigs.get(member).is_ok_and(|rig| rig.role == CharacterRigRole::Body) {
				continue;
			}
			let needs_clip = match anims.get(member) {
				Ok(root) => root.0 != desired,
				Err(_) => true,
			};
			if needs_clip {
				commands.entity(member).insert(AnimRefRoot(desired));
			}
		}
	}
}

fn queue_preview_camera_focus(
	mut events: MessageReader<CharacterMenuEvent<MenuEvent>>,
	mut pending: ResMut<PendingCameraFocus>,
) {
	for event in events.read() {
		if let CharacterMenuEvent::CameraFocus(focus) = event {
			pending.focus = Some(*focus);
			pending.resolved = None;
		}
	}
}

fn apply_preview_camera_focus(
	mut pending: ResMut<PendingCameraFocus>,
	mut cameras: Query<
		(&mut Transform, &mut CameraController, &mut Camera, &Projection),
		With<Camera3d>,
	>,
	windows: Query<&Window, With<PrimaryWindow>>,
	home: Query<(), With<HomeScreen>>,
	gallery: Query<(), With<GalleryScreen>>,
	weapons: Query<(), With<WeaponGalleryScreen>>,
	roots: Query<&CharacterMembers, With<CharacterPreviewRoot>>,
	rigs: Query<(Entity, &CharacterRig, &BoneMap, &GlobalTransform)>,
	transforms: Query<&GlobalTransform>,
) {
	if !weapons.is_empty() {
		return;
	}
	let Some(focus) = pending.focus else {
		return;
	};
	let Ok((mut transform, mut controller, mut camera, projection)) = cameras.single_mut() else {
		return;
	};
	camera.viewport = None;
	let target = if let Some(resolved) = pending.resolved {
		resolved
	} else if let Some((resolved, look_at)) =
		resolve_focus_transform(focus, &roots, &rigs, &transforms)
	{
		pending.resolved = Some(resolved);
		pending.look_at = Some(look_at);
		resolved
	} else {
		pending.look_at = Some(focus.look_at_offset);
		Transform::from_translation(focus.camera_offset).looking_at(focus.look_at_offset, Vec3::Y)
	};
	*transform = target;
	if !home.is_empty() || !gallery.is_empty() {
		let aspect = windows
			.single()
			.ok()
			.map(|window| window.width() / window.height().max(1.0))
			.unwrap_or(16.0 / 9.0);
		let look_at =
			pending.look_at.unwrap_or_else(|| target.translation + *target.forward() * 4.0);
		offset_camera_into_display_pane(&mut transform, projection, look_at, aspect);
	}
	sync_controller_from_transform(&mut controller, &transform);
}

fn default_body_focus(menu: &CharacterMenu) -> CameraFocus {
	menu.camera_focus_for_event(MenuEvent::Cycle(CharacterField::Animation, 0))
		.unwrap_or(BODY_FOCUS)
}

fn resolve_focus_transform(
	focus: CameraFocus,
	roots: &Query<&CharacterMembers, With<CharacterPreviewRoot>>,
	rigs: &Query<(Entity, &CharacterRig, &BoneMap, &GlobalTransform)>,
	transforms: &Query<&GlobalTransform>,
) -> Option<(Transform, Vec3)> {
	let role = match focus.rig {
		FocusRig::Body => CharacterRigRole::Body,
		FocusRig::Head => CharacterRigRole::Head,
	};
	for members in roots {
		for member in members.iter() {
			let Ok((_, rig, bones, rig_gt)) = rigs.get(member) else {
				continue;
			};
			if rig.role != role {
				continue;
			}
			let socket_gt = if focus.socket == "root" {
				*rig_gt
			} else {
				let bone = *bones.by_name.get(focus.socket)?;
				*transforms.get(bone).ok()?
			};
			let camera_pos = socket_oriented_point(&socket_gt, focus.camera_offset);
			let look_at = socket_oriented_point(&socket_gt, focus.look_at_offset);
			return Some((
				Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y),
				look_at,
			));
		}
	}
	None
}

fn socket_oriented_point(socket: &GlobalTransform, local_offset: Vec3) -> Vec3 {
	socket.translation() + socket.rotation() * local_offset
}

fn sync_controller_from_transform(controller: &mut CameraController, transform: &Transform) {
	let rotation = transform.rotation;
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	controller.yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = 2.0 * (w * x - y * z);
	controller.pitch = sin_pitch.clamp(-1.0, 1.0).asin();
}

/// Shift the framed body into the right-hand display pane so the left menu
/// does not sit on top of it. NDC x = 0 is screen center.
fn offset_camera_into_display_pane(
	transform: &mut Transform,
	projection: &Projection,
	look_at: Vec3,
	aspect: f32,
) {
	let Projection::Perspective(perspective) = projection else {
		return;
	};
	let depth = transform.translation.distance(look_at).max(0.1);
	let half_height = (perspective.fov * 0.5).tan() * depth;
	let half_width = half_height * aspect.max(0.1);
	let pane_center = (DESCRIPTION_PANE_LEFT_PERCENT + 100.0) * 0.005;
	let ndc_x = pane_center * 2.0 - 1.0;
	transform.translation -= *transform.right() * ndc_x * half_width;
	let ndc_y = 0.16;
	transform.translation -= *transform.up() * ndc_y * half_height;
}
