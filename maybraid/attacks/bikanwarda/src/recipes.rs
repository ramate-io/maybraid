//! [`AttackDefinition`] values built from [`crate::BikanwardaAttackParams`].
//!
//! These reuse the [#1147](https://github.com/ramate-io/maybraid/pull/1147)
//! contract. They do not add hazard shapes or aim kinds.

use attacks::{
	AimPolicy, AttackDefinition, AttackTiming, FriendlyFire, HazardOrigin, HazardShape, HazardSpec,
	InterruptionPolicy, TelegraphSpec, WavefrontSpec,
};

use crate::{BikanwardaAttackParams, BRAY, GOD_BEAM, PARADE, PARADE_BLAST};

/// Insert Parade, Parade blast, God Beam, and Bray into a catalog.
pub fn register_bikanwarda_attacks(
	catalog: &mut attacks::AttackCatalog,
	params: &BikanwardaAttackParams,
) {
	catalog
		.insert(parade_definition(params))
		.insert(parade_blast_definition(params))
		.insert(god_beam_definition(params))
		.insert(bray_definition(params));
}

pub fn parade_definition(params: &BikanwardaAttackParams) -> AttackDefinition {
	let parade = params.parade;
	AttackDefinition::new(
		PARADE,
		AttackTiming::new(parade.windup, parade.execution(), parade.recovery, parade.cooldown),
		AimPolicy::TrackUntilExecute,
		InterruptionPolicy::AUTHORED,
		FriendlyFire::ExcludeOwnerAndAllies,
	)
	.with_telegraph(TelegraphSpec {
		shape: HazardShape::Line {
			length: parade.line_length(),
			half_width: parade.line_half_width,
		},
		origin: HazardOrigin::Owner,
		persist_into_execute: true,
	})
}

pub fn parade_blast_definition(params: &BikanwardaAttackParams) -> AttackDefinition {
	let parade = params.parade;
	AttackDefinition::new(
		PARADE_BLAST,
		AttackTiming::new(0.0, parade.blast_execute, 0.0, 0.0),
		AimPolicy::OwnerPoseAtWindup,
		InterruptionPolicy::AUTHORED,
		FriendlyFire::ExcludeOwnerAndAllies,
	)
	.with_concurrent(true)
	.with_hazard(HazardSpec {
		shape: HazardShape::Sphere { radius: parade.blast_radius },
		origin: HazardOrigin::Emission,
		damage: parade.blast_damage_spec(),
	})
}

pub fn god_beam_definition(params: &BikanwardaAttackParams) -> AttackDefinition {
	let beam = params.god_beam;
	AttackDefinition::new(
		GOD_BEAM,
		AttackTiming::new(beam.windup, beam.execution, beam.recovery, beam.cooldown),
		// PLACEHOLDER tracking: lock at windup. [#1119] leaves lock vs follow open.
		AimPolicy::TargetAtWindup,
		InterruptionPolicy::AUTHORED,
		FriendlyFire::ExcludeOwnerAndAllies,
	)
	.with_telegraph(TelegraphSpec {
		shape: HazardShape::Sphere { radius: beam.warning_radius() },
		origin: HazardOrigin::AimPoint,
		persist_into_execute: false,
	})
	.with_hazard(HazardSpec {
		shape: HazardShape::Cylinder { radius: beam.column_radius(), height: beam.column_height },
		origin: HazardOrigin::AimPoint,
		damage: beam.damage_spec(),
	})
}

pub fn bray_definition(params: &BikanwardaAttackParams) -> AttackDefinition {
	let bray = params.bray;
	AttackDefinition::new(
		BRAY,
		AttackTiming::new(bray.windup, bray.execution(), bray.recovery, bray.cooldown),
		AimPolicy::TrackUntilExecute,
		InterruptionPolicy::AUTHORED,
		FriendlyFire::ExcludeOwnerAndAllies,
	)
	.with_telegraph(TelegraphSpec {
		shape: HazardShape::Line { length: bray.range, half_width: bray.telegraph_half_width },
		origin: HazardOrigin::Emission,
		persist_into_execute: true,
	})
	.with_hazard(HazardSpec {
		shape: HazardShape::Wavefront(
			WavefrontSpec::new(bray.radius, bray.speed, bray.range)
				.with_waves(bray.waves, bray.interval)
				.with_half_depth(bray.half_depth)
				.with_pitch(bray.pitch),
		),
		origin: HazardOrigin::Emission,
		damage: bray.damage_spec(),
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use attacks::{AttackDefaults, HazardOrigin, HazardShape};

	#[test]
	fn recipes_copy_authored_policy() {
		let params = BikanwardaAttackParams::PLACEHOLDER;
		for definition in
			[parade_definition(&params), god_beam_definition(&params), bray_definition(&params)]
		{
			assert_eq!(definition.interruption, InterruptionPolicy::AUTHORED);
			assert_eq!(definition.friendly_fire, FriendlyFire::ExcludeOwnerAndAllies);
		}
		assert_eq!(god_beam_definition(&params).timing.cooldown, AttackDefaults::AUTHORED.cooldown);
	}

	#[test]
	fn god_beam_uses_aim_point_column() -> anyhow::Result<()> {
		let definition = god_beam_definition(&BikanwardaAttackParams::PLACEHOLDER);
		assert_eq!(definition.aim, AimPolicy::TargetAtWindup);
		assert!((definition.timing.execution - 10.0).abs() < 1e-6);
		let hazard = definition
			.hazards
			.first()
			.ok_or_else(|| anyhow::anyhow!("god beam must author a column hazard"))?;
		assert_eq!(hazard.origin, HazardOrigin::AimPoint);
		let HazardShape::Cylinder { radius, height } = hazard.shape else {
			return Err(anyhow::anyhow!("expected cylinder, got {:?}", hazard.shape));
		};
		assert!((radius - 3.5).abs() < 1e-6);
		assert!((height - 24.0).abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn bray_is_emission_wavefront() -> anyhow::Result<()> {
		let params = BikanwardaAttackParams::PLACEHOLDER;
		let definition = bray_definition(&params);
		let hazard = definition
			.hazards
			.first()
			.ok_or_else(|| anyhow::anyhow!("bray must author a wavefront"))?;
		assert_eq!(hazard.origin, HazardOrigin::Emission);
		let HazardShape::Wavefront(wave) = hazard.shape else {
			return Err(anyhow::anyhow!("expected wavefront, got {:?}", hazard.shape));
		};
		assert_eq!(wave.waves, params.bray.waves);
		assert!((wave.pitch - params.bray.pitch).abs() < 1e-6);
		assert!((wave.radius - params.bray.radius).abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn parade_parent_has_no_gameplay_hazards() {
		let definition = parade_definition(&BikanwardaAttackParams::PLACEHOLDER);
		assert!(definition.hazards.is_empty());
		assert!(definition.telegraph.is_some());
		assert!(parade_blast_definition(&BikanwardaAttackParams::PLACEHOLDER).concurrent);
	}
}
