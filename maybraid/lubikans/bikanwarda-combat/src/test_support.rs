use std::time::Duration;

use attacks::AttackUser;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bikanwarda_attacks::{BikanwardaAttackParams, ParadeParams};
use combat_targeting::{CombatContact, CombatTargeting, TargetSource};
use lubikans::{
	LubikanAccountId, LubikanActivationSessionId, LubikanOwner, LubikanSummon,
	LubikanSummonInstanceId,
};

use crate::plugin::attach_summon_policy;
use crate::{
	BikanwardaLubikanPlugin, BikanwardaSummonConfig, BikanwardaSummonForm, BikanwardaSummonPolicy,
};

pub fn short_params() -> BikanwardaAttackParams {
	let mut params = BikanwardaAttackParams::PLACEHOLDER;
	params.parade = ParadeParams {
		windup: 0.2,
		recovery: 0.1,
		cooldown: 0.0,
		blast_count: 3,
		interval: 0.25,
		first_distance: 2.0,
		spacing: 2.0,
		blast_radius: 1.1,
		blast_execute: 0.15,
		line_half_width: 0.4,
		blast_height: 1.0,
		blast_damage: 20.0,
		blast_falloff: None,
	};
	params.god_beam.windup = 0.2;
	params.god_beam.recovery = 0.1;
	params.god_beam.cooldown = 0.0;
	params.god_beam.column_height = 8.0;
	params.bray.windup = 0.2;
	params.bray.recovery = 0.1;
	params.bray.cooldown = 0.0;
	params
}

pub fn policy_app(auto_cast: bool) -> App {
	let mut app = App::new();
	app.insert_resource(short_params());
	app.insert_resource(BikanwardaSummonConfig { auto_cast });
	app.add_plugins((MinimalPlugins, TransformPlugin, BikanwardaLubikanPlugin));
	app.finish();
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(0.05)));
	app.world_mut()
		.resource_mut::<Time<Virtual>>()
		.set_max_delta(Duration::from_secs(30));
	app.update();
	app
}

pub fn step(app: &mut App, dt: f32) {
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(dt)));
	app.update();
}

pub fn spawn_owner(app: &mut App, translation: Vec3) -> Entity {
	app.world_mut()
		.spawn((
			LubikanOwner,
			Transform::from_translation(translation),
			GlobalTransform::from_translation(translation),
			CombatTargeting::default(),
		))
		.id()
}

pub fn spawn_summon(
	app: &mut App,
	owner: Entity,
	form: BikanwardaSummonForm,
	translation: Vec3,
) -> Entity {
	let definition = form.definition_id();
	let summon = app
		.world_mut()
		.spawn((
			LubikanSummon {
				owner,
				account: LubikanAccountId::unique(definition),
				definition,
				session: LubikanActivationSessionId::from_raw(1),
				instance: LubikanSummonInstanceId::from_raw(1),
				elapsed: Duration::ZERO,
				lifetime: Some(Duration::from_secs(20)),
				spent: -25.0,
			},
			Transform::from_translation(translation),
			GlobalTransform::from_translation(translation),
		))
		.id();
	let _ = app.world_mut().run_system_once(attach_summon_policy);
	summon
}

pub fn observe(
	app: &mut App,
	observer: Entity,
	subject: Entity,
	point: Vec3,
	source: TargetSource,
) {
	let now = app.world().resource::<Time>().elapsed_secs();
	let mut targeting = app.world().get::<CombatTargeting>(observer).cloned().unwrap_or_default();
	targeting.upsert_contact(CombatContact {
		subject,
		position: point,
		movement_vector: Vec3::ZERO,
		visible_point: point,
		visible_head: None,
		last_spotted_at: now,
	});
	if !source.is_empty() && source != TargetSource::SPOTTING {
		targeting.include(subject, source);
	}
	targeting.rebalance(now);
	app.world_mut().entity_mut(observer).insert(targeting);
}

pub fn policy_of(app: &App, summon: Entity) -> Option<BikanwardaSummonPolicy> {
	app.world().get::<BikanwardaSummonPolicy>(summon).cloned()
}

pub fn has_attack_user(app: &App, summon: Entity) -> bool {
	app.world().get::<AttackUser>(summon).is_some()
}
