use crate::*;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError, RwLock};
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

#[test]
fn final_event_survives_out_of_order_owner_drop_and_waits_for_work_scope() -> anyhow::Result<()> {
	fn send_sync<T: Send + Sync>() {}
	send_sync::<Operation>();
	let capture = Capture::default();
	let subscriber = tracing_subscriber::registry().with(capture.clone());
	tracing::subscriber::with_default(subscriber, || -> anyhow::Result<()> {
		let first = Operation::new("first", "Fixture", Correlation::default())
			.ok_or_else(|| anyhow::anyhow!("operation disabled"))?;
		let second = Operation::new("second", "Fixture", Correlation::default())
			.ok_or_else(|| anyhow::anyhow!("operation disabled"))?;
		let scope = first.enter();
		update(|c| c.cache_hits += 1);
		drop(first);
		assert!(capture.rows().is_empty());
		drop(scope);
		let scope = second.enter();
		update(|c| c.cache_misses += 1);
		drop(scope);
		drop(second);
		let rows = capture.rows();
		assert_eq!(rows.len(), 2);
		assert_eq!(rows[0].get("origin").map(String::as_str), Some("first"));
		assert_eq!(rows[0].get("cache_hits").map(String::as_str), Some("1"));
		assert_eq!(rows[1].get("origin").map(String::as_str), Some("second"));
		assert_eq!(rows[1].get("cache_misses").map(String::as_str), Some("1"));
		Ok(())
	})
}

#[test]
fn lock_acquisition_held_work_and_busy_are_distinct_from_generation() -> anyhow::Result<()> {
	let capture = Capture::default();
	tracing::subscriber::with_default(
		tracing_subscriber::registry().with(capture.clone()),
		|| -> anyhow::Result<()> {
			let operation = Operation::new("lock-fixture", "Fixture", Correlation::default())
				.ok_or_else(|| anyhow::anyhow!("operation disabled"))?;
			let scope = operation.enter();
			let lock = Arc::new(RwLock::new(7));
			let (ready_tx, ready_rx) = std::sync::mpsc::channel();
			let (release_tx, release_rx) = std::sync::mpsc::channel();
			let worker_lock = Arc::clone(&lock);
			let worker = std::thread::spawn(move || -> anyhow::Result<()> {
				let _guard = worker_lock.write().map_err(|e| anyhow::anyhow!("{e}"))?;
				ready_tx.send(())?;
				release_rx.recv()?;
				std::thread::sleep(Duration::from_millis(20));
				Ok(())
			});
			ready_rx.recv()?;
			assert!(lock.try_read().is_err());
			update(|c| c.busy += 1);
			let wait = LockWait::new("store");
			release_tx.send(())?;
			let guard = wait.acquired(lock.read().map_err(|e| anyhow::anyhow!("{e}"))?);
			assert_eq!(*guard, 7);
			drop(guard);
			worker.join().map_err(|_| anyhow::anyhow!("lock worker failed"))??;
			drop(scope);
			drop(operation);
			let rows = capture.rows();
			let row = rows.first().ok_or_else(|| anyhow::anyhow!("missing completion"))?;
			assert_eq!(row.get("store_acquires").map(String::as_str), Some("1"));
			assert_eq!(row.get("busy").map(String::as_str), Some("1"));
			assert_eq!(row.get("builds").map(String::as_str), Some("0"));
			assert!(
				row.get("store_wait_ns")
					.ok_or_else(|| anyhow::anyhow!("missing wait"))?
					.parse::<u64>()?
					> 0
			);
			assert!(
				row.get("store_held_ns")
					.ok_or_else(|| anyhow::anyhow!("missing held"))?
					.parse::<u64>()?
					> 0
			);
			Ok(())
		},
	)
}

#[test]
fn disabled_filter_starts_no_operation_or_scope() {
	tracing::subscriber::with_default(
		tracing_subscriber::registry().with(tracing_subscriber::filter::LevelFilter::OFF),
		|| {
			assert!(!enabled());
			assert!(Operation::new("off", "Fixture", Correlation::default()).is_none());
			assert!(Phase::new("build", "Fixture", 0).is_none());
		},
	);
}

#[test]
fn operation_phase_output_is_bounded_without_losing_aggregate_accounting() -> anyhow::Result<()> {
	let capture = Capture::default();
	tracing::subscriber::with_default(
		tracing_subscriber::registry().with(capture.clone()),
		|| -> anyhow::Result<()> {
			let operation = Operation::new("bounded", "Fixture", Correlation::default())
				.ok_or_else(|| anyhow::anyhow!("operation disabled"))?;
			let scope = operation.enter();
			for _ in 0..300 {
				let _phase = Phase::new("build", "Fixture", 0);
				update(|c| c.builds += 1);
			}
			drop(scope);
			drop(operation);
			let rows = capture.rows();
			let row = rows.first().ok_or_else(|| anyhow::anyhow!("missing summary"))?;
			assert_eq!(row.get("builds").map(String::as_str), Some("300"));
			assert_eq!(row.get("coarse_spans").map(String::as_str), Some("64"));
			assert_eq!(row.get("coarse_suppressed").map(String::as_str), Some("236"));
			assert!(
				row.get("details")
					.ok_or_else(|| anyhow::anyhow!("missing detail count"))?
					.parse::<u64>()?
					<= 256
			);
			Ok(())
		},
	)
}

#[test]
fn operation_span_fields_stay_stable_while_messages_keep_correlation() -> anyhow::Result<()> {
	#[derive(Clone, Default)]
	struct Spans(Arc<Mutex<Vec<BTreeMap<String, String>>>>);
	impl<S: tracing::Subscriber> Layer<S> for Spans {
		fn on_new_span(
			&self,
			attrs: &tracing::span::Attributes<'_>,
			_: &tracing::Id,
			_: tracing_subscriber::layer::Context<'_, S>,
		) {
			if attrs.metadata().name() == "hcsg.operation" {
				let mut fields = Fields::default();
				attrs.record(&mut fields);
				self.0.lock().unwrap_or_else(PoisonError::into_inner).push(fields.0);
			}
		}
	}
	let capture = Capture::default();
	let spans = Spans::default();
	tracing::subscriber::with_default(
		tracing_subscriber::registry().with(capture.clone()).with(spans.clone()),
		|| -> anyhow::Result<()> {
			for subscription in [11, 29] {
				let operation = Operation::new(
					"worker",
					"Fixture",
					Correlation {
						subscription,
						epoch: subscription + 1,
						quantum: subscription + 2,
						queue_wait_ns: subscription + 3,
						epoch_known: true,
						application_session: subscription + 4,
					},
				)
				.ok_or_else(|| anyhow::anyhow!("operation disabled"))?;
				drop(operation.enter());
				drop(operation);
			}
			let names = spans.0.lock().unwrap_or_else(PoisonError::into_inner);
			assert_eq!(names.len(), 2);
			assert_eq!(names[0], names[1]);
			assert_eq!(
				names[0].keys().map(String::as_str).collect::<Vec<_>>(),
				vec!["family", "origin"]
			);
			let rows = capture.rows();
			assert_eq!(rows.len(), 2);
			assert_ne!(rows[0].get("operation"), rows[1].get("operation"));
			assert_eq!(rows[0].get("subscription").map(String::as_str), Some("11"));
			assert_eq!(rows[1].get("subscription").map(String::as_str), Some("29"));
			Ok(())
		},
	)
}
