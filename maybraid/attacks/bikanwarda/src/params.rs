//! Authored Bikanwarda attack numbers.
//!
//! Values marked **PLACEHOLDER** are not agreed in
//! [#1119](https://github.com/ramate-io/maybraid/issues/1119) or its comments.
//! Agreed facts stay next to the field. Callers copy [`Self::PLACEHOLDER`] and
//! override; the runtime never invents a missing number.

use attacks::{AttackDefaults, DamageFalloff, DamageSpec};
use bevy::prelude::*;

/// Tunable Parade / God Beam / Bray recipe. Register into [`AttackCatalog`](attacks::AttackCatalog)
/// from a copy of this resource — do not edit gameplay policy in the driver.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct BikanwardaAttackParams {
	pub parade: ParadeParams,
	pub god_beam: GodBeamParams,
	pub bray: BrayParams,
}

impl BikanwardaAttackParams {
	/// Published placeholder set. See each field's comment for agreement status.
	pub const PLACEHOLDER: Self = Self {
		parade: ParadeParams::PLACEHOLDER,
		god_beam: GodBeamParams::PLACEHOLDER,
		bray: BrayParams::PLACEHOLDER,
	};
}

impl Default for BikanwardaAttackParams {
	fn default() -> Self {
		Self::PLACEHOLDER
	}
}

/// Line telegraph, then sequential spherical blasts along the frozen aim.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParadeParams {
	/// PLACEHOLDER. [#1119](https://github.com/ramate-io/maybraid/issues/1119) asks for a warning line, not a duration.
	pub windup: f32,
	/// PLACEHOLDER recovery after the last blast.
	pub recovery: f32,
	/// PLACEHOLDER. Shared authored cooldown until Parade spacing is agreed.
	pub cooldown: f32,
	/// PLACEHOLDER blast count along the line.
	pub blast_count: u8,
	/// PLACEHOLDER seconds between successive blasts.
	pub interval: f32,
	/// PLACEHOLDER meters from the owner to the first explosion center.
	pub first_distance: f32,
	/// PLACEHOLDER meters between successive explosion centers.
	pub spacing: f32,
	/// PLACEHOLDER explosion gameplay radius.
	pub blast_radius: f32,
	/// PLACEHOLDER execute window for one blast (must be long enough for one pulse).
	pub blast_execute: f32,
	/// PLACEHOLDER telegraph half-width.
	pub line_half_width: f32,
	/// PLACEHOLDER world Y for blast centers (ground-hugging stand-in).
	pub blast_height: f32,
	/// PLACEHOLDER once-per-blast damage. Copies [`AttackDefaults::AUTHORED`].
	pub blast_damage: f32,
	/// PLACEHOLDER linear falloff radius. `None` is full damage in the sphere.
	pub blast_falloff: Option<f32>,
}

impl ParadeParams {
	pub const PLACEHOLDER: Self = Self {
		windup: 1.4,
		recovery: 0.8,
		cooldown: AttackDefaults::AUTHORED.cooldown,
		blast_count: 5,
		interval: 0.35,
		first_distance: 4.0,
		spacing: 4.0,
		blast_radius: 3.0,
		blast_execute: 0.2,
		line_half_width: 0.55,
		blast_height: 0.5,
		blast_damage: AttackDefaults::AUTHORED.damage,
		blast_falloff: Some(3.0),
	};

	pub fn line_length(self) -> f32 {
		let extra = f32::from(self.blast_count.saturating_sub(1)) * self.spacing.max(0.0);
		(self.first_distance.max(0.0) + extra + self.blast_radius.max(0.0)).max(0.5)
	}

	pub fn execution(self) -> f32 {
		let last = f32::from(self.blast_count.saturating_sub(1)) * self.interval.max(0.0);
		last + self.blast_execute.max(0.05)
	}

	pub fn blast_damage_spec(self) -> DamageSpec {
		DamageSpec {
			amount: self.blast_damage.max(0.0),
			cadence: 0.0,
			falloff: match self.blast_falloff {
				Some(radius) => DamageFalloff::Linear { radius },
				None => DamageFalloff::None,
			},
		}
	}

	/// World center for blast `index` on a frozen snapshot.
	pub fn blast_point(self, origin: Vec3, direction: Vec3, index: u8) -> Vec3 {
		let heading = flatten(direction);
		let distance = self.first_distance.max(0.0) + f32::from(index) * self.spacing.max(0.0);
		origin + heading * distance + Vec3::Y * (self.blast_height - origin.y)
	}
}

/// Heavenly column on the observed aim point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GodBeamParams {
	/// Agreed: warning, then a 10 s active column
	/// ([#1119](https://github.com/ramate-io/maybraid/issues/1119),
	/// [#1104](https://github.com/ramate-io/maybraid/issues/1104)).
	pub execution: f32,
	/// PLACEHOLDER eye-glow / telegraph hold.
	pub windup: f32,
	/// PLACEHOLDER.
	pub recovery: f32,
	/// PLACEHOLDER. Shared authored cooldown.
	pub cooldown: f32,
	/// Agreed 7 m column width. Whether this is **diameter or radius** is not agreed.
	pub column_width: f32,
	/// PLACEHOLDER interpretation of [`Self::column_width`]. `true` treats 7 m as diameter
	/// (radius 3.5), matching draft VFX [#1150](https://github.com/ramate-io/maybraid/pull/1150).
	pub width_is_diameter: bool,
	/// PLACEHOLDER gameplay column height. VFX may draw a taller sky shaft.
	pub column_height: f32,
	/// PLACEHOLDER standing damage. Copies [`AttackDefaults::AUTHORED`].
	pub damage: f32,
	/// PLACEHOLDER pulse cadence. Copies [`AttackDefaults::AUTHORED`].
	pub cadence: f32,
	/// PLACEHOLDER telegraph disc radius matches the column unless overridden.
	pub telegraph_radius: Option<f32>,
}

impl GodBeamParams {
	pub const PLACEHOLDER: Self = Self {
		execution: 10.0,
		windup: 1.6,
		recovery: 1.0,
		cooldown: AttackDefaults::AUTHORED.cooldown,
		column_width: 7.0,
		width_is_diameter: true,
		column_height: 24.0,
		damage: AttackDefaults::AUTHORED.damage,
		cadence: AttackDefaults::AUTHORED.cadence,
		telegraph_radius: None,
	};

	pub fn column_radius(self) -> f32 {
		if self.width_is_diameter {
			self.column_width.max(0.0) * 0.5
		} else {
			self.column_width.max(0.0)
		}
	}

	pub fn warning_radius(self) -> f32 {
		self.telegraph_radius.unwrap_or_else(|| self.column_radius())
	}

	pub fn damage_spec(self) -> DamageSpec {
		DamageSpec {
			amount: self.damage.max(0.0),
			cadence: self.cadence,
			falloff: DamageFalloff::None,
		}
	}
}

/// Mouth-emitted constant-size rings plus presentation cues.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrayParams {
	/// PLACEHOLDER head-flick / telegraph hold.
	pub windup: f32,
	/// PLACEHOLDER recovery after the last ring retires.
	pub recovery: f32,
	/// PLACEHOLDER. Shared authored cooldown.
	pub cooldown: f32,
	/// PLACEHOLDER ring radius. [#1119](https://github.com/ramate-io/maybraid/issues/1119) leaves Bray spread/range open.
	pub radius: f32,
	/// PLACEHOLDER travel speed.
	pub speed: f32,
	/// PLACEHOLDER travel distance.
	pub range: f32,
	/// PLACEHOLDER sequential ring count.
	pub waves: u8,
	/// PLACEHOLDER seconds between rings.
	pub interval: f32,
	/// PLACEHOLDER half-thickness. Copies the [`attacks::WavefrontSpec`] default.
	pub half_depth: f32,
	/// PLACEHOLDER extra pitch above the flattened heading, radians.
	pub pitch: f32,
	/// PLACEHOLDER telegraph half-width along the travel heading.
	pub telegraph_half_width: f32,
	/// PLACEHOLDER once-per-wave damage. Copies [`AttackDefaults::AUTHORED`].
	pub damage: f32,
	/// PLACEHOLDER head-flick amplitude, radians.
	pub head_flick_pitch: f32,
	/// PLACEHOLDER delay from Bray execute start to the landing impact cue.
	pub impact_delay: f32,
	/// PLACEHOLDER rumble radius around the mouth.
	pub rumble_radius: f32,
	/// PLACEHOLDER rumble duration.
	pub rumble_secs: f32,
	/// PLACEHOLDER rumble intensity in `0..=1`.
	pub rumble_intensity: f32,
}

impl BrayParams {
	pub const PLACEHOLDER: Self = Self {
		windup: 0.85,
		recovery: 0.8,
		cooldown: AttackDefaults::AUTHORED.cooldown,
		radius: 4.0,
		speed: 10.0,
		range: 28.0,
		waves: 3,
		interval: 0.45,
		half_depth: 0.35,
		pitch: 0.0,
		telegraph_half_width: 0.5,
		damage: AttackDefaults::AUTHORED.damage,
		head_flick_pitch: 0.55,
		impact_delay: 0.12,
		rumble_radius: 24.0,
		rumble_secs: 0.35,
		rumble_intensity: 0.72,
	};

	pub fn execution(self) -> f32 {
		let last = f32::from(self.waves.saturating_sub(1)) * self.interval.max(0.0);
		let travel = if self.speed <= 1e-4 { 0.0 } else { self.range.max(0.0) / self.speed };
		last + travel + 0.05
	}

	pub fn damage_spec(self) -> DamageSpec {
		DamageSpec { amount: self.damage.max(0.0), cadence: 0.0, falloff: DamageFalloff::None }
	}
}

fn flatten(direction: Vec3) -> Vec3 {
	let flat = Vec3::new(direction.x, 0.0, direction.z);
	if flat.length_squared() <= 1e-8 {
		Vec3::NEG_Z
	} else {
		flat.normalize()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn god_beam_active_phase_is_the_agreed_ten_seconds() {
		assert!((GodBeamParams::PLACEHOLDER.execution - 10.0).abs() < 1e-6);
		assert!((GodBeamParams::PLACEHOLDER.column_width - 7.0).abs() < 1e-6);
	}

	#[test]
	fn placeholder_width_as_diameter_is_explicit() {
		let params = GodBeamParams::PLACEHOLDER;
		assert!(params.width_is_diameter);
		assert!((params.column_radius() - 3.5).abs() < 1e-6);
	}

	#[test]
	fn parade_execution_covers_the_last_blast() {
		let params = ParadeParams::PLACEHOLDER;
		let needed = f32::from(params.blast_count - 1) * params.interval + params.blast_execute;
		assert!(params.execution() + 1e-4 >= needed);
	}

	#[test]
	fn bray_execution_covers_the_last_ring() -> anyhow::Result<()> {
		let params = BrayParams::PLACEHOLDER;
		if params.speed <= 1e-4 {
			return Err(anyhow::anyhow!("placeholder speed must be positive"));
		}
		let needed = f32::from(params.waves - 1) * params.interval + params.range / params.speed;
		assert!(params.execution() + 1e-4 >= needed);
		Ok(())
	}
}
