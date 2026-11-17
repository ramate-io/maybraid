use super::*;
use avian3d::prelude::{Collider, PhysicsPlugins, RigidBody};
use bevy::time::TimeUpdateStrategy;
use spotting_intelligence::{
	InterestLayers, SpotBounds, SpotDirective, SpottedContact, SpottingHint, SpottingSettings,
};

pub(super) fn app(discovery: usize, retained: usize) -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		PhysicsPlugins::default(),
		bevy::asset::AssetPlugin::default(),
		bevy::mesh::MeshPlugin,
	));
	app.insert_resource(TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(125)))
		.insert_resource(SpottingObserveLimits {
			max_observers_per_tick: discovery,
			max_retained_observers_per_tick: retained,
			..default()
		})
		.add_plugins(crate::SpottingAvianPlugin);
	app.finish();
	app
}

pub(super) fn subject(app: &mut App, at: Vec3) -> Entity {
	app.world_mut()
		.spawn((
			Transform::from_translation(at),
			SpotSubject::new(InterestLayers::CHARACTER, SpotBounds::capsule(0.4, 0.9)),
		))
		.id()
}

pub(super) fn user() -> SpottingUser {
	let directive = SpotDirective {
		discovery_interval_secs: 0.125,
		respot_interval_secs: 0.0,
		..SpotDirective::new(InterestLayers::CHARACTER, 20.0)
	};
	SpottingUser::new(Vec3::Y * 1.6, [directive]).with_settings(SpottingSettings::new(4, 4, 10.0))
}

pub(super) fn observer(app: &mut App, subject: Option<Entity>, discovery_due: bool) -> Entity {
	let mut user = user();
	user.next_discovery_at = if discovery_due { 0.0 } else { 10_000.0 };
	if let Some(subject) = subject {
		user.contacts.insert(
			subject,
			SpottedContact::new(subject, Vec3::X * 4.0, Vec3::ZERO, Vec3::X * 4.0, None, -0.5, 0.0),
		);
	}
	app.world_mut()
		.spawn((Transform::default(), user, IntelligenceLod::missing()))
		.id()
}

pub(super) fn priority(app: &mut App, entities: &[Entity]) {
	let mut priority = app.world_mut().resource_mut::<IntelligencePriority>();
	for (rank, entity) in entities.iter().enumerate() {
		priority.rank.insert(*entity, rank as u32);
	}
}

pub(super) fn contact(
	app: &App,
	observer: Entity,
	subject: Entity,
) -> anyhow::Result<SpottedContact> {
	app.world()
		.get::<SpottingUser>(observer)
		.and_then(|user| user.contacts.get(&subject))
		.copied()
		.ok_or_else(|| anyhow::anyhow!("missing contact"))
}

#[test]
fn retained_contact_after_discovery_prefix_uses_independent_grant() -> anyhow::Result<()> {
	let mut app = app(1, 1);
	let target = subject(&mut app, Vec3::X * 4.0);
	let mut observers: Vec<_> = (0..8).map(|_| observer(&mut app, None, true)).collect();
	let ninth = observer(&mut app, Some(target), false);
	observers.push(ninth);
	priority(&mut app, &observers);
	app.update();
	assert!(contact(&app, ninth, target)?.last_success_at >= 0.0);
	assert!(
		app.world()
			.get::<SpottingUser>(observers[0])
			.is_some_and(|user| user.next_discovery_at > 0.0)
	);
	// Discovery beyond its one-observer grant must remain deferred.
	assert_eq!(
		app.world().get::<SpottingUser>(observers[1]).map(|user| user.next_discovery_at),
		Some(0.0)
	);
	Ok(())
}

#[test]
fn zero_retained_budget_preserves_deferred_contact_state() -> anyhow::Result<()> {
	let mut app = app(1, 0);
	let target = subject(&mut app, Vec3::X * 4.0);
	let first = observer(&mut app, None, true);
	let deferred = observer(&mut app, Some(target), false);
	priority(&mut app, &[first, deferred]);
	let before = contact(&app, deferred, target)?;
	app.update();
	assert_eq!(contact(&app, deferred, target)?, before);
	Ok(())
}

#[test]
fn fixed_priority_and_shared_skip_resets_do_not_starve_retained_round() -> anyhow::Result<()> {
	let mut app = app(1, 1);
	let target = subject(&mut app, Vec3::X * 4.0);
	let discoverer = observer(&mut app, None, true);
	let waiting: Vec<_> = (0..4).map(|_| observer(&mut app, Some(target), false)).collect();
	let mut order = vec![discoverer];
	order.extend(&waiting);
	priority(&mut app, &order);
	for _ in 0..waiting.len() {
		for entity in &order {
			if let Some(mut lod) = app.world_mut().get_mut::<IntelligenceLod>(*entity) {
				lod.skips = 0;
			}
		}
		app.update();
	}
	for entity in waiting {
		assert!(contact(&app, entity, target)?.last_success_at >= 0.0);
	}
	Ok(())
}

#[test]
fn retained_occlusion_updates_failure_without_fabricating_fresh_sight() -> anyhow::Result<()> {
	let mut app = app(0, 1);
	let target = subject(&mut app, Vec3::X * 4.0);
	app.world_mut().spawn((
		Transform::from_xyz(2.0, 0.0, 0.0),
		RigidBody::Static,
		Collider::cuboid(0.5, 10.0, 10.0),
		PhysicsInteractionLayer::fixed_layers(),
	));
	app.update();
	let observer = observer(&mut app, Some(target), false);
	app.update();
	let contact = contact(&app, observer, target)?;
	assert_eq!(contact.last_success_at, -0.5);
	assert!(contact.last_attempt_at >= 0.0);
	assert_eq!(contact.consecutive_failures, 1);
	assert!(!contact.is_fresh(0.0, 0.2));
	Ok(())
}

#[test]
fn hints_without_contacts_never_enter_retained_executor() {
	let mut app = app(0, 8);
	let target = subject(&mut app, Vec3::X * 4.0);
	let observer = observer(&mut app, None, false);
	if let Some(mut user) = app.world_mut().get_mut::<SpottingUser>(observer) {
		user.hint(target, SpottingHint::new(4));
	}
	app.update();
	assert!(
		app.world()
			.get::<SpottingUser>(observer)
			.is_some_and(|user| user.contacts.is_empty())
	);
}

#[test]
fn retained_observer_and_sample_caps_leave_unprobed_contacts_unchanged() -> anyhow::Result<()> {
	let mut app = app(0, 1);
	let targets: Vec<_> = (1..=3).map(|n| subject(&mut app, Vec3::X * (n as f32 * 4.0))).collect();
	let first = observer(&mut app, Some(targets[0]), false);
	let second = observer(&mut app, Some(targets[0]), false);
	if let Some(mut user) = app.world_mut().get_mut::<SpottingUser>(first) {
		user.settings = SpottingSettings::new(3, 1, 10.0);
		for target in &targets[1..] {
			user.contacts.insert(
				*target,
				SpottedContact::new(*target, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, None, -0.5, 0.0),
			);
		}
	}
	priority(&mut app, &[first, second]);
	let second_before = contact(&app, second, targets[0])?;
	app.update();
	let attempted = targets
		.iter()
		.filter(|target| {
			contact(&app, first, **target).is_ok_and(|contact| contact.last_attempt_at >= 0.0)
		})
		.count();
	assert_eq!(attempted, 1);
	assert_eq!(contact(&app, second, targets[0])?, second_before);
	Ok(())
}

#[test]
fn stale_despawned_and_not_due_queue_entries_do_not_spend_grants() -> anyhow::Result<()> {
	let mut app = app(0, 1);
	let target = subject(&mut app, Vec3::X * 4.0);
	let first = observer(&mut app, Some(target), false);
	let removed = observer(&mut app, Some(target), false);
	let not_due = observer(&mut app, Some(target), false);
	let last = observer(&mut app, Some(target), false);
	priority(&mut app, &[first, removed, not_due, last]);
	app.update();
	app.world_mut().entity_mut(removed).despawn();
	if let Some(mut user) = app.world_mut().get_mut::<SpottingUser>(not_due) {
		if let Some(contact) = user.contacts.get_mut(&target) {
			contact.next_respot_at = 1000.0;
		}
	}
	app.update();
	assert!(contact(&app, last, target)?.last_success_at >= 0.0);
	Ok(())
}

#[test]
fn discovery_serviced_observer_is_not_probed_twice() -> anyhow::Result<()> {
	let mut app = app(1, 1);
	let target = subject(&mut app, Vec3::X * 4.0);
	app.world_mut().spawn((
		Transform::from_xyz(2.0, 0.0, 0.0),
		RigidBody::Static,
		Collider::cuboid(0.5, 10.0, 10.0),
		PhysicsInteractionLayer::fixed_layers(),
	));
	app.update();
	let first = observer(&mut app, Some(target), true);
	let retained = observer(&mut app, Some(target), false);
	priority(&mut app, &[first, retained]);
	app.update();
	assert_eq!(contact(&app, first, target)?.consecutive_failures, 1);
	assert_eq!(contact(&app, retained, target)?.consecutive_failures, 1);
	Ok(())
}

#[test]
fn cleanup_removes_invalid_contacts_even_with_both_budgets_zero() {
	let mut app = app(0, 0);
	let target = subject(&mut app, Vec3::X * 4.0);
	let observer = observer(&mut app, Some(target), false);
	app.world_mut().entity_mut(target).despawn();
	app.update();
	assert!(
		app.world()
			.get::<SpottingUser>(observer)
			.is_some_and(|user| user.contacts.is_empty())
	);
}

#[test]
fn contact_only_grant_does_not_probe_unknown_hints() -> anyhow::Result<()> {
	let mut app = app(0, 1);
	let known = subject(&mut app, Vec3::X * 4.0);
	let unknown = subject(&mut app, Vec3::X * 8.0);
	let observer = observer(&mut app, Some(known), false);
	if let Some(mut user) = app.world_mut().get_mut::<SpottingUser>(observer) {
		user.hint(unknown, SpottingHint::new(4));
	}
	app.update();
	assert!(contact(&app, observer, known)?.last_success_at >= 0.0);
	assert!(
		app.world()
			.get::<SpottingUser>(observer)
			.is_some_and(|user| !user.contacts.contains_key(&unknown))
	);
	Ok(())
}

#[test]
fn zero_visibility_samples_preserves_contact_observation_state() -> anyhow::Result<()> {
	let mut app = app(0, 1);
	let target = subject(&mut app, Vec3::X * 4.0);
	let observer = observer(&mut app, Some(target), false);
	if let Some(mut user) = app.world_mut().get_mut::<SpottingUser>(observer) {
		user.settings.vision_samples = 0;
		if let Some(contact) = user.contacts.get_mut(&target) {
			contact.consecutive_failures = 3;
			contact.last_attempt_at = -0.25;
		}
	}
	let before = contact(&app, observer, target)?;
	app.update();
	assert_eq!(contact(&app, observer, target)?, before);
	Ok(())
}

#[test]
fn both_zero_budgets_forget_expired_contact_with_live_subject() {
	let mut app = app(0, 0);
	let target = subject(&mut app, Vec3::X * 4.0);
	let observer = observer(&mut app, Some(target), false);
	if let Some(mut user) = app.world_mut().get_mut::<SpottingUser>(observer) {
		user.settings.memory_secs = 0.1;
	}
	app.update();
	assert!(app.world().get::<SpotSubject>(target).is_some());
	assert!(
		app.world()
			.get::<SpottingUser>(observer)
			.is_some_and(|user| user.contacts.is_empty())
	);
}

#[test]
fn retained_refresh_rechecks_visible_targets_at_close_combat_distances() -> anyhow::Result<()> {
	let mut app = app(0, 3);
	let mut pairs = Vec::new();
	for distance in [5.0, 15.0, 30.0] {
		let target = subject(&mut app, Vec3::X * distance);
		let observer = observer(&mut app, Some(target), false);
		if let Some(mut user) = app.world_mut().get_mut::<SpottingUser>(observer) {
			user.directives[0].range = 40.0;
		}
		pairs.push((observer, target, distance));
	}
	app.update();
	for (observer, target, distance) in pairs {
		let seen = contact(&app, observer, target)?;
		assert!(seen.last_success_at >= 0.0);
		assert_eq!(seen.position, Vec3::X * distance);
	}
	Ok(())
}

#[test]
fn close_cover_can_leave_eye_visibility_clear_while_blocking_muzzle_segment() -> anyhow::Result<()>
{
	use bevy::ecs::system::RunSystemOnce;
	let mut app = app(0, 0);
	app.world_mut().spawn((
		Transform::from_xyz(2.5, 0.5, 0.0),
		RigidBody::Static,
		Collider::cuboid(0.2, 1.0, 1.0),
		PhysicsInteractionLayer::fixed_layers(),
	));
	app.update();
	app.update();
	let (eye_clear, muzzle_clear) = app
		.world_mut()
		.run_system_once(|spatial: SpatialQuery| {
			let target = Vec3::new(5.0, 1.0, 0.0);
			let filter = PhysicsInteractionLayer::Fixed.query_filter();
			(
				crate::clear_segment(Vec3::Y * 1.6, target, &spatial, &filter),
				crate::clear_segment(Vec3::Y * 0.2, target, &spatial, &filter),
			)
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	assert!(eye_clear);
	assert!(!muzzle_clear);
	Ok(())
}
