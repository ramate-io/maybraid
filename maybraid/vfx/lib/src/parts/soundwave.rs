//! Torus primitive for a directional soundwave. Bevy builds the torus around
//! local Y; [`soundwave_transform`] maps that axle onto the live heading so the
//! disc matches the gameplay wave (local +Z travel, disc in XY).

mod material;

pub use material::{SoundwaveMaterial, SoundwaveMaterialPlugin};

use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use crate::composition::SoundwavePart;
use crate::membership::VfxMemberOf;
use crate::spawn::{VfxLayerLife, VfxSpawn};

/// Authored torus major radius. Scale XZ so this matches gameplay radius.
pub const SOUNDWAVE_MAJOR: f32 = 0.55;
/// Authored torus minor radius. Scale Y so this matches gameplay thickness.
pub const SOUNDWAVE_MINOR: f32 = 0.08;

/// Unit torus used by the Bray consumer and the named recipe.
pub fn soundwave_torus_mesh() -> Mesh {
	Torus::new(SOUNDWAVE_MAJOR, SOUNDWAVE_MINOR).mesh().into()
}

/// Pose a Y-axle torus onto a traveling disc.
///
/// Gameplay [`attacks`] wavefronts use local +Z travel with the disc in XY.
/// Bevy's torus is around local Y, so the axle maps with
/// [`Quat::from_rotation_arc`] from `Y` to the heading — **not** from `Z`.
pub fn soundwave_transform(
	position: Vec3,
	direction: Vec3,
	radius: f32,
	thickness: f32,
) -> Transform {
	let heading = direction.normalize_or(Vec3::Z);
	Transform {
		translation: position,
		rotation: Quat::from_rotation_arc(Vec3::Y, heading),
		scale: Vec3::new(
			(radius.max(0.05) / SOUNDWAVE_MAJOR).max(1e-3),
			(thickness.max(0.05) / (SOUNDWAVE_MINOR * 2.0)).max(0.2),
			(radius.max(0.05) / SOUNDWAVE_MAJOR).max(1e-3),
		),
	}
}

/// One driven or recipe-spawned ring.
#[derive(Component, Debug)]
pub struct VfxSoundwave {
	pub origin: Vec3,
	pub direction: Vec3,
	pub radius: f32,
	pub thickness: f32,
	pub age: f32,
	pub playback: f32,
}

impl SoundwavePart {
	pub(crate) fn spawn(
		&self,
		commands: &mut Commands,
		parent: Entity,
		transform: Transform,
		spawn: &VfxSpawn,
	) {
		let heading = transform.rotation * Vec3::Y;
		let radius = SOUNDWAVE_MAJOR * transform.scale.x.max(spawn.clamped_scale());
		let thickness = SOUNDWAVE_MINOR * 2.0 * transform.scale.y.max(0.2);
		// Parent sits at the instance origin; uniforms are world metres so the
		// compression edge lines up with the gameplay wave / playground burst.
		let world_origin = spawn.transform.translation + transform.translation;
		let tint = spawn.tint.unwrap_or(Color::srgb(0.72, 0.88, 1.0));
		let material = SoundwaveMaterial::new(
			world_origin,
			heading,
			radius,
			thickness,
			tint,
			spawn.clamped_intensity(),
		);
		let local = soundwave_transform(Vec3::ZERO, heading, radius, thickness);
		commands.spawn((
			Name::new(format!("vfx-soundwave-{}", self.name)),
			ChildOf(parent),
			VfxMemberOf(parent),
			Mesh3d(self.mesh.clone()),
			local,
			Visibility::Inherited,
			NotShadowCaster,
			SoundwaveMaterialSlot(material),
			VfxSoundwave {
				origin: world_origin,
				direction: heading,
				radius,
				thickness,
				age: 0.0,
				playback: spawn.clamped_playback(),
			},
			VfxLayerLife {
				age: 0.0,
				duration: self.duration,
				playback: spawn.clamped_playback(),
				waiting_for_emitter: false,
			},
		));
	}
}

/// Temporary holder so [`stamp_soundwave_materials`] can add the asset after spawn.
#[derive(Component)]
pub struct SoundwaveMaterialSlot(pub SoundwaveMaterial);

pub fn stamp_soundwave_materials(
	mut commands: Commands,
	mut materials: ResMut<Assets<SoundwaveMaterial>>,
	pending: Query<(Entity, &SoundwaveMaterialSlot)>,
) {
	for (entity, slot) in &pending {
		let handle = materials.add(slot.0.clone());
		commands
			.entity(entity)
			.insert(MeshMaterial3d(handle))
			.remove::<SoundwaveMaterialSlot>();
	}
}

pub(crate) fn tick_soundwaves(
	time: Res<Time>,
	instances: Query<&crate::spawn::VfxInstance>,
	mut materials: ResMut<Assets<SoundwaveMaterial>>,
	mut waves: Query<(
		&VfxMemberOf,
		&mut VfxSoundwave,
		&mut Transform,
		Option<&MeshMaterial3d<SoundwaveMaterial>>,
		Option<&mut VfxLayerLife>,
	)>,
) {
	let dt = time.delta_secs();
	for (member, mut wave, mut transform, material, life) in &mut waves {
		if !member.instance_armed(&instances) {
			continue;
		}
		wave.age += dt * wave.playback;
		if let Some(mut life) = life {
			life.age = wave.age;
		}
		*transform = soundwave_transform(Vec3::ZERO, wave.direction, wave.radius, wave.thickness);
		if let Some(handle) = material {
			if let Some(mut material) = materials.get_mut(&handle.0) {
				material.follow(wave.origin, wave.direction, wave.radius, wave.thickness);
				material.set_age(wave.age);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn torus_axle_maps_to_wave_heading_not_local_z() {
		let heading = Vec3::X;
		let pose = soundwave_transform(Vec3::new(2.0, 1.0, -4.0), heading, 4.0, 0.7);
		let axle = pose.rotation * Vec3::Y;
		assert!(
			(axle - heading).length() < 1e-4,
			"Bevy torus axle is Y; it must map onto the heading, got {axle}"
		);
		let wrong = Quat::from_rotation_arc(Vec3::Z, heading) * Vec3::Y;
		assert!(
			(wrong - heading).length() > 0.5,
			"from_rotation_arc(Z, heading) is the orientation bug"
		);
		assert!((pose.translation - Vec3::new(2.0, 1.0, -4.0)).length() < 1e-5);
	}

	#[test]
	fn scale_tracks_gameplay_radius_and_thickness() {
		let pose = soundwave_transform(Vec3::ZERO, Vec3::NEG_Z, 4.0, 0.7);
		assert!((pose.scale.x * SOUNDWAVE_MAJOR - 4.0).abs() < 1e-4);
		assert!((pose.scale.z * SOUNDWAVE_MAJOR - 4.0).abs() < 1e-4);
		assert!((pose.scale.y * SOUNDWAVE_MINOR * 2.0 - 0.7).abs() < 1e-4);
	}
}
