//! Emissive open-cross reticle at the first fixed hit along the firearm barrel.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::light::NotShadowCaster;
#[cfg(test)]
use bevy::mesh::VertexAttributeValues;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use damage::DamageApplied;
use firearms::{muzzle_world, BoneMap, FirearmMembers, FirearmRoot, RigRoot};
use lod_avian::PhysicsInteractionLayer;

use player::CameraFollow;

use crate::pose::HeldFirearm;
use crate::FirearmUser;

const REST_COLOR: Color = Color::srgb(0.45, 1.0, 0.95);
const FLASH_COLOR: Color = Color::srgb(1.0, 0.88, 0.28);
const REST_EMISSIVE: f32 = 12.0;
const FLASH_EMISSIVE: f32 = 26.0;
const FLASH_SECS: f32 = 0.12;
const FLASH_SCALE: f32 = 1.45;

/// Half-extent of the cross in mesh space. Uniform scale turns this into world size.
const OUTER: f32 = 1.0;
/// Inner radius of the open center. Strokes start here so the aim point stays clear.
const GAP: f32 = 0.36;
/// Half-width of each stroke.
const HALF_THICK: f32 = 0.11;

/// Four strokes, each listed counter-clockwise when looking along -Z (mesh normal is +Z).
const ARMS: [[[f32; 2]; 4]; 4] = [
	[[OUTER, HALF_THICK], [GAP, HALF_THICK], [GAP, -HALF_THICK], [OUTER, -HALF_THICK]],
	[[-GAP, HALF_THICK], [-OUTER, HALF_THICK], [-OUTER, -HALF_THICK], [-GAP, -HALF_THICK]],
	[[HALF_THICK, OUTER], [-HALF_THICK, OUTER], [-HALF_THICK, GAP], [HALF_THICK, GAP]],
	[[HALF_THICK, -GAP], [-HALF_THICK, -GAP], [-HALF_THICK, -OUTER], [HALF_THICK, -OUTER]],
];

#[derive(Component, Clone, Copy, Debug)]
pub struct Reticle {
	pub aim_distance: f32,
	pub surface_lift: f32,
	pub angular_size: f32,
	pub flash_until: f32,
}

impl Default for Reticle {
	fn default() -> Self {
		Self { aim_distance: 100.0, surface_lift: 0.015, angular_size: 0.018, flash_until: 0.0 }
	}
}

impl Reticle {
	pub fn flashing(self, now: f32) -> bool {
		now < self.flash_until
	}

	/// Unit open cross in the XY plane, facing +Z.
	pub fn mesh() -> Mesh {
		let mut positions = Vec::with_capacity(ARMS.len() * 4);
		let mut normals = Vec::with_capacity(ARMS.len() * 4);
		let mut uvs = Vec::with_capacity(ARMS.len() * 4);
		let mut indices = Vec::with_capacity(ARMS.len() * 6);
		for corners in ARMS {
			let base = positions.len() as u32;
			for [x, y] in corners {
				positions.push([x, y, 0.0]);
				normals.push([0.0, 0.0, 1.0]);
				uvs.push([0.0, 0.0]);
			}
			indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
		}
		Mesh::new(PrimitiveTopology::TriangleList, default())
			.with_inserted_indices(Indices::U32(indices))
			.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
			.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
			.with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
	}

	/// Place the cross on `target`, facing the camera so the opening stays square in view.
	pub fn pose(target: Vec3, camera: &GlobalTransform, size: f32) -> Transform {
		let mut transform = Transform::from_translation(target).with_scale(Vec3::splat(size));
		let away = target - camera.translation();
		if away.length_squared() > 1e-8 {
			transform.look_to(away, camera.up());
		}
		transform
	}
}

pub fn spawn_reticle(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	commands.spawn((
		Name::new("reticle"),
		Reticle::default(),
		Mesh3d(meshes.add(Reticle::mesh())),
		MeshMaterial3d(materials.add(glow_material(REST_COLOR, REST_EMISSIVE))),
		Transform::IDENTITY,
		Visibility::Hidden,
		NotShadowCaster,
	));
}

pub(crate) fn ingest_hit_markers(
	time: Res<Time>,
	mut hits: MessageReader<DamageApplied>,
	followed: Query<Entity, With<CameraFollow>>,
	mut reticles: Query<&mut Reticle>,
) {
	let Ok(shooter) = followed.single() else {
		return;
	};
	if !hits.read().any(|hit| hit.source == Some(shooter)) {
		return;
	}
	let until = time.elapsed_secs() + FLASH_SECS;
	for mut reticle in &mut reticles {
		reticle.flash_until = until;
	}
}

pub(crate) fn update_reticle(
	time: Res<Time>,
	spatial: SpatialQuery,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
	users: Query<&FirearmUser, With<CameraFollow>>,
	guns: Query<&FirearmMembers, (With<HeldFirearm>, With<FirearmRoot>)>,
	maps: Query<&BoneMap, With<RigRoot>>,
	globals: Query<&GlobalTransform, Without<Camera3d>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut reticles: Query<(
		&Reticle,
		&MeshMaterial3d<StandardMaterial>,
		&mut Transform,
		&mut Visibility,
	)>,
) {
	let Ok(camera) = cameras.single() else {
		return;
	};
	let Some(user) = users.iter().next() else {
		return;
	};
	let Some((origin, direction)) = barrel_ray(user.held, &guns, &maps, &globals) else {
		return;
	};
	let Ok((reticle, mesh_material, mut transform, mut visibility)) = reticles.single_mut() else {
		return;
	};

	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);
	let (target, normal) = if let Some(hit) =
		spatial.cast_ray(origin, direction, reticle.aim_distance, true, &filter)
	{
		(origin + *direction * hit.distance, hit.normal)
	} else {
		(origin + *direction * reticle.aim_distance, -*direction)
	};
	let target = target + normal * reticle.surface_lift;
	let distance = camera.translation().distance(target);
	let flashing = reticle.flashing(time.elapsed_secs());
	let size = (distance * reticle.angular_size).clamp(0.04, 1.8);
	let size = if flashing { size * FLASH_SCALE } else { size };
	*transform = Reticle::pose(target, camera, size);
	*visibility = Visibility::Visible;
	if let Some(mut standard) = materials.get_mut(&mesh_material.0) {
		let (color, gain) =
			if flashing { (FLASH_COLOR, FLASH_EMISSIVE) } else { (REST_COLOR, REST_EMISSIVE) };
		*standard = glow_material(color, gain);
	}
}

fn glow_material(color: Color, gain: f32) -> StandardMaterial {
	let glow = color.to_linear();
	StandardMaterial {
		base_color: color,
		emissive: LinearRgba::rgb(glow.red * gain, glow.green * gain, glow.blue * gain),
		unlit: true,
		cull_mode: None,
		depth_bias: -10.0,
		..default()
	}
}

fn barrel_ray(
	held: Entity,
	guns: &Query<&FirearmMembers, (With<HeldFirearm>, With<FirearmRoot>)>,
	maps: &Query<&BoneMap, With<RigRoot>>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
) -> Option<(Vec3, Dir3)> {
	let members = guns.get(held).ok()?;
	for member in members.iter() {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&barrel) = map.by_name.get("barrel") else {
			continue;
		};
		let Ok(global) = globals.get(barrel) else {
			continue;
		};
		let (muzzle, direction) = muzzle_world(global);
		let Ok(direction) = Dir3::new(direction) else {
			continue;
		};
		return Some((muzzle, direction));
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flash_is_live_until_deadline() {
		let reticle = Reticle { flash_until: 1.2, ..Reticle::default() };
		assert!(reticle.flashing(1.1));
		assert!(!reticle.flashing(1.2));
	}

	#[test]
	fn open_cross_leaves_the_center_clear() -> anyhow::Result<()> {
		let mesh = Reticle::mesh();
		let VertexAttributeValues::Float32x3(positions) =
			mesh.try_attribute(Mesh::ATTRIBUTE_POSITION)?
		else {
			anyhow::bail!("expected positions");
		};
		anyhow::ensure!(positions.len() == 16, "four strokes");
		let mut right = false;
		let mut left = false;
		let mut up = false;
		let mut down = false;
		for p in positions {
			let inside = p[0].abs() < GAP - 1e-4 && p[1].abs() < GAP - 1e-4;
			anyhow::ensure!(!inside, "vertex in the open center: {p:?}");
			right |= p[0] > GAP && p[1].abs() <= HALF_THICK + 1e-4;
			left |= p[0] < -GAP && p[1].abs() <= HALF_THICK + 1e-4;
			up |= p[1] > GAP && p[0].abs() <= HALF_THICK + 1e-4;
			down |= p[1] < -GAP && p[0].abs() <= HALF_THICK + 1e-4;
		}
		anyhow::ensure!(right && left && up && down, "missing a stroke");
		Ok(())
	}

	#[test]
	fn pose_faces_the_camera() -> anyhow::Result<()> {
		let camera = GlobalTransform::from_xyz(0.0, 1.0, 4.0);
		let target = Vec3::new(0.0, 1.0, 0.0);
		let size = 0.4;
		let pose = Reticle::pose(target, &camera, size);
		let away = (target - camera.translation()).normalize();
		let facing = pose.forward().as_vec3().dot(away);
		anyhow::ensure!(facing > 0.99, "cross should look along the view, got {facing}");
		anyhow::ensure!((pose.translation - target).length() < 1e-4);
		anyhow::ensure!((pose.scale.x - size).abs() < 1e-4);
		let upright = pose.up().as_vec3().dot(camera.up().as_vec3());
		anyhow::ensure!(upright > 0.99, "cross should stay screen-up, got {upright}");
		Ok(())
	}
}
