#![cfg(feature = "generation-tracing")]
//! Behavioral fixtures for the production context/worker instrumentation.
use bevy::math::DVec3;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{
	spawn_worker, universal_bounds, GenerationContext, GenerationScheme, HcsgClass, HcsgDemand,
	HcsgStorage,
};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tracing::field::{Field, Visit};
use tracing_subscriber::{layer::SubscriberExt, Layer};

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<BTreeMap<String, String>>>>);
#[derive(Default)]
struct Fields(BTreeMap<String, String>);
impl Visit for Fields {
	fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
		self.0.insert(field.name().to_owned(), format!("{value:?}"));
	}
	fn record_str(&mut self, field: &Field, value: &str) {
		self.0.insert(field.name().to_owned(), value.to_owned());
	}
}
impl<S: tracing::Subscriber> Layer<S> for Capture {
	// Fixtures use scoped dispatchers; keep interest dynamic across those scopes.
	// Subscriber-sensitive integration tests run outside the unit-worker process.
	fn register_callsite(
		&self,
		_: &'static tracing::Metadata<'static>,
	) -> tracing::subscriber::Interest {
		tracing::subscriber::Interest::sometimes()
	}
	fn max_level_hint(&self) -> Option<tracing::level_filters::LevelFilter> {
		Some(tracing::level_filters::LevelFilter::TRACE)
	}
	fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
		let mut fields = Fields::default();
		event.record(&mut fields);
		self.0.lock().unwrap_or_else(PoisonError::into_inner).push(fields.0);
	}
}
impl Capture {
	fn rows(&self) -> Vec<BTreeMap<String, String>> {
		self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
	}
}
struct Dependency(u32);
struct Root(u32);
impl GenerationScheme for Dependency {
	const INDEX_SCALE: DVec3 = DVec3::ONE;
	fn original_ids_for(_: &mut GenerationContext, _: bevy::shape::Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId(Id::Universal)]
	}
	fn build_with_id(_: &mut GenerationContext, _: Id) -> Option<(Self, bevy::shape::Aabb3d)> {
		Some((Self(41), universal_bounds()))
	}
}
impl GenerationScheme for Root {
	const INDEX_SCALE: DVec3 = DVec3::ONE;
	fn original_ids_for(_: &mut GenerationContext, _: bevy::shape::Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId(Id::Universal), OriginalId(Id::Universal)]
	}
	fn build_with_id(cx: &mut GenerationContext, _: Id) -> Option<(Self, bevy::shape::Aabb3d)> {
		Some((Self(cx.get_or_generate::<Dependency>(Id::Universal)?.0 + 1), universal_bounds()))
	}
}

#[test]
fn sync_cache_output_versions_and_non_lifo_context_lifetimes_are_preserved() -> anyhow::Result<()> {
	let capture = Capture::default();
	tracing::subscriber::with_default(
		tracing_subscriber::registry().with(capture.clone()),
		|| -> anyhow::Result<()> {
			let storage = HcsgStorage::default();
			let mut first = GenerationContext::with_origin(&storage, "fixture.first");
			let mut second = GenerationContext::with_origin(&storage, "fixture.second");
			let value = first
				.get_or_generate::<Root>(Id::Universal)
				.ok_or_else(|| anyhow::anyhow!("missing root"))?;
			let version = storage
				.entry::<Root>(Id::Universal)
				.ok_or_else(|| anyhow::anyhow!("missing entry"))?
				.version;
			let hit = first
				.get_or_generate::<Root>(Id::Universal)
				.ok_or_else(|| anyhow::anyhow!("missing hit"))?;
			assert_eq!(value.0, 42);
			assert!(Arc::ptr_eq(&value, &hit));
			drop(first); // The later context still lives; it must retain its identity.
			assert_eq!(second.get_or_generate::<Root>(Id::Universal).map(|r| r.0), Some(42));
			drop(second);
			assert_eq!(storage.entry::<Root>(Id::Universal).map(|e| e.version), Some(version));
			let rows = capture.rows();
			let first = rows
				.iter()
				.find(|r| r.get("origin").is_some_and(|s| s == "fixture.first"))
				.ok_or_else(|| anyhow::anyhow!("missing first completion"))?;
			assert_eq!(first.get("builds").map(String::as_str), Some("2"));
			assert_eq!(first.get("cache_hits").map(String::as_str), Some("1"));
			assert_eq!(first.get("published").map(String::as_str), Some("2"));
			let second = rows
				.iter()
				.find(|r| r.get("origin").is_some_and(|s| s == "fixture.second"))
				.ok_or_else(|| anyhow::anyhow!("missing second completion"))?;
			assert_eq!(second.get("builds").map(String::as_str), Some("0"));
			assert_eq!(second.get("cache_hits").map(String::as_str), Some("1"));
			Ok(())
		},
	)
}

#[test]
fn queued_replacement_cancellation_and_worker_output_remain_deduplicated() -> anyhow::Result<()> {
	let capture = Capture::default();
	tracing::subscriber::with_default(
		tracing_subscriber::registry().with(capture.clone()),
		|| -> anyhow::Result<()> {
			let storage = HcsgStorage::default();
			let demand = HcsgDemand::default();
			let old =
				demand.subscribe::<Root>(None, vec![universal_bounds()], None, HcsgClass::Near);
			demand.subscribe::<Root>(
				Some(old),
				vec![universal_bounds(), universal_bounds()],
				None,
				HcsgClass::Near,
			);
			std::thread::sleep(Duration::from_millis(5));
			let worker = spawn_worker(storage.clone(), demand.clone())?;
			assert!(demand.wait_idle(Duration::from_secs(5)));
			drop(worker);
			assert_eq!(storage.get::<Root>(Id::Universal).map(|r| r.0), Some(42));
			let rows = capture.rows();
			assert!(
				rows.iter().any(|r| r.get("message").is_some_and(|s| s == "hcsg.cancel")
					&& r.get("reason").is_some_and(|s| s == "replaced")),
				"captured events: {rows:?}"
			);
			let quantum = rows
				.iter()
				.find(|r| r.get("message").is_some_and(|s| s == "hcsg.quantum.complete"))
				.ok_or_else(|| anyhow::anyhow!("missing quantum completion"))?;
			assert_eq!(quantum.get("visited_ids").map(String::as_str), Some("1"));
			let operation = rows
				.iter()
				.find(|r| {
					r.get("message").is_some_and(|s| s == "hcsg.operation.complete")
						&& r.get("epoch_known").is_some_and(|s| s == "true")
				})
				.ok_or_else(|| anyhow::anyhow!("missing worker operation"))?;
			assert_eq!(operation.get("builds").map(String::as_str), Some("2"));
			assert_eq!(operation.get("published").map(String::as_str), Some("2"));
			assert!(
				operation
					.get("queue_wait_ns")
					.ok_or_else(|| anyhow::anyhow!("missing queue delay"))?
					.parse::<u64>()?
					> 0
			);
			assert_eq!(operation.get("subscription"), quantum.get("subscription"));
			Ok(())
		},
	)
}

#[test]
fn stale_context_does_not_build_or_publish() -> anyhow::Result<()> {
	let capture = Capture::default();
	tracing::subscriber::with_default(
		tracing_subscriber::registry().with(capture.clone()),
		|| -> anyhow::Result<()> {
			let storage = HcsgStorage::default();
			{
				let mut cx = GenerationContext::with_stale(&storage, &|| true);
				assert!(cx.get_or_generate::<Root>(Id::Universal).is_none());
			}
			assert!(storage.get::<Root>(Id::Universal).is_none());
			let rows = capture.rows();
			let row = rows
				.iter()
				.find(|r| r.get("message").is_some_and(|s| s == "hcsg.operation.complete"))
				.ok_or_else(|| anyhow::anyhow!("missing completion"))?;
			assert_eq!(row.get("cancelled").map(String::as_str), Some("1"));
			assert_eq!(row.get("builds").map(String::as_str), Some("0"));
			assert_eq!(row.get("published").map(String::as_str), Some("0"));
			Ok(())
		},
	)
}

struct SlowDiscovery(u32);
impl GenerationScheme for SlowDiscovery {
	const INDEX_SCALE: DVec3 = DVec3::ONE;
	fn original_ids_for(_: &mut GenerationContext, _: bevy::shape::Aabb3d) -> Vec<OriginalId> {
		// Discovery is cooperative: it can finish after the 30 ms quantum.
		std::thread::sleep(Duration::from_millis(35));
		vec![OriginalId(Id::Universal)]
	}
	fn build_with_id(cx: &mut GenerationContext, _: Id) -> Option<(Self, bevy::shape::Aabb3d)> {
		Some((Self(cx.get_or_generate::<Dependency>(Id::Universal)?.0 + 1), universal_bounds()))
	}
}

#[test]
fn discovery_overrun_yields_and_resumed_quantum_retains_correlation() -> anyhow::Result<()> {
	let capture = Capture::default();
	tracing::subscriber::with_default(
		tracing_subscriber::registry().with(capture.clone()),
		|| -> anyhow::Result<()> {
			let storage = HcsgStorage::default();
			let demand = HcsgDemand::default();
			demand.subscribe::<SlowDiscovery>(
				None,
				vec![universal_bounds()],
				None,
				HcsgClass::Near,
			);
			let worker = spawn_worker(storage.clone(), demand.clone())?;
			assert!(demand.wait_idle(Duration::from_secs(5)));
			drop(worker);
			assert_eq!(storage.get::<SlowDiscovery>(Id::Universal).map(|v| v.0), Some(42));
			let rows = capture.rows();
			let quanta: Vec<_> = rows
				.iter()
				.filter(|r| r.get("message").is_some_and(|s| s == "hcsg.quantum.complete"))
				.collect();
			assert_eq!(quanta.len(), 2);
			assert_eq!(quanta[0].get("subscription"), quanta[1].get("subscription"));
			assert_eq!(quanta[0].get("quantum").map(String::as_str), Some("1"));
			assert_eq!(quanta[1].get("quantum").map(String::as_str), Some("2"));
			assert_eq!(quanta[0].get("visited_ids").map(String::as_str), Some("0"));
			assert_eq!(quanta[0].get("overrun").map(String::as_str), Some("true"));
			assert_eq!(quanta[0].get("done").map(String::as_str), Some("false"));
			assert_eq!(quanta[1].get("visited_ids").map(String::as_str), Some("1"));
			assert_eq!(quanta[1].get("done").map(String::as_str), Some("true"));
			Ok(())
		},
	)
}

#[test]
fn demand_lifecycle_output_runs_after_releasing_registry_guard() -> anyhow::Result<()> {
	#[derive(Clone)]
	struct LockProbe {
		demand: HcsgDemand,
		previous: Arc<Mutex<Option<lod::hcsg::SubscriptionId>>>,
		available: Arc<Mutex<Vec<bool>>>,
	}
	impl<S: tracing::Subscriber> Layer<S> for LockProbe {
		fn on_event(
			&self,
			event: &tracing::Event<'_>,
			_: tracing_subscriber::layer::Context<'_, S>,
		) {
			let mut fields = Fields::default();
			event.record(&mut fields);
			if !fields
				.0
				.get("message")
				.is_some_and(|s| s == "hcsg.cancel" || s == "hcsg.enqueue")
			{
				return;
			}
			let previous = *self.previous.lock().unwrap_or_else(PoisonError::into_inner);
			if let Some(previous) = previous {
				let available = matches!(self.demand.try_read(previous, 0), Ok(None));
				self.available.lock().unwrap_or_else(PoisonError::into_inner).push(available);
			}
		}
	}
	let demand = HcsgDemand::default();
	let probe = LockProbe {
		demand: demand.clone(),
		previous: Default::default(),
		available: Default::default(),
	};
	tracing::subscriber::with_default(
		tracing_subscriber::registry().with(Capture::default()).with(probe.clone()),
		|| {
			let old =
				demand.subscribe::<Root>(None, vec![universal_bounds()], None, HcsgClass::Near);
			*probe.previous.lock().unwrap_or_else(PoisonError::into_inner) = Some(old);
			demand.subscribe::<Root>(Some(old), vec![universal_bounds()], None, HcsgClass::Near);
		},
	);
	let checks = probe.available.lock().unwrap_or_else(PoisonError::into_inner);
	assert_eq!(checks.len(), 2, "replacement cancellation and enqueue must both be observable");
	assert!(
		checks.iter().all(|available| *available),
		"diagnostic output held demand guard: {checks:?}"
	);
	Ok(())
}
