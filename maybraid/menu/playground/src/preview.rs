//! Live character preview driven by [`CharacterMenuState`].

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::scene::prelude::bsn;
use character_ui_menu::{CameraFocus, FocusRig};
use crozon_character_playground::CameraController;
use crozon_character_items::InventoryItem;
use crozon_character_ui_menus::{
	CharacterField, CharacterMenu, ConceptSpecies, MenuEvent, BODY_FOCUS,
};
use crozon_characters::{
	character_bounds, AnimRef, AnimRefRoot, BoneMap, CharacterComponents, CharacterHostSystems,
	CharacterMembers, CharacterRecipe, CharacterRig, CharacterRigRole, ComponentsOnly,
};
use crozon_characters::species::braidman::BraidmanConfig;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use maybraid_character_ui_menu_renderer::CharacterMenuEvent;
use menu_screens::{SpinRevealCurrent, SpinRevealScreen, SpinRevealSystems};

use crate::character::{CharacterMenuState, CharacterScreen};

#[derive(Component)]
struct CharacterPreviewRoot;

#[derive(Resource, Default)]
struct PreviewSyncState {
	key: String,
}

#[derive(Resource, Default)]
struct PendingCameraFocus {
	focus: Option<CameraFocus>,
	resolved: Option<Transform>,
}

pub struct CharacterPreviewPlugin;

impl Plugin for CharacterPreviewPlugin {
	fn build(&self, app: &mut App) {
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
		DirectionalLight { illuminance: 10000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 500.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}

fn sync_preview(
	mut commands: Commands,
	screens: Query<Entity, With<CharacterScreen>>,
	spin_screens: Query<Entity, With<SpinRevealScreen>>,
	menu_state: Res<CharacterMenuState>,
	spin: Option<Res<SpinRevealCurrent>>,
	mut sync: ResMut<PreviewSyncState>,
	mut pending: ResMut<PendingCameraFocus>,
	roots: Query<Entity, With<CharacterPreviewRoot>>,
) {
	if !screens.is_empty() {
		let key = format!("{:?}", menu_state.0);
		if sync.key == key && !roots.is_empty() {
			return;
		}
		sync.key = key;
		for entity in &roots {
			commands.entity(entity).despawn();
		}
		spawn_from_menu(&mut commands, &menu_state.0);
		pending.resolved = None;
		if pending.focus.is_none() {
			pending.focus = Some(default_body_focus(&menu_state.0));
		}
		return;
	}

	if !spin_screens.is_empty() {
		if let Some(spin) = spin {
			let key = format!("spin:{:?}", spin.item);
			if sync.key == key && !roots.is_empty() {
				return;
			}
			sync.key = key;
			for entity in &roots {
				commands.entity(entity).despawn();
			}
			spawn_from_item(&mut commands, &spin.item);
			pending.resolved = None;
			pending.focus = Some(BODY_FOCUS);
			return;
		}
	}

	for entity in &roots {
		commands.entity(entity).despawn();
	}
	sync.key.clear();
	pending.focus = None;
	pending.resolved = None;
}

fn spawn_from_item(commands: &mut Commands, item: &InventoryItem) {
	let Some(mesh) = item.mesh() else {
		return;
	};
	let material = item.material();
	let mut config = BraidmanConfig::default_preview();
	config.clothing = vec![mesh];
	config.colors.set_clothing_color(mesh, material.color);
	config.colors.set_clothing_material(mesh, material.id);
	spawn_clothed(commands, &config.clothed());
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
	menu_state: Res<CharacterMenuState>,
	roots: Query<&CharacterMembers, With<CharacterPreviewRoot>>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	let desired = AnimRef::from(menu_state.0.animation());
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
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
	roots: Query<&CharacterMembers, With<CharacterPreviewRoot>>,
	rigs: Query<(Entity, &CharacterRig, &BoneMap, &GlobalTransform)>,
	transforms: Query<&GlobalTransform>,
) {
	let Some(focus) = pending.focus else {
		return;
	};
	let Ok((mut transform, mut controller)) = cameras.single_mut() else {
		return;
	};
	let target = if let Some(resolved) = pending.resolved {
		resolved
	} else if let Some(resolved) = resolve_focus_transform(focus, &roots, &rigs, &transforms) {
		pending.resolved = Some(resolved);
		resolved
	} else {
		Transform::from_translation(focus.camera_offset).looking_at(focus.look_at_offset, Vec3::Y)
	};
	*transform = target;
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
) -> Option<Transform> {
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
			return Some(Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y));
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
