//! Optional glue from `attacks::AttackPresentationEvent` to driven VFX.
//!
//! Core effects stay on [`VfxDrive`]. This module mirrors the presentation hook
//! from [PR #1147](https://github.com/ramate-io/maybraid/pull/1147) with local
//! types so `maybraid-vfx` does not depend on the unmerged `attacks` crate.
//! [#1119](https://github.com/ramate-io/maybraid/issues/1119) copies event
//! fields into [`AttackDriveEvent`] and applies [`AttackDriveBindings`].
//!
//! TODO(#1147): once that PR merges, replace these local mirror types with a
//! feature-gated `attacks` dependency (`attack-presentation`) so there is only
//! one presentation-event model. Do not take a hard dep while `#1147` is
//! unmerged.
//!
//! Gameplay despawns entities tagged `attacks::AttackPresentationBinding` on
//! cancel, death, sleep, retirement, or despawn. After this crate spawns a
//! root, insert that `attacks` component on the same entity using
//! [`VfxAttackBinding`]'s instance and kind. This crate never names `attacks`
//! types.
//!
//! | `#1147` payload | Driven effect |
//! | --- | --- |
//! | `Hazard` + `Cylinder { radius, height }` | [`god_beam`](crate::effects::god_beam): `size` = diameter = `2 * radius`, `reach` = `height` |
//! | `Hazard` + `Wavefront { speed, length, .. }` | [`sonic_wave`](crate::effects::sonic_wave): `size` = `length`, `speed` = `speed`, one ring |
//! | `Telegraph`, `Stage` | ignored (windup must not double-play the column) |
//! | `Ended` / `Despawned` | remove the binding; gameplay may also despawn the root |
//!
//! Sonic-wave expansion is owned by the effect. The runtime's wavefront pose
//! origin is the **current** disc center, so [`AttackDriveBindings`] does not
//! follow that translation.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::composition::EffectKind;
use crate::drive::VfxDrive;
use crate::library::VfxLibrary;
use crate::spawn::{despawn_vfx, SpawnVfxExt, VfxSpawn};

/// Local copy of `attacks::AttackInstanceId(u64)`.
///
/// TODO(#1147): delete this mirror and use `attacks::AttackInstanceId` behind
/// the `attack-presentation` feature after that crate is on `main`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttackInstanceKey(pub u64);

/// Local copy of `attacks::AttackPresentationBindingKind`.
///
/// TODO(#1147): replace with `attacks::AttackPresentationBindingKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VfxAttackBindingKind {
	Telegraph,
	Hazard { index: usize },
}

/// Tag spawned VFX roots. [#1119](https://github.com/ramate-io/maybraid/issues/1119)
/// also inserts `attacks::AttackPresentationBinding` with the same ids.
///
/// TODO(#1147): keep this VFX-side tag or map 1:1 onto the `attacks` component
/// once the feature-gated dep exists.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct VfxAttackBinding {
	pub instance: AttackInstanceKey,
	pub kind: VfxAttackBindingKind,
}

/// Local copy of `attacks::PresentationPhase`.
///
/// TODO(#1147): replace with `attacks::PresentationPhase`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackDrivePhase {
	Spawned,
	Updated,
	Despawned,
}

/// Local copy of `attacks::HazardPose`.
///
/// TODO(#1147): replace with `attacks::HazardPose`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackDrivePose {
	pub origin: Vec3,
	pub rotation: Quat,
}

impl AttackDrivePose {
	pub fn new(origin: Vec3, rotation: Quat) -> Self {
		Self { origin, rotation }
	}

	pub fn transform(self) -> Transform {
		Transform { translation: self.origin, rotation: self.rotation, scale: Vec3::ONE }
	}
}

/// Hazard / telegraph volumes this adapter can drive. Other `#1147` shapes
/// (`Sphere`, `Box`, `Line`) stay [`Unsupported`].
///
/// TODO(#1147): replace with `attacks::HazardShape` (ignore unsupported variants).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AttackDriveShape {
	/// Vertical column. Local Y is up, base at the pose origin.
	Cylinder {
		radius: f32,
		height: f32,
	},
	/// Moving disc. Pose origin is the current wavefront center.
	Wavefront {
		radius: f32,
		speed: f32,
		length: f32,
	},
	Unsupported,
}

impl AttackDriveShape {
	pub fn cylinder(radius: f32, height: f32) -> Self {
		Self::Cylinder { radius, height }
	}

	pub fn wavefront(radius: f32, speed: f32, length: f32) -> Self {
		Self::Wavefront { radius, speed, length }
	}

	pub fn effect_kind(self) -> Option<EffectKind> {
		match self {
			Self::Cylinder { .. } => Some(EffectKind::GodBeam),
			Self::Wavefront { .. } => Some(EffectKind::SonicWave),
			Self::Unsupported => None,
		}
	}

	/// Cylinder width is **diameter** (`2 * radius`). One Wavefront hazard is one ring.
	pub fn drive(self) -> Option<VfxDrive> {
		match self {
			Self::Cylinder { radius, height } => Some(
				VfxDrive::god_beam()
					.with_size(radius.max(0.0) * 2.0)
					.with_reach(height.max(0.0)),
			),
			Self::Wavefront { speed, length, .. } => {
				Some(VfxDrive::sonic_wave().with_size(length).with_speed(speed).with_count(1))
			}
			Self::Unsupported => None,
		}
	}

	/// God Beam follows the column pose. Sonic wave owns its own expansion.
	pub fn follows_pose_translation(self) -> bool {
		matches!(self, Self::Cylinder { .. })
	}
}

/// Local copy of `attacks::AttackPresentationKind` payloads this crate cares about.
///
/// TODO(#1147): replace with `attacks::AttackPresentationKind`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AttackDriveKind {
	Stage,
	Telegraph { phase: AttackDrivePhase, pose: AttackDrivePose, shape: AttackDriveShape },
	Hazard { index: usize, phase: AttackDrivePhase, pose: AttackDrivePose, shape: AttackDriveShape },
	Ended,
}

/// Local copy of one `attacks::AttackPresentationEvent`.
///
/// TODO(#1147): replace with `attacks::AttackPresentationEvent` plus a
/// `From` impl, behind `attack-presentation`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackDriveEvent {
	pub instance: AttackInstanceKey,
	pub kind: AttackDriveKind,
}

impl AttackDriveEvent {
	pub fn hazard(
		instance: u64,
		index: usize,
		phase: AttackDrivePhase,
		pose: AttackDrivePose,
		shape: AttackDriveShape,
	) -> Self {
		Self {
			instance: AttackInstanceKey(instance),
			kind: AttackDriveKind::Hazard { index, phase, pose, shape },
		}
	}

	pub fn telegraph(
		instance: u64,
		phase: AttackDrivePhase,
		pose: AttackDrivePose,
		shape: AttackDriveShape,
	) -> Self {
		Self {
			instance: AttackInstanceKey(instance),
			kind: AttackDriveKind::Telegraph { phase, pose, shape },
		}
	}

	pub fn ended(instance: u64) -> Self {
		Self { instance: AttackInstanceKey(instance), kind: AttackDriveKind::Ended }
	}

	pub fn stage(instance: u64) -> Self {
		Self { instance: AttackInstanceKey(instance), kind: AttackDriveKind::Stage }
	}

	/// `#1119` copies `AttackPresentationEvent` fields here.
	pub fn from_presentation(instance: u64, kind: AttackDriveKind) -> Self {
		Self { instance: AttackInstanceKey(instance), kind }
	}

	pub fn binding(self) -> Option<VfxAttackBinding> {
		match self.kind {
			AttackDriveKind::Telegraph { .. } => Some(VfxAttackBinding {
				instance: self.instance,
				kind: VfxAttackBindingKind::Telegraph,
			}),
			AttackDriveKind::Hazard { index, .. } => Some(VfxAttackBinding {
				instance: self.instance,
				kind: VfxAttackBindingKind::Hazard { index },
			}),
			AttackDriveKind::Stage | AttackDriveKind::Ended => None,
		}
	}

	pub fn phase(self) -> Option<AttackDrivePhase> {
		match self.kind {
			AttackDriveKind::Telegraph { phase, .. } | AttackDriveKind::Hazard { phase, .. } => {
				Some(phase)
			}
			AttackDriveKind::Stage | AttackDriveKind::Ended => None,
		}
	}

	pub fn pose(self) -> Option<AttackDrivePose> {
		match self.kind {
			AttackDriveKind::Telegraph { pose, .. } | AttackDriveKind::Hazard { pose, .. } => {
				Some(pose)
			}
			AttackDriveKind::Stage | AttackDriveKind::Ended => None,
		}
	}

	pub fn shape(self) -> Option<AttackDriveShape> {
		match self.kind {
			AttackDriveKind::Telegraph { shape, .. } | AttackDriveKind::Hazard { shape, .. } => {
				Some(shape)
			}
			AttackDriveKind::Stage | AttackDriveKind::Ended => None,
		}
	}

	/// Only **hazard** cylinders and wavefronts spawn a driven look.
	pub fn presents(self) -> bool {
		matches!(self.kind, AttackDriveKind::Hazard { .. })
			&& self.shape().and_then(AttackDriveShape::effect_kind).is_some()
	}

	pub fn effect_kind(self) -> Option<EffectKind> {
		if !self.presents() {
			return None;
		}
		self.shape().and_then(AttackDriveShape::effect_kind)
	}

	pub fn drive(self) -> Option<VfxDrive> {
		if !self.presents() {
			return None;
		}
		self.shape().and_then(AttackDriveShape::drive)
	}

	pub fn spawn(self) -> Option<VfxSpawn> {
		let drive = self.drive()?;
		let pose = self.pose()?;
		Some(VfxSpawn { transform: pose.transform(), drive, ..Default::default() })
	}

	pub fn follow_transform(self) -> Option<Transform> {
		if !self.presents() {
			return None;
		}
		let pose = self.pose()?;
		let shape = self.shape()?;
		if shape.follows_pose_translation() {
			return Some(pose.transform());
		}
		None
	}

	pub fn should_clear_instance(self) -> bool {
		matches!(self.kind, AttackDriveKind::Ended)
	}

	pub fn should_despawn_binding(self) -> bool {
		self.phase() == Some(AttackDrivePhase::Despawned)
	}
}

/// Presenter table. Spawn on `Spawned`, follow `Updated`, drop on cancel.
///
/// Does not insert `attacks::AttackPresentationBinding` — the caller does.
#[derive(Resource, Default, Debug)]
pub struct AttackDriveBindings {
	entities: HashMap<(u64, VfxAttackBindingKind), Entity>,
}

impl AttackDriveBindings {
	pub fn entity(&self, binding: VfxAttackBinding) -> Option<Entity> {
		self.entities.get(&(binding.instance.0, binding.kind)).copied()
	}

	pub fn remember(&mut self, binding: VfxAttackBinding, entity: Entity) {
		self.entities.insert((binding.instance.0, binding.kind), entity);
	}

	pub fn take(&mut self, binding: VfxAttackBinding) -> Option<Entity> {
		self.entities.remove(&(binding.instance.0, binding.kind))
	}

	/// Apply one mirrored presentation event. Returns a newly spawned VFX root.
	pub fn apply(
		&mut self,
		commands: &mut Commands,
		library: &VfxLibrary,
		transforms: &mut Query<&mut Transform>,
		event: AttackDriveEvent,
	) -> Option<Entity> {
		if event.should_clear_instance() {
			self.clear_instance(commands, event.instance);
			return None;
		}
		let binding = event.binding()?;
		if event.should_despawn_binding() {
			if let Some(entity) = self.take(binding) {
				despawn_vfx(commands, entity);
			}
			return None;
		}
		if !event.presents() {
			return None;
		}
		if let Some(entity) = self.entity(binding) {
			if let Some(next) = event.follow_transform() {
				if let Ok(mut current) = transforms.get_mut(entity) {
					*current = next;
					return None;
				}
			} else if transforms.get(entity).is_ok() {
				return None;
			}
		}
		let spawn = event.spawn()?;
		let definition = match event.effect_kind()? {
			EffectKind::GodBeam => &library.god_beam,
			EffectKind::SonicWave => &library.sonic_wave,
			EffectKind::Burst => return None,
		};
		let entity = commands.spawn_vfx(definition, spawn);
		commands.entity(entity).insert(binding);
		self.remember(binding, entity);
		Some(entity)
	}

	pub fn clear_instance(&mut self, commands: &mut Commands, instance: AttackInstanceKey) {
		self.entities.retain(|(bound, _), entity| {
			if *bound == instance.0 {
				despawn_vfx(commands, *entity);
				false
			} else {
				true
			}
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::drive::{GOD_BEAM_WIDTH, SONIC_WAVE_SPEED};

	fn ground() -> AttackDrivePose {
		AttackDrivePose::new(Vec3::new(1.0, 0.0, 2.0), Quat::IDENTITY)
	}

	#[test]
	fn cylinder_hazard_maps_to_seven_meter_god_beam() -> Result<(), String> {
		let event = AttackDriveEvent::hazard(
			7,
			0,
			AttackDrivePhase::Spawned,
			ground(),
			AttackDriveShape::cylinder(3.5, 96.0),
		);
		let drive = event.drive().ok_or("expected drive")?;
		if event.effect_kind() != Some(EffectKind::GodBeam) {
			return Err("expected god beam".into());
		}
		if (drive.size - GOD_BEAM_WIDTH).abs() >= 1e-4 {
			return Err(format!("diameter {}", drive.size));
		}
		if (drive.reach - 96.0).abs() >= 1e-4 {
			return Err(format!("height {}", drive.reach));
		}
		if event.follow_transform() != Some(ground().transform()) {
			return Err("column should follow pose".into());
		}
		Ok(())
	}

	#[test]
	fn wavefront_hazard_maps_to_one_sonic_ring() -> Result<(), String> {
		let event = AttackDriveEvent::hazard(
			3,
			1,
			AttackDrivePhase::Spawned,
			ground(),
			AttackDriveShape::wavefront(1.4, 22.0, 28.0),
		);
		let drive = event.drive().ok_or("expected drive")?;
		if event.effect_kind() != Some(EffectKind::SonicWave) {
			return Err("expected sonic wave".into());
		}
		if (drive.size - 28.0).abs() >= 1e-4 {
			return Err(format!("range {}", drive.size));
		}
		if (drive.speed - SONIC_WAVE_SPEED).abs() >= 1e-4 {
			return Err(format!("speed {}", drive.speed));
		}
		if drive.count != 1 {
			return Err(format!("count {}", drive.count));
		}
		if event.follow_transform().is_some() {
			return Err("wavefront translation is owned by the effect".into());
		}
		Ok(())
	}

	#[test]
	fn telegraph_and_stage_do_not_spawn_driven_looks() {
		let pose = ground();
		let telegraph = AttackDriveEvent::telegraph(
			1,
			AttackDrivePhase::Spawned,
			pose,
			AttackDriveShape::cylinder(3.5, 96.0),
		);
		let stage = AttackDriveEvent::stage(1);
		assert!(telegraph.drive().is_none());
		assert!(stage.drive().is_none());
		assert!(!telegraph.presents());
	}

	#[test]
	fn ended_and_despawned_clear_presenter_state() {
		let ended = AttackDriveEvent::ended(9);
		let despawn = AttackDriveEvent::hazard(
			9,
			0,
			AttackDrivePhase::Despawned,
			ground(),
			AttackDriveShape::cylinder(3.5, 96.0),
		);
		assert!(ended.should_clear_instance());
		assert!(despawn.should_despawn_binding());
		assert_eq!(
			despawn.binding(),
			Some(VfxAttackBinding {
				instance: AttackInstanceKey(9),
				kind: VfxAttackBindingKind::Hazard { index: 0 },
			})
		);
	}

	#[test]
	fn unsupported_shapes_are_ignored() {
		let event = AttackDriveEvent::hazard(
			2,
			0,
			AttackDrivePhase::Spawned,
			ground(),
			AttackDriveShape::Unsupported,
		);
		assert!(event.drive().is_none());
		assert!(event.spawn().is_none());
	}

	#[test]
	fn binding_table_remembers_and_takes_roots() {
		let binding = VfxAttackBinding {
			instance: AttackInstanceKey(4),
			kind: VfxAttackBindingKind::Hazard { index: 2 },
		};
		let mut table = AttackDriveBindings::default();
		let entity = Entity::from_bits(11);
		table.remember(binding, entity);
		assert_eq!(table.entity(binding), Some(entity));
		assert_eq!(table.take(binding), Some(entity));
		assert!(table.entity(binding).is_none());
	}

	#[test]
	fn from_presentation_matches_event_fields() {
		let kind = AttackDriveKind::Hazard {
			index: 0,
			phase: AttackDrivePhase::Updated,
			pose: ground(),
			shape: AttackDriveShape::cylinder(3.5, 10.0),
		};
		let event = AttackDriveEvent::from_presentation(1147, kind);
		assert_eq!(event.instance, AttackInstanceKey(1147));
		assert_eq!(event.kind, kind);
	}
}
