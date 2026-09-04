use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use spotting_intelligence::SpottingUser;

use crate::{
	AffiliationStrength, Affiliations, ThreatDiscoveryPolicy, ThreatGroupId, ThreatId,
	ThreatIntelligencePlugin, ThreatIntelligenceUser, ThreatKnowledge, ThreatRecord,
	ThreatRegistry, ThreatSource, ThreatSubject,
};

const FFA: ThreatGroupId = ThreatGroupId::group(1);

fn ffa_affiliations(id: ThreatId) -> Affiliations {
	let mut affiliations = Affiliations::with_self(id);
	affiliations.join(FFA, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(FFA, AffiliationStrength::permanent(1.0));
	affiliations
}

#[test]
fn shared_ffa_group_classifies_another_member_as_a_threat() -> anyhow::Result<()> {
	let mut world = World::new();
	let entity = world.spawn_empty().id();
	let recipient = ffa_affiliations(ThreatId(1));
	let record = ThreatRecord {
		id: ThreatId(2),
		entity,
		position: Vec3::X,
		salience: 1.0,
		affiliations: ffa_affiliations(ThreatId(2)),
	};
	let mut knowledge = ThreatKnowledge::default();
	assert!(knowledge
		.observe(&record, &recipient, ThreatSource::LOCAL_SCAN, 1.0, 0.0, 0.2)
		.is_some());
	assert_eq!(knowledge.len(), 1);
	Ok(())
}

#[test]
fn decayed_antagonism_reclassifies_retained_threats() -> anyhow::Result<()> {
	let mut world = World::new();
	let entity = world.spawn_empty().id();
	let mut recipient = Affiliations::with_self(ThreatId(1));
	recipient.antagonize(FFA, AffiliationStrength::decaying(1.0, 0.0, 1.0));
	let mut subject = Affiliations::with_self(ThreatId(2));
	subject.join(FFA, AffiliationStrength::permanent(1.0));
	let record = ThreatRecord {
		id: ThreatId(2),
		entity,
		position: Vec3::X,
		salience: 1.0,
		affiliations: subject,
	};
	let mut knowledge = ThreatKnowledge::default();
	knowledge.observe(&record, &recipient, ThreatSource::LOCAL_SCAN, 1.0, 0.0, 0.2);
	knowledge.maintain(
		&recipient,
		ThreatDiscoveryPolicy { threat_threshold: 0.2, ..default() },
		4.0,
	);
	assert!(knowledge.is_empty());
	Ok(())
}

#[test]
fn source_removal_does_not_erase_another_reason() -> anyhow::Result<()> {
	let mut world = World::new();
	let entity = world.spawn_empty().id();
	let recipient = ffa_affiliations(ThreatId(1));
	let record = ThreatRecord {
		id: ThreatId(2),
		entity,
		position: Vec3::X,
		salience: 1.0,
		affiliations: ffa_affiliations(ThreatId(2)),
	};
	let mut knowledge = ThreatKnowledge::default();
	knowledge.observe(&record, &recipient, ThreatSource::LOCAL_SCAN, 1.0, 0.0, 0.2);
	knowledge.observe(&record, &recipient, ThreatSource::SESSION, 1.0, 0.0, 0.2);
	assert!(knowledge.remove_source(record.id, ThreatSource::LOCAL_SCAN));
	assert!(knowledge.get(record.id).is_some());
	Ok(())
}

#[test]
fn registry_returns_only_nearby_subjects() -> anyhow::Result<()> {
	let mut world = World::new();
	let near = world.spawn_empty().id();
	let far = world.spawn_empty().id();
	let mut registry = ThreatRegistry::default();
	registry.upsert(
		near,
		ThreatSubject::new(ThreatId(1)),
		&ffa_affiliations(ThreatId(1)),
		Vec3::X * 10.0,
	)?;
	registry.upsert(
		far,
		ThreatSubject::new(ThreatId(2)),
		&ffa_affiliations(ThreatId(2)),
		Vec3::X * 100.0,
	)?;
	let nearby = registry.local(Vec3::ZERO, 20.0);
	assert_eq!(nearby.len(), 1);
	assert_eq!(nearby[0].entity, near);
	Ok(())
}

#[test]
fn discovery_excludes_self_but_learns_another_ffa_member() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, ThreatIntelligencePlugin));
	let observer_id = ThreatId(1);
	let other_id = ThreatId(2);
	let observer = app
		.world_mut()
		.spawn((
			ThreatSubject::new(observer_id),
			ffa_affiliations(observer_id),
			ThreatIntelligenceUser::default(),
			ThreatKnowledge::default(),
			GlobalTransform::default(),
		))
		.id();
	app.world_mut().spawn((
		ThreatSubject::new(other_id),
		ffa_affiliations(other_id),
		GlobalTransform::from_translation(Vec3::X),
	));
	app.update();
	let knowledge = app.world().get::<ThreatKnowledge>(observer);
	assert!(knowledge.is_some_and(|knowledge| {
		knowledge.get(observer_id).is_none() && knowledge.get(other_id).is_some()
	}));
	Ok(())
}

#[test]
fn exported_threat_hint_is_removed_with_knowledge() -> Result<(), bevy::ecs::system::RunSystemError>
{
	let mut world = World::new();
	let threat = world.spawn_empty().id();
	let record = ThreatRecord {
		id: ThreatId(2),
		entity: threat,
		position: Vec3::X,
		salience: 1.0,
		affiliations: ffa_affiliations(ThreatId(2)),
	};
	let recipient = ffa_affiliations(ThreatId(1));
	let mut knowledge = ThreatKnowledge::default();
	knowledge.observe(&record, &recipient, ThreatSource::LOCAL_SCAN, 1.0, 0.0, 0.2);
	let user = world.spawn((knowledge, SpottingUser::default())).id();
	world.run_system_once(crate::export_threat_spotting_hints)?;
	assert!(world
		.get::<SpottingUser>(user)
		.is_some_and(|spotting| spotting.hints.contains_key(&threat)));
	world
		.get_mut::<ThreatKnowledge>(user)
		.map(|mut knowledge| knowledge.remove_source(record.id, ThreatSource::LOCAL_SCAN));
	world.run_system_once(crate::export_threat_spotting_hints)?;
	assert!(world
		.get::<SpottingUser>(user)
		.is_some_and(|spotting| !spotting.hints.contains_key(&threat)));
	Ok(())
}
