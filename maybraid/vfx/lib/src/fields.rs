//! Procedural field meshes and age-driven field parts.

use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;

use crate::drive::{FieldEnvelope, VfxDrive, VfxWaveStates};
use crate::seed;
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

/// Curved speed stroke trailing a live sonic ring. Visual only.
#[derive(Component, Debug, Clone, Copy)]
pub struct VfxSonicTrail {
	pub ring_index: u32,
	pub slot: u32,
}

/// Bounded pose history per ring, recorded from authoritative [`SonicWaveState`] snapshots.
#[derive(Component, Clone, Debug, Default)]
pub struct VfxSonicHistory {
	pub rings: Vec<RingHistory>,
}

#[derive(Clone, Copy, Debug)]
pub struct SonicTrailSample {
	pub local_translation: Vec3,
	pub direction: Vec3,
	pub travel: f32,
}

const HISTORY_CAP: usize = 16;
const RECORD_TRAVEL_EPS: f32 = 0.08;

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

/// Tapered ribbon along +Z (0 = head at wavefront, 1 = tail). Width in X, depth in Y.
pub fn unit_speed_stroke_mesh() -> Mesh {
	let segments = 8u32;
	let mut positions = Vec::new();
	let mut normals = Vec::new();
	let mut uvs = Vec::new();
	let mut indices = Vec::new();
	for i in 0..=segments {
		let t = i as f32 / segments as f32;
		let width = (1.0 - t * 0.82) * 0.5;
		let z = t;
		for side in [-1.0f32, 1.0] {
			positions.push([side * width, 0.0, z]);
			normals.push([0.0, 1.0, 0.0]);
			uvs.push([side * 0.5 + 0.5, t]);
		}
		if i > 0 {
			let base = (i - 1) * 2;
			let next = i * 2;
			indices.extend([base, next, base + 1, next, next + 1, base + 1]);
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
		Option<&'static VfxSonicTrail>,
		&'static mut Transform,
		&'static mut Visibility,
		Option<&'static MeshMaterial3d<FieldMaterial>>,
		Option<&'static mut VfxLayerLife>,
	),
>;

pub fn record_sonic_wave_history(
	mut instances: Query<(
		&VfxInstance,
		&VfxWaveStates,
		&mut VfxSonicHistory,
		&GlobalTransform,
		Option<&VfxCancel>,
	)>,
) {
	for (instance, waves, mut history, root, cancel) in &mut instances {
		if !instance.armed || cancel.is_some() {
			history.rings.clear();
			continue;
		}
		let (_, rotation, translation) = root.to_scale_rotation_translation();
		let root_tf = Transform { translation, rotation, scale: Vec3::ONE };
		for wave in &waves.waves {
			if wave.expired() {
				continue;
			}
			let slot = history.ring_slot(wave.index);
			let local = wave.local_ring_transform(root_tf).translation;
			let travel = wave.travel();
			let ring = &mut history.rings[slot];
			if ring.samples.is_empty()
				|| travel - ring.last_travel >= RECORD_TRAVEL_EPS
				|| ring.samples.last().map(|s| s.local_translation.distance(local)).unwrap_or(0.0)
					>= RECORD_TRAVEL_EPS
			{
				ring.samples.push(SonicTrailSample {
					local_translation: local,
					direction: wave.direction,
					travel,
				});
				ring.last_travel = travel;
				if ring.samples.len() > HISTORY_CAP {
					let drop = ring.samples.len() - HISTORY_CAP;
					ring.samples.drain(0..drop);
				}
			}
		}
	}
}

impl VfxSonicHistory {
	fn ring_slot(&mut self, index: u32) -> usize {
		if let Some(pos) = self.rings.iter().position(|ring| ring.index == index) {
			return pos;
		}
		self.rings.push(RingHistory { index, ..RingHistory::default() });
		self.rings.len() - 1
	}

	pub fn ring(&self, index: u32) -> Option<&RingHistory> {
		self.rings.iter().find(|ring| ring.index == index)
	}
}

#[derive(Clone, Debug, Default)]
pub struct RingHistory {
	pub index: u32,
	pub samples: Vec<SonicTrailSample>,
	pub last_travel: f32,
}

/// Visual-only ribbon pose from recorded history. Does not affect gameplay motion.
pub fn sonic_trail_pose(
	history: &[SonicTrailSample],
	direction: Vec3,
	radius: f32,
	slot: u32,
	ribbon_count: u32,
	seed: u64,
	max_length: f32,
) -> Option<Transform> {
	if history.len() < 2 || ribbon_count == 0 {
		return None;
	}
	let dir = if direction.length_squared() > 1e-8 { direction.normalize() } else { Vec3::Z };
	let up = if dir.y.abs() > 0.95 { Vec3::X } else { Vec3::Y };
	let lateral = dir.cross(up).normalize_or_zero();
	let spin = seed::unit(seed::stream(seed, seed::LAYER_TRAIL, slot)) * std::f32::consts::TAU;
	let lateral = Quat::from_axis_angle(dir, spin + slot as f32 * 0.9) * lateral;
	let length_scale =
		(0.45 + 0.55 * seed::unit(seed::stream(seed, seed::LAYER_TRAIL, slot + 17)))
			* max_length.max(0.25);
	let head = history[0].local_translation;
	let tail_index = history.len().min(6).saturating_sub(1);
	let tail = history[tail_index].local_translation;
	let chord = head - tail;
	let length = chord.length().min(length_scale).max(0.15);
	let tail_point = head - dir * length;
	let curl = lateral * radius * (0.18 + 0.12 * slot as f32);
	let mid = (head + tail_point) * 0.5 + curl;
	let tangent = (head - mid).normalize_or_zero();
	let rotation = Quat::from_rotation_arc(Vec3::Z, tangent);
	let width = radius
		* (0.05 + 0.04 * seed::unit(seed::stream(seed, seed::LAYER_TRAIL, slot + 3)));
	Some(Transform {
		translation: head - dir * radius * 0.04 + lateral * radius * 0.08,
		rotation,
		scale: Vec3::new(width, 1.0, length),
	})
}

pub fn tick_vfx_fields(
	instances: Query<(
		&VfxInstance,
		&VfxPendingLayers,
		Option<&VfxCancel>,
		Option<&VfxWaveStates>,
		Option<&VfxSonicHistory>,
		&GlobalTransform,
	)>,
	mut materials: ResMut<Assets<FieldMaterial>>,
	mut fields: FieldTickQuery,
) {
	for (member, _field, ring, trail, mut transform, mut visibility, material, life) in &mut fields
	{
		let Ok((instance, pending, cancel, waves, history, root)) =
			instances.get(member.root())
		else {
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
		let visual = pending.spawn.field_visual.sonic;
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
		} else if let Some(trail) = trail {
			let Some(wave) = waves.and_then(|states| states.get(trail.ring_index)) else {
				*visibility = Visibility::Hidden;
				continue;
			};
			if wave.expired() {
				*visibility = Visibility::Hidden;
				continue;
			}
			let ring_history = history.and_then(|history| history.ring(trail.ring_index));
			let samples = ring_history.map(|ring| ring.samples.as_slice()).unwrap_or(&[]);
			if let Some(pose) = sonic_trail_pose(
				samples,
				wave.direction,
				wave.radius,
				trail.slot,
				visual.trail_ribbons_per_ring.max(1),
				instance.seed,
				visual.trail_max_length,
			) {
				*transform = pose;
				envelope.alpha *= wave.fade(drive.speed, drive.fade_out);
				*visibility = Visibility::Inherited;
			} else {
				*visibility = Visibility::Hidden;
				continue;
			}
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
	fn trail_pose_bends_behind_travel_direction() -> Result<(), String> {
		let samples = [
			SonicTrailSample {
				local_translation: Vec3::new(4.0, 0.0, 0.0),
				direction: Vec3::Z,
				travel: 4.0,
			},
			SonicTrailSample {
				local_translation: Vec3::new(3.0, 0.0, 0.0),
				direction: Vec3::Z,
				travel: 3.0,
			},
			SonicTrailSample {
				local_translation: Vec3::new(2.0, 0.0, 0.0),
				direction: Vec3::Z,
				travel: 2.0,
			},
		];
		let pose = sonic_trail_pose(&samples, Vec3::Z, 0.55, 0, 4, 42, 2.5).ok_or("pose")?;
		if pose.translation.z >= samples[0].local_translation.z {
			return Err("trail head should sit behind the wavefront".into());
		}
		if pose.scale.z < 0.1 {
			return Err("trail length missing".into());
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
