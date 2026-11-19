//! Bounded provenance for slow calls in the existing exclusive retirement drain.

use std::time::Instant;

use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::{Children, Component, Entity, Resource, World};
use lod_generation_diagnostics::retired_host::{RetiredHostCompletion, RetiredHostSamples};

#[derive(Component)]
#[component(storage = "SparseSet")]
pub(super) struct RetiredHostOrigin(&'static str);

impl RetiredHostOrigin {
	pub(super) fn of<T>() -> Self {
		Self(std::any::type_name::<T>())
	}
}

#[derive(Default, Resource)]
pub(in crate::hcsg) struct RetirementProbe(RetiredHostSamples);

pub(super) struct PendingSample {
	root: Entity,
	scheme: &'static str,
	direct_children: usize,
	root_disabled: bool,
	live_before: u32,
	started: Instant,
}

impl PendingSample {
	pub(super) fn begin(world: &World, root: Entity) -> Option<Self> {
		let budget = world.get_resource::<RetirementProbe>()?;
		if budget.0.is_full() {
			return None;
		}
		Some(Self {
			root,
			// Direct RetiredHost callers need not carry a Presented<T> scheme.
			scheme: world.get::<RetiredHostOrigin>(root).map_or("unknown", |origin| origin.0),
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
		let root = world.spawn((RetiredHost, RetiredHostOrigin::of::<FixtureScheme>())).id();
		let child = world.spawn(ChildOf(root)).id();
		assert!(PendingSample::begin(&world, root).is_none());
		world.init_resource::<RetirementProbe>();
		let probe = PendingSample::begin(&world, root).context("marked root should be sampled")?;
		assert_eq!(probe.scheme, std::any::type_name::<FixtureScheme>());
		assert_eq!(probe.direct_children, 1);
		assert!(!probe.root_disabled);
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
