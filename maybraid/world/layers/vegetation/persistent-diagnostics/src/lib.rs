//! Bounded observations for the explicitly configured one-tile High fixture.
//!
//! Wire this plugin and its dependency only from the default-off diagnostic game
//! feature. The clean fixture does not depend on this crate or allocate counters.
//! Samples are emitted at most once per real second, never in a catch-up burst.
//! All expensive work is restricted to one fixture's descendants, with a hard cap.

use std::sync::atomic::Ordering;

use avian3d::prelude::Collider;
use bevy::prelude::*;
use chico::persistent::{
	FixtureMode, FixtureState, FixtureStatus, PersistentCounters, VegetationInstanceFixture,
};
use serde::Serialize;
use vegetation_components::VegetationInstance;

const MAX_DESCENDANTS: usize = 100_000;

pub struct VegetationPersistentDiagnosticsPlugin;

impl Plugin for VegetationPersistentDiagnosticsPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, sample_fixture);
	}
}

#[derive(Default)]
struct SampleClock {
	previous: Option<f64>,
}

impl SampleClock {
	fn take(&mut self, now: f64) -> Option<f64> {
		if !now.is_finite() {
			return None;
		}
		let interval = self.previous.map_or(0.0, |previous| now - previous);
		if self.previous.is_some() && interval < 1.0 {
			return None;
		}
		self.previous = Some(now);
		Some(interval)
	}
}

/// Cumulative command-side observations, not queued phase-item estimates.
#[derive(Clone, Copy, Default, Serialize)]
struct CounterSnapshot {
	upload_calls: u64,
	upload_bytes: u64,
	issued_main_draws: u64,
	issued_prepass_draws: u64,
	issued_shadow_draws: u64,
	issued_main_instances: u64,
	mesh_misses: u64,
}

impl CounterSnapshot {
	fn read(counters: &PersistentCounters) -> Self {
		let values = &counters.0;
		Self {
			upload_calls: values.upload_calls.load(Ordering::Relaxed),
			upload_bytes: values.upload_bytes.load(Ordering::Relaxed),
			issued_main_draws: values.issued_main_draws.load(Ordering::Relaxed),
			issued_prepass_draws: values.issued_prepass_draws.load(Ordering::Relaxed),
			issued_shadow_draws: values.issued_shadow_draws.load(Ordering::Relaxed),
			issued_main_instances: values.issued_main_instances.load(Ordering::Relaxed),
			mesh_misses: values.mesh_misses.load(Ordering::Relaxed),
		}
	}

	fn delta(self, previous: Self) -> Self {
		Self {
			upload_calls: self.upload_calls.saturating_sub(previous.upload_calls),
			upload_bytes: self.upload_bytes.saturating_sub(previous.upload_bytes),
			issued_main_draws: self.issued_main_draws.saturating_sub(previous.issued_main_draws),
			issued_prepass_draws: self
				.issued_prepass_draws
				.saturating_sub(previous.issued_prepass_draws),
			issued_shadow_draws: self
				.issued_shadow_draws
				.saturating_sub(previous.issued_shadow_draws),
			issued_main_instances: self
				.issued_main_instances
				.saturating_sub(previous.issued_main_instances),
			mesh_misses: self.mesh_misses.saturating_sub(previous.mesh_misses),
		}
	}

	fn reset_since(self, previous: Self) -> bool {
		self.upload_calls < previous.upload_calls
			|| self.upload_bytes < previous.upload_bytes
			|| self.issued_main_draws < previous.issued_main_draws
			|| self.issued_prepass_draws < previous.issued_prepass_draws
			|| self.issued_shadow_draws < previous.issued_shadow_draws
			|| self.issued_main_instances < previous.issued_main_instances
			|| self.mesh_misses < previous.mesh_misses
	}
}

#[derive(Serialize)]
struct FixtureSample {
	schema_version: u8,
	elapsed_seconds: f64,
	interval_seconds: f64,
	mode: &'static str,
	state: &'static str,
	reason: &'static str,
	selected_band: &'static str,
	fixture_hosts: usize,
	host_count_capped: bool,
	tile_min: [f32; 2],
	tile_id: String,
	recipe: &'static str,
	source_digest: u64,
	source_plants: usize,
	batches: usize,
	instances: usize,
	bounds_valid: bool,
	bounds_min: [f32; 3],
	bounds_max: [f32; 3],
	ordinary_render_entities: usize,
	semantic_anchors: usize,
	collider_compounds: usize,
	selected_proxies: usize,
	selected_proxy_entities: usize,
	render_scan_complete: bool,
	resident_bytes: u64,
	counters_available: bool,
	counters_reset: bool,
	counter_interval_valid: bool,
	pipeline_misses_available: bool,
	counters: CounterSnapshot,
	delta: CounterSnapshot,
}

/// Reads no general-world populations: the status marker identifies fixture roots.
/// A second root is sufficient to invalidate exact-one-host evidence, so enumeration
/// stops at two. Descendant traversal similarly reports truncation explicitly.
fn sample_fixture(
	time: Res<Time<Real>>,
	fixture: Option<Res<VegetationInstanceFixture>>,
	counters: Option<Res<PersistentCounters>>,
	hosts: Query<(Entity, &FixtureStatus)>,
	children: Query<&Children>,
	meshes: Query<(), With<Mesh3d>>,
	anchors: Query<(), With<VegetationInstance>>,
	colliders: Query<(), With<Collider>>,
	mut clock: Local<SampleClock>,
	mut previous: Local<CounterSnapshot>,
) {
	let Some(fixture) = fixture else { return };
	let Some(tile_min) = fixture.tile_min else { return };
	let now = time.elapsed_secs_f64();
	let Some(interval) = clock.take(now) else { return };
	let mut roots = hosts.iter().take(2);
	let root = roots.next();
	let duplicate = roots.next().is_some();
	let count = usize::from(root.is_some()) + usize::from(duplicate);
	let current = counters
		.as_ref()
		.map_or_else(CounterSnapshot::default, |value| CounterSnapshot::read(value));
	let mut sample = FixtureSample {
		schema_version: 1,
		elapsed_seconds: now,
		interval_seconds: interval,
		mode: match fixture.mode {
			FixtureMode::Ordinary => "ordinary",
			FixtureMode::Persistent => "persistent",
		},
		state: "Pending",
		reason: "fixture_host_absent",
		selected_band: "High",
		fixture_hosts: count,
		host_count_capped: duplicate,
		tile_min: tile_min.to_array(),
		tile_id: format!("{:?}", fixture.id()),
		recipe: "orchard_upper_canopy_100m_production32",
		source_digest: 0,
		source_plants: 0,
		batches: 0,
		instances: 0,
		bounds_valid: false,
		bounds_min: [0.0; 3],
		bounds_max: [0.0; 3],
		ordinary_render_entities: 0,
		semantic_anchors: 0,
		collider_compounds: 0,
		selected_proxies: 0,
		selected_proxy_entities: 0,
		render_scan_complete: true,
		resident_bytes: 0,
		counters_available: counters.is_some(),
		counters_reset: current.reset_since(*previous),
		counter_interval_valid: interval > 0.0
			&& counters.is_some()
			&& !current.reset_since(*previous),
		pipeline_misses_available: false,
		counters: current,
		delta: if interval > 0.0 { current.delta(*previous) } else { CounterSnapshot::default() },
	};
	*previous = current;
	if let Some((entity, status)) = root {
		sample.state = match status.state {
			FixtureState::Pending => "Pending",
			FixtureState::Ready => "Ready",
			FixtureState::Failed => "Failed",
		};
		sample.reason = status.reason;
		sample.source_digest = status.source_digest;
		sample.source_plants = status.source_plants;
		sample.batches = status.batches;
		sample.instances = status.instances;
		sample.bounds_min = status.bounds.min.to_array();
		sample.bounds_max = status.bounds.max.to_array();
		sample.bounds_valid = status.state == FixtureState::Ready
			&& status.bounds.min.is_finite()
			&& status.bounds.max.is_finite()
			&& status.bounds.min.cmple(status.bounds.max).all();
		sample.selected_proxies = status.selected_proxies;
		sample.resident_bytes = status.resident_bytes;
		let mut render_entities = usize::from(meshes.contains(entity));
		sample.semantic_anchors = usize::from(anchors.contains(entity));
		sample.collider_compounds = usize::from(colliders.contains(entity));
		let mut descendants = children.iter_descendants(entity);
		for descendant in descendants.by_ref().take(MAX_DESCENDANTS) {
			render_entities += usize::from(meshes.contains(descendant));
			sample.semantic_anchors += usize::from(anchors.contains(descendant));
			sample.collider_compounds += usize::from(colliders.contains(descendant));
		}
		sample.render_scan_complete = descendants.next().is_none();
		match status.mode {
			FixtureMode::Ordinary => sample.ordinary_render_entities = render_entities,
			FixtureMode::Persistent => sample.selected_proxy_entities = render_entities,
		}
	}
	if duplicate {
		sample.state = "Failed";
		sample.reason = "multiple_fixture_hosts";
		sample.bounds_valid = false;
	} else if !sample.render_scan_complete {
		sample.state = "Failed";
		sample.reason = "fixture_descendant_scan_limit";
	}
	match serde_json::to_string(&sample) {
		Ok(json) => {
			info!(target: "persistent_vegetation::diagnostics", sample = %json, "persistent_vegetation_sample")
		}
		Err(error) => {
			error!(target: "persistent_vegetation::diagnostics", %error, "persistent_vegetation_sample_serialization_failed")
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{CounterSnapshot, SampleClock};

	#[test]
	fn cadence_drops_catch_up_bursts_and_rejects_nonfinite_time() {
		let mut clock = SampleClock::default();
		assert_eq!(clock.take(f64::NAN), None);
		assert_eq!(clock.take(10.0), Some(0.0));
		assert_eq!(clock.take(10.999), None);
		assert_eq!(clock.take(11.0), Some(1.0));
		assert_eq!(clock.take(19.0), Some(8.0));
		assert_eq!(clock.take(19.0), None);
		assert_eq!(clock.take(19.5), None);
		assert_eq!(clock.take(20.0), Some(1.0));
	}

	#[test]
	fn command_counters_serialize_all_passes_and_saturate_on_reset() -> Result<(), serde_json::Error>
	{
		let previous =
			CounterSnapshot { upload_calls: 4, issued_main_draws: 7, ..Default::default() };
		let current = CounterSnapshot {
			upload_calls: 2,
			upload_bytes: 352,
			issued_main_draws: 10,
			issued_prepass_draws: 3,
			issued_shadow_draws: 5,
			issued_main_instances: 120,
			mesh_misses: 1,
		};
		assert!(current.reset_since(previous));
		let value = serde_json::to_value(current.delta(previous))?;
		assert_eq!(value["upload_calls"], 0);
		assert_eq!(value["upload_bytes"], 352);
		assert_eq!(value["issued_main_draws"], 3);
		assert_eq!(value["issued_prepass_draws"], 3);
		assert_eq!(value["issued_shadow_draws"], 5);
		assert_eq!(value["issued_main_instances"], 120);
		assert_eq!(value["mesh_misses"], 1);
		assert_eq!(value.as_object().map(|fields| fields.len()), Some(7));
		Ok(())
	}
}
