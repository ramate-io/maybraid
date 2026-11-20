//! Bounded provenance for slow calls in the existing exclusive retirement drain.

use std::time::Instant;

use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::{Children, Commands, Component, Entity, Resource, Vec3, World};
use bevy::shape::Aabb3d;
use lod_generation_diagnostics::retired_host::{RetiredHostCompletion, RetiredHostSamples};

#[derive(Component)]
#[component(storage = "SparseSet")]
pub(super) struct RetiredHostOrigin {
	scheme: &'static str,
	reason: &'static str,
	successor: Option<Entity>,
	last_focus_xz_distance_m: Option<f32>,
	queued: Instant,
}

pub(super) fn stamp<T>(
	commands: &mut Commands,
	entity: Entity,
	reason: &'static str,
	successor: Option<Entity>,
	focus: Option<Vec3>,
	bounds: Option<Aabb3d>,
) {
	if !lod_generation_diagnostics::retired_host::enabled() {
		return;
	}
	// Focus changes only with a bounds publication. It is not a current player position.
	let last_focus_xz_distance_m = focus.zip(bounds).map(|(focus, bounds)| {
		let closest = focus.clamp(Vec3::from(bounds.min), Vec3::from(bounds.max));
		(focus.x - closest.x).hypot(focus.z - closest.z)
	});
	commands.entity(entity).try_insert(RetiredHostOrigin {
		scheme: std::any::type_name::<T>(),
		reason,
		successor,
		last_focus_xz_distance_m,
		queued: Instant::now(),
	});
}

#[derive(Default, Resource)]
pub(in crate::hcsg) struct RetirementProbe(RetiredHostSamples);

pub(super) struct PendingSample {
	root: Entity,
	scheme: &'static str,
	reason: &'static str,
	last_focus_xz_distance_m: Option<f32>,
	successor: Option<(u64, Option<bool>)>,
	queued_for: Option<std::time::Duration>,
	direct_children: usize,
	root_disabled: bool,
	live_before: u32,
	started: Instant,
}

impl PendingSample {
	pub(super) fn begin(world: &World, root: Entity) -> Option<Self> {
		let budget = world.get_resource::<RetirementProbe>()?;
		let origin = world.get::<RetiredHostOrigin>(root);
		let reason = origin.map_or("unknown", |origin| origin.reason);
		if !budget.0.wants(reason) {
			return None;
		}
		Some(Self {
			root,
			scheme: origin.map_or("unknown", |origin| origin.scheme),
			reason,
			last_focus_xz_distance_m: origin.and_then(|origin| origin.last_focus_xz_distance_m),
			successor: origin.and_then(|origin| origin.successor).map(|entity| {
				(
					entity.to_bits(),
					world
						.get_entity(entity)
						.ok()
						.map(|successor| successor.contains::<crate::LodSceneHostStreamed>()),
				)
			}),
			queued_for: origin.map(|origin| origin.queued.elapsed()),
			direct_children: world.get::<Children>(root).map_or(0, |children| children.len()),
			root_disabled: world.get::<Disabled>(root).is_some(),
			live_before: world.entities().len(),
			started: Instant::now(),
		})
	}

	pub(super) fn finish(self, world: &mut World) {
		let elapsed = self.started.elapsed();
		let live_after = world.entities().len();
		if let Some(mut probe) = world.get_resource_mut::<RetirementProbe>() {
			probe.0.record(RetiredHostCompletion {
				root_bits: self.root.to_bits(),
				scheme: self.scheme,
				reason: self.reason,
				last_focus_xz_distance_m: self.last_focus_xz_distance_m,
				successor: self.successor,
				queued_for: self.queued_for,
				direct_children: self.direct_children,
				root_disabled: self.root_disabled,
				live_before: self.live_before,
				live_after,
				elapsed,
			});
		}
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Context as _;

	use super::*;
	use crate::hcsg::presentation::despawn_retired_hosts;
	use crate::hcsg::RetiredHost;
	use bevy::prelude::ChildOf;

	#[test]
	fn probe_copies_scheme_and_root_metadata_without_changing_teardown() -> anyhow::Result<()> {
		struct FixtureScheme;

		let mut world = World::new();
		let successor = world.spawn_empty().id();
		let root = world
			.spawn((
				RetiredHost,
				RetiredHostOrigin {
					scheme: std::any::type_name::<FixtureScheme>(),
					reason: "version",
					successor: Some(successor),
					last_focus_xz_distance_m: None,
					queued: Instant::now(),
				},
			))
			.id();
		let child = world.spawn(ChildOf(root)).id();
		assert!(PendingSample::begin(&world, root).is_none());
		world.init_resource::<RetirementProbe>();
		let probe = PendingSample::begin(&world, root).context("marked root should be sampled")?;
		assert_eq!(probe.scheme, std::any::type_name::<FixtureScheme>());
		assert_eq!(probe.reason, "version");
		assert_eq!(probe.successor, Some((successor.to_bits(), Some(false))));
		world.entity_mut(successor).insert(crate::LodSceneHostStreamed);
		assert_eq!(
			PendingSample::begin(&world, root).and_then(|sample| sample.successor),
			Some((successor.to_bits(), Some(true)))
		);
		assert_eq!(probe.direct_children, 1);
		assert!(!probe.root_disabled);
		world.entity_mut(root).insert(Disabled);
		assert!(PendingSample::begin(&world, root).is_some_and(|sample| sample.root_disabled));
		despawn_retired_hosts(&mut world);
		assert!(world.get_entity(root).is_err());
		assert!(world.get_entity(child).is_err());
		assert_eq!(i64::from(world.entities().len()) - i64::from(probe.live_before), -2);
		Ok(())
	}

	#[test]
	fn probe_admits_direct_retired_hosts_without_scheme() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<RetirementProbe>();
		let root = world.spawn(RetiredHost).id();
		let probe = PendingSample::begin(&world, root).context("direct root should be sampled")?;
		assert_eq!(probe.scheme, "unknown");
		despawn_retired_hosts(&mut world);
		assert!(world.get_entity(root).is_err());
		Ok(())
	}
}
