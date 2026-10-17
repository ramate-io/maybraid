//! Procedural field meshes and age-driven field parts.

use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;

use crate::drive::{FieldEnvelope, VfxDrive, VfxWaveStates};
use crate::field_material::{FieldKind, FieldMaterial};
use crate::membership::VfxMemberOf;
use crate::spawn::{VfxCancel, VfxInstance, VfxLayerLife, VfxPendingLayers};

/// One animated field mesh. Material is unique per instance.
#[derive(Component, Debug)]
pub struct VfxField {
	pub kind: FieldKind,
	pub playback: f32,
}

/// One vortex ring that only renders a [`crate::drive::SonicWaveState`] snapshot.
#[derive(Component, Debug, Clone, Copy)]
pub struct VfxSonicRing {
	pub index: u32,
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

/// Hollow vortex ring in XY, facing +Z. Outer diameter 1, depth 1.
/// `scale.xy = 2 * radius` and `scale.z = thickness`.
pub fn unit_vortex_ring_mesh() -> Mesh {
	let segments = 40u32;
	let inner = 0.36;
	let outer = 0.5;
	let mut positions = Vec::new();
	let mut normals = Vec::new();
	let mut uvs = Vec::new();
	let mut indices = Vec::new();
	let mut push = |pos: Vec3, normal: Vec3, uv: [f32; 2]| {
		positions.push(pos.to_array());
		normals.push(normal.to_array());
		uvs.push(uv);
	};
	for i in 0..=segments {
		let t = i as f32 / segments as f32;
		let angle = t * std::f32::consts::TAU;
		let dir = Vec3::new(angle.cos(), angle.sin(), 0.0);
		let inner_pos = dir * inner;
		let outer_pos = dir * outer;
		// Front (+Z), back (−Z), outer wall, inner wall.
		push(outer_pos + Vec3::Z * 0.5, Vec3::Z, [t, 1.0]);
		push(inner_pos + Vec3::Z * 0.5, Vec3::Z, [t, 0.0]);
		push(outer_pos - Vec3::Z * 0.5, Vec3::NEG_Z, [t, 1.0]);
		push(inner_pos - Vec3::Z * 0.5, Vec3::NEG_Z, [t, 0.0]);
		push(outer_pos + Vec3::Z * 0.5, dir, [t, 1.0]);
		push(outer_pos - Vec3::Z * 0.5, dir, [t, 0.0]);
		push(inner_pos + Vec3::Z * 0.5, -dir, [t, 1.0]);
		push(inner_pos - Vec3::Z * 0.5, -dir, [t, 0.0]);
		if i > 0 {
			let a = (i - 1) * 8;
			let b = i * 8;
			indices.extend([
				a,
				b,
				a + 1,
				b,
				b + 1,
				a + 1, // front
				a + 2,
				a + 3,
				b + 2,
				b + 2,
				a + 3,
				b + 3, // back
				a + 4,
				a + 5,
				b + 4,
				b + 4,
				a + 5,
				b + 5, // outer
				a + 6,
				b + 6,
				a + 7,
				b + 6,
				b + 7,
				a + 7, // inner
			]);
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
		Option<&'static VfxSonicRing>,
		&'static mut Transform,
		&'static mut Visibility,
		Option<&'static MeshMaterial3d<FieldMaterial>>,
		Option<&'static mut VfxLayerLife>,
	),
>;

pub fn tick_vfx_fields(
	instances: Query<(
		&VfxInstance,
		&VfxPendingLayers,
		Option<&VfxCancel>,
		Option<&VfxWaveStates>,
		&GlobalTransform,
	)>,
	mut materials: ResMut<Assets<FieldMaterial>>,
	mut fields: FieldTickQuery,
) {
	for (member, _field, ring, mut transform, mut visibility, material, life) in &mut fields {
		let Ok((instance, pending, cancel, waves, root)) = instances.get(member.root()) else {
			continue;
		};
		if !instance.armed {
			*visibility = Visibility::Hidden;
			continue;
		}
		if let Some(mut life) = life {
			life.age = instance.age;
		}
		let drive = pending.spawn.drive;
		let mut envelope = FieldEnvelope::sample_canceled(
			instance.age,
			&drive,
			cancel.map(|cancel| cancel.started_age),
		);
		if let Some(ring) = ring {
			let Some(wave) = waves.and_then(|states| states.get(ring.index)) else {
				*visibility = Visibility::Hidden;
				continue;
			};
			if wave.expired() {
				*visibility = Visibility::Hidden;
				continue;
			}
			let (_, rotation, translation) = root.to_scale_rotation_translation();
			*transform =
				wave.local_ring_transform(Transform { translation, rotation, scale: Vec3::ONE });
			envelope.alpha *= wave.fade(drive.speed, drive.fade_out);
			*visibility = Visibility::Inherited;
		} else {
			*visibility = Visibility::Inherited;
		}
		if let Some(handle) = material {
			if let Some(mut material) = materials.get_mut(&handle.0) {
				material.set_age(instance.age);
				material.set_envelope(envelope.alpha, envelope.reveal, envelope.contact);
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
	fn vortex_ring_is_a_hollow_hoop_facing_plus_z() -> Result<(), String> {
		let mesh = unit_vortex_ring_mesh();
		let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| a.as_float3())
		else {
			return Err("missing positions".into());
		};
		if positions.len() < 64 {
			return Err(format!("verts {}", positions.len()));
		}
		let mut max_z = 0.0f32;
		let mut min_radial = f32::MAX;
		let mut max_radial = 0.0f32;
		for position in positions {
			max_z = max_z.max(position[2].abs());
			let radial = Vec2::new(position[0], position[1]).length();
			min_radial = min_radial.min(radial);
			max_radial = max_radial.max(radial);
		}
		if (max_z - 0.5).abs() > 0.02 {
			return Err(format!("depth {max_z}"));
		}
		if min_radial < 0.30 || max_radial > 0.52 {
			return Err(format!("radial {min_radial}..{max_radial}"));
		}
		Ok(())
	}
}
