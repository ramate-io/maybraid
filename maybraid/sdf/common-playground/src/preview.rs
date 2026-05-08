use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use render_item::{
	mesh::{handle::MeshHandle, MeshDispatch},
	DispatchRenderItem,
};
use sdf_common::{SdfCommonPrimitive, SdfCommonRenderItem};

use crate::{ground::GroundPlane, PlaygroundMaterial};

#[derive(Component)]
pub struct SdfPreviewRoot;

#[derive(Resource, Clone)]
pub struct PreviewConfig {
	pub primitive: SdfCommonPrimitive,
	pub res_2: u8,
}

pub fn keyboard_preview(mut config: ResMut<PreviewConfig>, keyboard: Res<ButtonInput<KeyCode>>) {
	let mut changed = false;

	if keyboard.just_pressed(KeyCode::Tab) {
		config.primitive = next_primitive(&config.primitive);
		changed = true;
	}
	if keyboard.just_pressed(KeyCode::Digit1) {
		config.primitive = SdfCommonPrimitive::tapered_cylinder_default();
		changed = true;
	}
	if keyboard.just_pressed(KeyCode::Digit2) {
		config.primitive = SdfCommonPrimitive::noisy_cylinder_default();
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

fn next_primitive(current: &SdfCommonPrimitive) -> SdfCommonPrimitive {
	match current {
		SdfCommonPrimitive::TaperedCylinder(_) => SdfCommonPrimitive::noisy_cylinder_default(),
		SdfCommonPrimitive::NoisyCylinder(_) => SdfCommonPrimitive::tapered_cylinder_default(),
	}
}

/// Respawns the marching-cubes preview whenever [`PreviewConfig`] changes (variant or resolution).
pub fn sync_sdf_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	mut synced: Local<Option<(String, u8)>>,
	material: Res<PlaygroundMaterial>,
	root_q: Query<Entity, With<SdfPreviewRoot>>,
	dispatch_q: Query<Entity, With<MeshDispatch<MeshHandle<SdfCommonPrimitive>>>>,
	mesh_q: Query<Entity, (With<Mesh3d>, Without<GroundPlane>)>,
) {
	let key = (config.primitive.variant_key().to_string(), config.res_2);
	if synced.as_ref() == Some(&key) {
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
		DispatchRenderItem::new(SdfCommonRenderItem::new(
			config.primitive.clone(),
			MeshMaterial3d(material.0.clone()),
		)),
		Transform::IDENTITY,
	));
}
