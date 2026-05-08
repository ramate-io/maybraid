use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use render_item::{
	mesh::{handle::MeshHandle, MeshDispatch},
	DispatchRenderItem,
};

use crate::primitive::{PlaygroundPrimitive, PlaygroundRenderItem};

use crate::{ground::GroundPlane, PlaygroundMaterial};

#[derive(Component)]
pub struct SdfPreviewRoot;

#[derive(Resource, Clone)]
pub struct PreviewConfig {
	pub primitive: PlaygroundPrimitive,
	pub res_2: u8,
	pub transform: Transform,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self {
			primitive: PlaygroundPrimitive::tapered_cylinder_default(),
			res_2: 4,
			transform: Transform::default(),
		}
	}
}

fn preview_sync_key(config: &PreviewConfig) -> String {
	let geom = match &config.primitive {
		PlaygroundPrimitive::TaperedCylinder(c) => format!("t:{c:?}"),
		PlaygroundPrimitive::NoisyCylinder(n) => format!("n:{:?}|{:?}", n.inner, n.noise),
		PlaygroundPrimitive::CrookCylinder(c) => format!("k:{c:?}"),
		PlaygroundPrimitive::NoisyCrookCylinder(n) => {
			format!("nk:{:?}|{:?}", n.inner, n.noise)
		}
	};
	format!(
		"{geom}|{}|{:?}|{:?}",
		config.res_2, config.transform.translation, config.transform.scale
	)
}

pub fn keyboard_preview(
	mut config: ResMut<PreviewConfig>,
	keyboard: Res<ButtonInput<KeyCode>>,
	text_focus: Res<crate::input::TextEntryFocus>,
) {
	if text_focus.0 {
		return;
	}

	let mut changed = false;

	if keyboard.just_pressed(KeyCode::Tab) {
		config.primitive = next_primitive(&config.primitive);
		changed = true;
	}
	if keyboard.just_pressed(KeyCode::Digit1) {
		config.primitive = PlaygroundPrimitive::tapered_cylinder_default();
		changed = true;
	}
	if keyboard.just_pressed(KeyCode::Digit2) {
		config.primitive = PlaygroundPrimitive::noisy_cylinder_default();
		changed = true;
	}
	if keyboard.just_pressed(KeyCode::Digit3) {
		config.primitive = PlaygroundPrimitive::crook_cylinder_default();
		changed = true;
	}
	if keyboard.just_pressed(KeyCode::Digit4) {
		config.primitive = PlaygroundPrimitive::noisy_crook_cylinder_default();
		changed = true;
	}
	if keyboard.just_pressed(KeyCode::Equal) {
		config.res_2 = (config.res_2 + 1).min(8);
		changed = true;
	}
	if keyboard.just_pressed(KeyCode::Minus) {
		config.res_2 = config.res_2.saturating_sub(1).max(2);
		changed = true;
	}

	if changed {
		log::debug!(
			"sdf preview: {:?} res_2={}",
			config.primitive.variant_key(),
			config.res_2
		);
	}
}

fn next_primitive(current: &PlaygroundPrimitive) -> PlaygroundPrimitive {
	match current {
		PlaygroundPrimitive::TaperedCylinder(_) => PlaygroundPrimitive::noisy_cylinder_default(),
		PlaygroundPrimitive::NoisyCylinder(_) => PlaygroundPrimitive::crook_cylinder_default(),
		PlaygroundPrimitive::CrookCylinder(_) => PlaygroundPrimitive::noisy_crook_cylinder_default(),
		PlaygroundPrimitive::NoisyCrookCylinder(_) => PlaygroundPrimitive::tapered_cylinder_default(),
	}
}

/// Respawns the marching-cubes preview whenever [`PreviewConfig`] changes.
pub fn sync_sdf_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	mut synced: Local<Option<String>>,
	material: Res<PlaygroundMaterial>,
	root_q: Query<Entity, With<SdfPreviewRoot>>,
	dispatch_q: Query<Entity, With<MeshDispatch<MeshHandle<PlaygroundPrimitive>>>>,
	mesh_q: Query<Entity, (With<Mesh3d>, Without<GroundPlane>)>,
) {
	let key = preview_sync_key(&config);
	if synced.as_deref() == Some(&key) {
		return;
	}

	for e in dispatch_q.iter() {
		commands.entity(e).despawn();
	}
	for e in mesh_q.iter() {
		commands.entity(e).despawn();
	}
	for e in root_q.iter() {
		commands.entity(e).despawn();
	}

	*synced = Some(key);

	commands.spawn((
		SdfPreviewRoot,
		CascadeChunk::unit_center_chunk().with_res_2(config.res_2),
		DispatchRenderItem::new(PlaygroundRenderItem::new(
			config.primitive.clone(),
			MeshMaterial3d(material.0.clone()),
		)),
		config.transform,
	));
}
