use bevy::prelude::*;

use crate::hazard::WavefrontSpec;
use crate::{
	AimSnapshot, AttackEndReason, AttackId, AttackInstanceId, AttackStage, HazardPose, HazardShape,
};

/// Gameplay announcement. VFX crates subscribe; they do not own timing or Hits.
///
/// Kinds: `Stage`, `Telegraph`, `Hazard` (optional [`WavefrontPresentation`]),
/// and `Ended`. Bind presenter entities with [`AttackPresentationBinding`] so
/// interruption can despawn them.
#[derive(Message, Clone, Debug)]
pub struct AttackPresentationEvent {
	pub instance: AttackInstanceId,
	pub owner: Entity,
	pub definition: AttackId,
	pub kind: AttackPresentationKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttackPresentationKind {
	Stage {
		from: AttackStage,
		to: AttackStage,
		snapshot: AimSnapshot,
	},
	Telegraph {
		phase: PresentationPhase,
		pose: HazardPose,
		shape: HazardShape,
	},
	Hazard {
		index: usize,
		phase: PresentationPhase,
		pose: HazardPose,
		shape: HazardShape,
		wave: Option<WavefrontPresentation>,
	},
	Ended {
		reason: AttackEndReason,
	},
}

/// Authoritative per-wave state. Presenters follow this; they do not simulate motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WavefrontPresentation {
	/// Mouth / socket at emit time.
	pub origin: Vec3,
	/// World-space travel heading.
	pub direction: Vec3,
	/// Current ring center.
	pub position: Vec3,
	pub speed: f32,
	pub range: f32,
	pub radius: f32,
	/// Full thickness along travel (`2 * half_depth`).
	pub thickness: f32,
	pub wave_index: u8,
}

impl WavefrontPresentation {
	pub fn from_wave(
		wave: WavefrontSpec,
		emission: Vec3,
		heading: Vec3,
		travel: f32,
		wave_index: u8,
	) -> Self {
		let pose = wave.pose(emission, heading, travel);
		Self {
			origin: emission,
			direction: heading,
			position: pose.origin,
			speed: wave.speed,
			range: wave.range,
			radius: wave.radius,
			thickness: wave.half_depth.max(0.0) * 2.0,
			wave_index,
		}
	}

	/// Distance the front has travelled along [`Self::direction`].
	pub fn traveled(self) -> f32 {
		let heading = self.direction.normalize_or(Vec3::Z);
		(self.position - self.origin).dot(heading).max(0.0)
	}

	/// Seconds since emit, from travelled distance / speed.
	pub fn travel_age(self) -> f32 {
		if self.speed <= 1e-4 {
			0.0
		} else {
			self.traveled() / self.speed
		}
	}

	/// 1 at emit, 0 at [`Self::range`]. The compression edge stays readable
	/// while the trail fades.
	pub fn trailing_fade(self) -> f32 {
		if self.range <= 1e-4 {
			1.0
		} else {
			(1.0 - self.traveled() / self.range).clamp(0.0, 1.0)
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationPhase {
	Spawned,
	Updated,
	Despawned,
}

/// Tag a presenter-owned entity so cancellation, death, sleep, and retirement
/// can despawn it without knowing the VFX type.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackPresentationBinding {
	pub instance: AttackInstanceId,
	pub kind: AttackPresentationBindingKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttackPresentationBindingKind {
	Telegraph,
	Hazard { index: usize, wave: u8 },
}

impl AttackPresentationEvent {
	pub fn ended(
		instance: AttackInstanceId,
		owner: Entity,
		definition: AttackId,
		reason: AttackEndReason,
	) -> Self {
		Self { instance, owner, definition, kind: AttackPresentationKind::Ended { reason } }
	}

	pub fn with_kind(
		instance: AttackInstanceId,
		owner: Entity,
		definition: AttackId,
		kind: AttackPresentationKind,
	) -> Self {
		Self { instance, owner, definition, kind }
	}
}
