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
/// Trailing wake length along heading, in gameplay thicknesses.
pub const SOUNDWAVE_WAKE_ALONG: f32 = 4.0;

/// Unit torus used by the Bray consumer and the named recipe.
pub fn soundwave_torus_mesh() -> Mesh {
	Torus::new(SOUNDWAVE_MAJOR, SOUNDWAVE_MINOR).mesh().into()
}

/// Discrete trailing ribbons around local Y. Pose maps that axle onto the
/// heading so the wisps sit on the ring and extend behind the front.
pub fn soundwave_wake_mesh() -> Mesh {
	let mut combined: Option<Mesh> = None;
	let count = 7u32;
	for i in 0..count {
		let angle = i as f32 * std::f32::consts::TAU / count as f32;
		let mut piece = Mesh::from(Cuboid::new(0.14, 1.0, 0.028));
		piece.transform_by(Transform {
			translation: Vec3::new(angle.cos(), 0.0, angle.sin()),
			rotation: Quat::from_rotation_y(-angle),
			scale: Vec3::ONE,
		});
		match &mut combined {
			None => combined = Some(piece),
			Some(mesh) => {
				mesh.merge(&piece).expect("soundwave wake ribbons share vertex layout");
			}
		}
	}
	combined.expect("wake mesh has ribbons")
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

/// Pose the Hanabi trail so a unit circle sits on the live ring.
///
/// Y maps onto the heading; XZ scale is gameplay radius. Y scale stays 1 so
/// behind-the-front velocity is not stretched by the wake length.
pub fn soundwave_trail_transform(position: Vec3, direction: Vec3, radius: f32) -> Transform {
	let heading = direction.normalize_or(Vec3::Z);
	Transform {
		translation: position,
		rotation: Quat::from_rotation_arc(Vec3::Y, heading),
		scale: Vec3::new(radius.max(0.05), 1.0, radius.max(0.05)),
	}
}

/// Pose a Y-axle cylinder so it extends behind the travelling front.
pub fn soundwave_wake_transform(
	position: Vec3,
	direction: Vec3,
	radius: f32,
	thickness: f32,
) -> Transform {
	let heading = direction.normalize_or(Vec3::Z);
	let length = thickness.max(0.05) * SOUNDWAVE_WAKE_ALONG;
	Transform {
		translation: position - heading * (length * 0.5),
		rotation: Quat::from_rotation_arc(Vec3::Y, heading),
		scale: Vec3::new(radius.max(0.05), length.max(0.05), radius.max(0.05)),
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
		let radius = SOUNDWAVE_MAJOR * transform.scale.x.abs().max(transform.scale.z.abs());
		let thickness = SOUNDWAVE_MINOR * 2.0 * transform.scale.y.abs();
		// Initial local values are synchronized to world space after propagation.
		let world_origin = transform.translation;
		let tint = spawn.tint.unwrap_or(Color::srgb(0.72, 0.88, 1.0));
		let material = SoundwaveMaterial::new(
			world_origin,
			heading,
			radius,
			thickness,
			tint,
			spawn.clamped_intensity(),
		);
		commands.spawn((
			Name::new(format!("vfx-soundwave-{}", self.name)),
			ChildOf(parent),
			VfxMemberOf(parent),
			Mesh3d(self.mesh.clone()),
			transform,
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
		Option<&MeshMaterial3d<SoundwaveMaterial>>,
		Option<&mut VfxLayerLife>,
	)>,
) {
	let dt = time.delta_secs();
	for (member, mut wave, material, life) in &mut waves {
		if !member.instance_armed(&instances) {
			continue;
		}
		wave.age += dt * wave.playback;
		if let Some(mut life) = life {
			life.age = wave.age;
		}
		if let Some(handle) = material {
			if let Some(mut material) = materials.get_mut(&handle.0) {
				material.set_age(wave.age);
			}
		}
	}
}

/// Material coordinates follow the full scene hierarchy without rewriting local placement.
pub(crate) fn sync_soundwave_placement(
	mut materials: ResMut<Assets<SoundwaveMaterial>>,
	mut waves: Query<(&GlobalTransform, &mut VfxSoundwave, &MeshMaterial3d<SoundwaveMaterial>)>,
) {
	for (transform, mut wave, handle) in &mut waves {
		let affine = transform.affine();
		wave.origin = transform.translation();
		wave.direction = affine.transform_vector3(Vec3::Y).normalize_or(Vec3::Y);
		wave.radius = SOUNDWAVE_MAJOR
			* affine
				.transform_vector3(Vec3::X)
				.length()
				.max(affine.transform_vector3(Vec3::Z).length());
		wave.thickness = SOUNDWAVE_MINOR * 2.0 * affine.transform_vector3(Vec3::Y).length();
		if let Some(mut material) = materials.get_mut(&handle.0) {
			material.follow(wave.origin, wave.direction, wave.radius, wave.thickness);
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

	#[test]
	fn wake_extends_behind_the_live_front() {
		let front = Vec3::new(4.0, 2.0, 3.0);
		let heading = Vec3::X;
		let thickness = 0.7;
		let pose = soundwave_wake_transform(front, heading, 2.0, thickness);
		let axle = pose.rotation * Vec3::Y;
		assert!(
			(axle - heading).length() < 1e-4,
			"wake cylinder axle must map onto the heading, got {axle}"
		);
		let length = thickness * SOUNDWAVE_WAKE_ALONG;
		let expected = front - heading * (length * 0.5);
		assert!(
			(pose.translation - expected).length() < 1e-4,
			"wake must sit behind the front, got {}",
			pose.translation
		);
		assert!((pose.scale.x - 2.0).abs() < 1e-4);
		assert!((pose.scale.y - length).abs() < 1e-4);
		assert!(pose.translation.x < front.x, "wake center must trail the front");
	}

	#[test]
	fn trail_emitter_sits_on_the_live_ring() {
		let front = Vec3::new(4.0, 2.0, 3.0);
		let heading = Vec3::X;
		let pose = soundwave_trail_transform(front, heading, 2.0);
		let axle = pose.rotation * Vec3::Y;
		assert!((axle - heading).length() < 1e-4);
		assert!((pose.translation - front).length() < 1e-4);
		assert!((pose.scale - Vec3::new(2.0, 1.0, 2.0)).length() < 1e-4);
	}
	#[test]
	fn recipe_ring_keeps_local_pose_and_world_uniforms_follow_the_hierarchy(
	) -> Result<(), Box<dyn std::error::Error>> {
		use crate::composition::{EffectDefinition, EffectLayer};
		use crate::spawn::{spawn_vfx, VfxInstance};
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin))
			.init_asset::<SoundwaveMaterial>()
			.add_systems(Update, (stamp_soundwave_materials, tick_soundwaves).chain())
			.add_systems(
				PostUpdate,
				sync_soundwave_placement.after(bevy::transform::TransformSystems::Propagate),
			);
		let local = Transform::from_xyz(1.0, 2.0, 3.0)
			.with_rotation(Quat::from_rotation_z(0.5))
			.with_scale(Vec3::splat(2.0));
		let definition = EffectDefinition::new(
			"ring",
			[EffectLayer::soundwave(SoundwavePart {
				name: "ring".into(),
				mesh: default(),
				duration: 1.0,
			})
			.with_transform(local)],
		);
		let root = spawn_vfx(&mut app.world_mut().commands(), &definition, default());
		app.world_mut().flush();
		let placement = Transform::from_xyz(5.0, 6.0, 7.0)
			.with_rotation(Quat::from_rotation_y(0.7))
			.with_scale(Vec3::splat(3.0));
		app.world_mut().entity_mut(root).insert(placement);
		app.world_mut()
			.entity_mut(root)
			.get_mut::<VfxInstance>()
			.ok_or("missing instance")?
			.armed = true;
		app.update();
		app.update();
		let (transform, wave, material) = app
			.world_mut()
			.query::<(&Transform, &VfxSoundwave, &MeshMaterial3d<SoundwaveMaterial>)>()
			.single(app.world())?;
		assert_eq!(*transform, local);
		let expected = placement.to_matrix() * local.to_matrix();
		assert!(wave.origin.abs_diff_eq(expected.transform_point3(Vec3::ZERO), 1e-4));
		assert!(wave
			.direction
			.abs_diff_eq(expected.transform_vector3(Vec3::Y).normalize(), 1e-4));
		assert!((wave.radius - SOUNDWAVE_MAJOR * 6.0).abs() < 1e-4);
		let material = app
			.world()
			.resource::<Assets<SoundwaveMaterial>>()
			.get(&material.0)
			.ok_or("missing material")?;
		assert!(material.wave.truncate().abs_diff_eq(wave.origin, 1e-4));
		Ok(())
	}
}
