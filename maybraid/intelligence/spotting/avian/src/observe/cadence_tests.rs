use super::retained_tests::{app, contact, observer, priority, subject};
use super::*;
use bevy::time::{TimeUpdateStrategy, common_conditions::on_timer};
use spotting_intelligence::{SpottedContact, SpottingSettings};
use std::time::Duration;

#[test]
fn retained_contacts_refresh_between_application_discovery_ticks() -> anyhow::Result<()> {
	let mut app = app(1, 1);
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(25)))
		.configure_sets(
			Update,
			spotting_intelligence::SpottingSystems::Discover
				.run_if(on_timer(Duration::from_millis(125))),
		);
	let target = subject(&mut app, Vec3::X * 4.0);
	let actor = observer(&mut app, Some(target), true);
	for _ in 0..6 {
		app.update();
	}
	let discovery_at = app
		.world()
		.get::<SpottingUser>(actor)
		.ok_or_else(|| anyhow::anyhow!("missing observer"))?
		.next_discovery_at;
	assert!(discovery_at > 0.125);
	assert_eq!(app.world().resource::<SpottingFrameState>().discovery_observers, 1);
	for _ in 0..3 {
		app.update();
		let now = app.world().resource::<Time>().elapsed_secs();
		assert_eq!(contact(&app, actor, target)?.last_success_at, now);
		assert_eq!(app.world().resource::<SpottingFrameState>().discovery_observers, 0);
		assert_eq!(app.world().resource::<SpottingFrameState>().retained_observers, 1);
		assert_eq!(
			app.world().get::<SpottingUser>(actor).map(|user| user.next_discovery_at),
			Some(discovery_at)
		);
	}
	Ok(())
}

#[test]
fn phase_sample_and_observer_caps_hold_for_large_vision_and_variable_delta() -> anyhow::Result<()> {
	let mut app = app(3, 3);
	app.insert_resource(SpottingObserveLimits {
		max_observers_per_tick: 3,
		max_retained_observers_per_tick: 3,
		max_discovery_samples_per_tick: 5,
		max_retained_samples_per_tick: 7,
	});
	let first = subject(&mut app, Vec3::X * 4.0);
	let second = subject(&mut app, Vec3::X * 8.0);
	let mut actors = Vec::new();
	for _ in 0..10 {
		let actor = observer(&mut app, Some(first), true);
		let mut user = app
			.world_mut()
			.get_mut::<SpottingUser>(actor)
			.ok_or_else(|| anyhow::anyhow!("missing observer"))?;
		user.settings = SpottingSettings::new(1000, 10000, 10.0);
		user.directives[0].max_samples_per_subject = 9;
		user.contacts.insert(
			second,
			SpottedContact::new(second, Vec3::X * 8.0, Vec3::ZERO, Vec3::X * 8.0, None, -0.5, 0.0),
		);
		actors.push(actor);
	}
	priority(&mut app, &actors);
	for delta in [0, 16, 300, 1000, 0, 16] {
		app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(delta)));
		app.update();
		let frame = app.world().resource::<SpottingFrameState>();
		assert!(frame.discovery_observers <= 3 && frame.retained_observers <= 3);
		assert!(frame.discovery_samples <= 5 && frame.retained_samples <= 7);
		assert!(frame.discovery_samples + frame.retained_samples <= 12);
		// The enormous observer setting cannot accumulate catch-up service.
		assert!(frame.observed.len() <= 6);
	}
	Ok(())
}

#[test]
fn exhausted_retained_samples_preserve_the_fair_queue_head() -> anyhow::Result<()> {
	let mut app = app(0, 8);
	app.insert_resource(SpottingObserveLimits {
		max_observers_per_tick: 0,
		max_retained_observers_per_tick: 8,
		max_discovery_samples_per_tick: 0,
		max_retained_samples_per_tick: 1,
	});
	let target = subject(&mut app, Vec3::X * 4.0);
	let first = observer(&mut app, Some(target), false);
	let second = observer(&mut app, Some(target), false);
	let third = observer(&mut app, Some(target), false);
	priority(&mut app, &[first, second, third]);
	app.update();
	assert!(contact(&app, first, target)?.last_success_at >= 0.0);
	let second_before = contact(&app, second, target)?;
	app.world_mut()
		.resource_mut::<SpottingObserveLimits>()
		.max_retained_samples_per_tick = 0;
	app.update();
	assert_eq!(contact(&app, second, target)?, second_before);
	app.world_mut()
		.resource_mut::<SpottingObserveLimits>()
		.max_retained_samples_per_tick = 1;
	app.update();
	assert_eq!(
		contact(&app, second, target)?.last_success_at,
		app.world().resource::<Time>().elapsed_secs()
	);
	assert_eq!(contact(&app, third, target)?.last_success_at, -0.5);
	assert_eq!(app.world().resource::<SpottingFrameState>().retained_observers, 1);
	assert_eq!(app.world().resource::<SpottingFrameState>().retained_samples, 1);
	Ok(())
}

#[test]
fn zero_sample_or_observer_credit_preserves_contact_and_discovery_cursor() -> anyhow::Result<()> {
	for samples_zero in [true, false] {
		let mut app = app(1, 1);
		app.insert_resource(SpottingObserveLimits {
			max_observers_per_tick: usize::from(samples_zero),
			max_retained_observers_per_tick: usize::from(samples_zero),
			max_discovery_samples_per_tick: if samples_zero { 0 } else { 80 },
			max_retained_samples_per_tick: if samples_zero { 0 } else { 80 },
		});
		let target = subject(&mut app, Vec3::X * 4.0);
		let actor = observer(&mut app, Some(target), true);
		let before = app
			.world()
			.get::<SpottingUser>(actor)
			.ok_or_else(|| anyhow::anyhow!("missing observer"))?
			.clone();
		app.update();
		assert_eq!(app.world().get::<SpottingUser>(actor), Some(&before));
		assert_eq!(app.world().resource::<SpottingFrameState>().discovery_samples, 0);
		assert_eq!(app.world().resource::<SpottingFrameState>().retained_samples, 0);
	}
	Ok(())
}

#[test]
fn parent_observe_gate_still_gates_prepare_discovery_and_refresh() -> anyhow::Result<()> {
	let mut app = app(1, 1);
	app.configure_sets(Update, spotting_intelligence::SpottingSystems::Observe.run_if(|| false));
	let target = subject(&mut app, Vec3::X * 4.0);
	let actor = observer(&mut app, Some(target), true);
	let before = app
		.world()
		.get::<SpottingUser>(actor)
		.ok_or_else(|| anyhow::anyhow!("missing observer"))?
		.clone();
	app.update();
	assert_eq!(app.world().get::<SpottingUser>(actor), Some(&before));
	assert!(app.world().resource::<SpottingFrameState>().observed.is_empty());
	Ok(())
}

#[test]
fn discovery_serviced_contact_is_not_probed_again_by_refresh() -> anyhow::Result<()> {
	use avian3d::prelude::{Collider, RigidBody};
	use lod_avian::PhysicsInteractionLayer;

	let mut app = app(1, 1);
	let target = subject(&mut app, Vec3::X * 4.0);
	app.world_mut().spawn((
		Transform::from_xyz(2.0, 0.0, 0.0),
		RigidBody::Static,
		Collider::cuboid(0.5, 10.0, 10.0),
		PhysicsInteractionLayer::fixed_layers(),
	));
	app.update();
	let actor = observer(&mut app, Some(target), true);
	app.update();
	let sight = contact(&app, actor, target)?;
	assert_eq!(sight.consecutive_failures, 1);
	assert_eq!(sight.last_success_at, -0.5);
	let frame = app.world().resource::<SpottingFrameState>();
	assert_eq!(frame.observed.len(), 1);
	assert_eq!(frame.discovery_observers, 1);
	assert_eq!(frame.retained_observers, 0);
	Ok(())
}

#[test]
fn zero_observer_vision_does_not_advance_discovery_sample_cursor() -> anyhow::Result<()> {
	let mut app = app(1, 1);
	let target = subject(&mut app, Vec3::X * 4.0);
	let actor = observer(&mut app, Some(target), true);
	if let Some(mut user) = app.world_mut().get_mut::<SpottingUser>(actor) {
		user.settings.vision_samples = 0;
	}
	let before = app
		.world()
		.get::<SpottingUser>(actor)
		.ok_or_else(|| anyhow::anyhow!("missing observer"))?
		.clone();
	app.update();
	let after = app
		.world()
		.get::<SpottingUser>(actor)
		.ok_or_else(|| anyhow::anyhow!("missing observer"))?;
	assert_eq!(after.contacts, before.contacts);
	assert_eq!(after.clone().advance_sample_cursor(), before.clone().advance_sample_cursor());
	assert_eq!(app.world().resource::<SpottingFrameState>().discovery_samples, 0);
	Ok(())
}
