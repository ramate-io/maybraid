//! Procedural field meshes and age-driven field parts.

use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;

use crate::drive::{FieldEnvelope, VfxDrive};
use crate::field_material::{FieldKind, FieldMaterial};
use crate::membership::VfxMemberOf;
use crate::spawn::{VfxCancel, VfxInstance, VfxLayerLife, VfxPendingLayers};

/// One animated field mesh. Material is unique per instance.
#[derive(Component, Debug)]
pub struct VfxField {
	pub kind: FieldKind,
	pub playback: f32,
}

/// Expanding shock ring. Local XZ scale follows `speed * (age - delay)`.
#[derive(Component, Debug)]
pub struct VfxWavefront {
	pub speed: f32,
	pub range: f32,
	pub delay: f32,
}

/// Temporary holder so [`stamp_field_materials`] can add the asset after spawn.
#[derive(Component)]
pub struct FieldMaterialSlot(pub FieldMaterial);

/// Sustain light that follows the same envelope as the owning instance.
#[derive(Component, Debug)]
pub struct VfxSustainLight {
	pub peak: f32,
	pub playback: f32,
}

/// Unit cylinder: radius 0.5 (diameter 1), height 1, base at y = 0.
pub fn unit_column_mesh() -> Mesh {
	let mut mesh = Mesh::from(Cylinder { radius: 0.5, half_height: 0.5 });
	if let Some(VertexAttributeValues::Float32x3(positions)) =
		mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
	{
		for position in positions.iter_mut() {
			position[1] += 0.5;
		}
	}
	stamp_object_colors(&mut mesh);
	mesh
}

/// Unit disk of diameter 1 in XZ, slight Y lift so it sits on the ground.
pub fn unit_disk_mesh() -> Mesh {
	let mut mesh = Mesh::from(Cylinder { radius: 0.5, half_height: 0.02 });
	if let Some(VertexAttributeValues::Float32x3(positions)) =
		mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
	{
		for position in positions.iter_mut() {
			position[1] += 0.02;
		}
	}
	stamp_object_colors(&mut mesh);
	mesh
}

/// Thin annulus, outer radius 0.5, covering `spread` radians about +Z.
pub fn unit_ring_mesh(spread: f32) -> Mesh {
	let spread = spread.clamp(0.2, std::f32::consts::TAU);
	let segments = ((spread / std::f32::consts::TAU) * 48.0).ceil().max(8.0) as u32;
	let inner = 0.46;
	let outer = 0.5;
	let half = spread * 0.5;
	let mut positions = Vec::new();
	let mut normals = Vec::new();
	let mut uvs = Vec::new();
	let mut indices = Vec::new();
	for i in 0..=segments {
		let t = i as f32 / segments as f32;
		let angle = -half + spread * t;
		let dir = Vec3::new(angle.sin(), 0.0, angle.cos());
		let inner_p = dir * inner;
		let outer_p = dir * outer;
		positions.push(inner_p.to_array());
		positions.push(outer_p.to_array());
		normals.push([0.0, 1.0, 0.0]);
		normals.push([0.0, 1.0, 0.0]);
		uvs.push([t, 0.0]);
		uvs.push([t, 1.0]);
		if i > 0 {
			let a = (i - 1) * 2;
			let b = a + 1;
			let c = i * 2;
			let d = c + 1;
			indices.extend([a, c, b, b, c, d]);
		}
	}
	let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
	mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
	mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
	mesh.insert_indices(Indices::U32(indices));
	stamp_object_colors(&mut mesh);
	mesh
}

fn stamp_object_colors(mesh: &mut Mesh) {
	let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| a.as_float3())
	else {
		return;
	};
	let colors: Vec<[f32; 4]> = positions.iter().map(|p| [p[0], p[1], p[2], 1.0]).collect();
	mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
}

pub fn stamp_field_materials(
	mut commands: Commands,
	mut materials: ResMut<Assets<FieldMaterial>>,
	pending: Query<(Entity, &FieldMaterialSlot)>,
) {
	for (entity, slot) in &pending {
		let handle = materials.add(slot.0.clone());
		commands
			.entity(entity)
			.insert(MeshMaterial3d(handle))
			.remove::<FieldMaterialSlot>();
	}
}

type FieldTickQuery<'w, 's> = Query<
	'w,
	's,
	(
		&'static VfxMemberOf,
		&'static VfxField,
		Option<&'static VfxWavefront>,
		&'static mut Transform,
		&'static mut Visibility,
		Option<&'static MeshMaterial3d<FieldMaterial>>,
		Option<&'static mut VfxLayerLife>,
	),
>;

pub fn tick_vfx_fields(
	instances: Query<(&VfxInstance, &VfxPendingLayers, Option<&VfxCancel>)>,
	mut materials: ResMut<Assets<FieldMaterial>>,
	mut fields: FieldTickQuery,
) {
	for (member, field, wave, mut transform, mut visibility, material, life) in &mut fields {
		let Ok((instance, pending, cancel)) = instances.get(member.root()) else {
			continue;
		};
		if !instance.armed {
			*visibility = Visibility::Hidden;
			continue;
		}
		*visibility = Visibility::Inherited;
		if let Some(mut life) = life {
			life.age = instance.age;
		}
		let drive = pending.spawn.drive;
		let envelope = FieldEnvelope::sample_canceled(
			instance.age,
			&drive,
			cancel.map(|cancel| cancel.started_age),
		);
		if let Some(wave) = wave {
			let travel_age = (instance.age - wave.delay).max(0.0);
			let radius = (wave.speed * travel_age).clamp(0.15, wave.range);
			transform.scale.x = radius * 2.0;
			transform.scale.z = radius * 2.0;
			transform.scale.y = 1.0;
		}
		if let Some(handle) = material {
			if let Some(mut material) = materials.get_mut(&handle.0) {
				material.set_age(instance.age);
				let mut env = envelope;
				if matches!(field.kind, FieldKind::ShockRing) {
					if let Some(wave) = wave {
						let started = instance.age + 1e-4 >= wave.delay;
						if !started {
							env.alpha = 0.0;
						}
					}
				}
				material.set_envelope(env.alpha, env.reveal, env.contact);
				if matches!(field.kind, FieldKind::ShockRing) {
					material.set_spread(drive.reach);
				}
			}
		}
	}
}

pub fn tick_vfx_sustain_lights(
	instances: Query<(&VfxInstance, &VfxPendingLayers, Option<&VfxCancel>)>,
	mut lights: Query<(&VfxMemberOf, &VfxSustainLight, &mut PointLight, Option<&mut VfxLayerLife>)>,
) {
	for (member, light, mut point, life) in &mut lights {
		let Ok((instance, pending, cancel)) = instances.get(member.root()) else {
			continue;
		};
		if !instance.armed {
			point.intensity = 0.0;
			continue;
		}
		if let Some(mut life) = life {
			life.age = instance.age;
		}
		let envelope = FieldEnvelope::sample_canceled(
			instance.age,
			&pending.spawn.drive,
			cancel.map(|cancel| cancel.started_age),
		);
		point.intensity = light.peak * envelope.alpha * light.playback.clamp(0.25, 1.35);
	}
}

pub fn column_pose(
	spawn_translation: Vec3,
	spawn_rotation: Quat,
	drive: &VfxDrive,
) -> (Transform, f32) {
	let sky = drive.far_point.unwrap_or(spawn_translation + Vec3::Y * drive.reach);
	let delta = sky - spawn_translation;
	let height = delta.length().max(1.0);
	let dir = if delta.length_squared() > 1e-6 { delta.normalize() } else { Vec3::Y };
	(
		Transform {
			translation: spawn_translation,
			rotation: spawn_rotation * Quat::from_rotation_arc(Vec3::Y, dir),
			scale: Vec3::ONE,
		},
		height,
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn column_mesh_sits_on_the_base() -> Result<(), String> {
		let mesh = unit_column_mesh();
		let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| a.as_float3())
		else {
			return Err("missing positions".into());
		};
		let min_y = positions.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
		let max_y = positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
		if min_y < -0.02 {
			return Err(format!("base {min_y}"));
		}
		if (max_y - 1.0).abs() > 0.05 {
			return Err(format!("height {max_y}"));
		}
		Ok(())
	}

	#[test]
	fn ring_mesh_covers_spread() -> Result<(), String> {
		let mesh = unit_ring_mesh(std::f32::consts::PI);
		let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| a.as_float3())
		else {
			return Err("missing positions".into());
		};
		if positions.len() < 16 {
			return Err(format!("verts {}", positions.len()));
		}
		Ok(())
	}
}
