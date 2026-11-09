//! Opt-in generation diagnostics. Only `lod/generation-tracing` installs callers.
//! Counters describe operation scope, including nested work, never exclusive CPU.

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use tracing::span::EnteredSpan;

pub const TARGET: &str = "maybraid.generation.profile";
static NEXT_OPERATION: AtomicU64 = AtomicU64::new(1);
thread_local! {
	static ACTIVE: RefCell<Vec<Active>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Correlation {
	pub subscription: u64,
	pub epoch: u64,
	pub quantum: u64,
	pub queue_wait_ns: u64,
	pub epoch_known: bool,
	pub application_session: u64,
}

#[derive(Debug, Default)]
pub struct Counts {
	pub active_inclusive_ns: u64,
	pub coarse_spans: u64,
	pub coarse_suppressed: u64,
	pub cache_hits: u64,
	pub cache_misses: u64,
	pub discoveries: u64,
	pub builds: u64,
	pub absent: u64,
	pub cancelled: u64,
	pub cycles: u64,
	pub published: u64,
	pub first_version: u64,
	pub last_version: u64,
	pub applied: u64,
	pub unchanged: u64,
	pub retired: u64,
	pub removed: u64,
	pub busy: u64,
	pub registry_acquires: u64,
	pub registry_wait_ns: u64,
	pub registry_held_ns: u64,
	pub store_acquires: u64,
	pub store_wait_ns: u64,
	pub store_held_ns: u64,
	pub discovery_inclusive_ns: u64,
	pub build_inclusive_ns: u64,
	pub publish_inclusive_ns: u64,
	pub details: u64,
	pub suppressed: u64,
}

pub fn enabled() -> bool {
	tracing::enabled!(target: "maybraid.generation.profile", tracing::Level::TRACE)
}

pub fn nanos(duration: Duration) -> u64 {
	duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

pub fn update(f: impl FnOnce(&mut Counts)) {
	if !enabled() {
		return;
	}
	ACTIVE.with(|active| {
		if let Some(counts) = active.borrow().last() {
			f(&mut counts.counts.lock().unwrap_or_else(PoisonError::into_inner));
		}
	});
}

/// Small stable family labels; full Rust type names are optional bounded detail.
pub fn family(ty: &str) -> &'static str {
	if ty.contains("WorldPoiTiles") {
		"world-poi"
	} else if ty.contains("PlacePois") || ty.contains("DevelopmentPlaces") {
		"places"
	} else if ty.contains("Grove") || ty.contains("Forest") {
		"vegetation"
	} else if ty.contains("Built<") || ty.contains("Development") || ty.contains("Urban") {
		"urban"
	} else if ty.contains("Terrain") || ty.contains("Durham") || ty.contains("Watershed") {
		"terrain"
	} else {
		"other"
	}
}

pub struct Operation {
	state: Arc<OperationState>,
}
struct OperationState {
	counts: Arc<Mutex<Counts>>,
	id: u64,
	origin: &'static str,
	family: &'static str,
	correlation: Correlation,
	started: Instant,
	span: tracing::Span,
}

impl Operation {
	pub fn new(origin: &'static str, ty: &'static str, correlation: Correlation) -> Option<Self> {
		if !enabled() {
			return None;
		}
		let id = NEXT_OPERATION.fetch_add(1, Ordering::Relaxed);
		let family = family(ty);
		let counts = Arc::new(Mutex::new(Counts::default()));
		// Tracy formats fields into zone names. Keep those fields stable;
		// operation/subscription/timing values belong in completion messages.
		let span = tracing::trace_span!(target: "maybraid.generation.profile", "hcsg.operation", origin, family);
		Some(Self {
			state: Arc::new(OperationState {
				counts,
				id,
				origin,
				family,
				correlation,
				started: Instant::now(),
				span,
			}),
		})
	}

	/// Lexical work scope. The returned guard owns its state, not a borrow of
	/// the context. No entered span is retained between Context calls.
	pub fn enter(&self) -> Option<EnteredOperation> {
		if !enabled() {
			return None;
		}
		if ACTIVE.with(|active| {
			active.borrow().last().is_some_and(|entry| entry.operation == self.state.id)
		}) {
			return None;
		}
		let token = NEXT_OPERATION.fetch_add(1, Ordering::Relaxed);
		ACTIVE.with(|active| {
			active.borrow_mut().push(Active {
				token,
				operation: self.state.id,
				counts: Arc::clone(&self.state.counts),
			})
		});
		Some(EnteredOperation {
			token,
			state: Arc::clone(&self.state),
			started: Instant::now(),
			_span: self.state.span.clone().entered(),
		})
	}
}

struct Active {
	token: u64,
	operation: u64,
	counts: Arc<Mutex<Counts>>,
}
pub struct EnteredOperation {
	token: u64,
	started: Instant,
	_span: EnteredSpan,
	state: Arc<OperationState>,
}
impl Drop for EnteredOperation {
	fn drop(&mut self) {
		self.state
			.counts
			.lock()
			.unwrap_or_else(PoisonError::into_inner)
			.active_inclusive_ns += nanos(self.started.elapsed());
		ACTIVE.with(|active| {
			let mut entries = active.borrow_mut();
			if let Some(index) = entries.iter().position(|entry| entry.token == self.token) {
				entries.remove(index);
			}
		});
	}
}

impl Drop for OperationState {
	fn drop(&mut self) {
		let c = self.counts.lock().unwrap_or_else(PoisonError::into_inner);
		// Final values are present when the event is created. The tracing-tracy
		// 0.12 adapter records this as a message for the Tracy 0.14.1 collector;
		// no late span fields or empty count zones are used.
		tracing::trace!(target: "maybraid.generation.profile", parent: &self.span,
            operation = self.id, origin = self.origin, family = self.family,
            subscription = self.correlation.subscription, epoch = self.correlation.epoch,
            quantum = self.correlation.quantum, queue_wait_ns = self.correlation.queue_wait_ns,
            epoch_known = self.correlation.epoch_known, application_session = self.correlation.application_session,
            lifetime_ns = nanos(self.started.elapsed()), active_inclusive_ns = c.active_inclusive_ns,
            coarse_spans = c.coarse_spans, coarse_suppressed = c.coarse_suppressed, cache_hits = c.cache_hits, cache_misses = c.cache_misses,
            discoveries = c.discoveries, builds = c.builds, absent = c.absent, cancelled = c.cancelled,
            cycles = c.cycles, published = c.published, first_version = c.first_version,
            last_version = c.last_version, applied = c.applied, unchanged = c.unchanged,
            retired = c.retired, removed = c.removed, busy = c.busy,
            registry_acquires = c.registry_acquires, registry_wait_ns = c.registry_wait_ns,
            registry_held_ns = c.registry_held_ns, store_acquires = c.store_acquires,
            store_wait_ns = c.store_wait_ns, store_held_ns = c.store_held_ns,
            discovery_inclusive_ns = c.discovery_inclusive_ns, build_inclusive_ns = c.build_inclusive_ns,
            publish_inclusive_ns = c.publish_inclusive_ns, details = c.details, suppressed = c.suppressed,
            "hcsg.operation.complete");
	}
}

struct DetailConfig {
	enabled: bool,
	depth: usize,
	limit: u64,
	ty: Option<String>,
}
fn config() -> &'static DetailConfig {
	static CONFIG: OnceLock<DetailConfig> = OnceLock::new();
	CONFIG.get_or_init(|| {
		let depth = std::env::var("MAYBRAID_GENERATION_TRACE_DEPTH").ok();
		let ty = std::env::var("MAYBRAID_GENERATION_TRACE_TYPE").ok();
		DetailConfig {
			enabled: depth.is_some() || ty.is_some(),
			depth: depth.and_then(|s| s.parse().ok()).unwrap_or(0).min(16),
			limit: std::env::var("MAYBRAID_GENERATION_TRACE_DETAILS")
				.ok()
				.and_then(|s| s.parse().ok())
				.unwrap_or(32)
				.min(256),
			ty,
		}
	})
}

pub struct Phase {
	kind: &'static str,
	started: Instant,
	_detail: Option<EnteredSpan>,
	_coarse: Option<EnteredSpan>,
}
impl Phase {
	pub fn new(kind: &'static str, ty: &'static str, depth: usize) -> Option<Self> {
		if !enabled() || !ACTIVE.with(|a| !a.borrow().is_empty()) {
			return None;
		}
		let config = config();
		let selected = config.enabled
			&& depth <= config.depth
			&& config.ty.as_ref().is_none_or(|s| ty.contains(s));
		let mut coarse = false;
		let mut detail = false;
		update(|c| {
			if depth == 0 {
				if c.coarse_spans < 64 {
					c.coarse_spans += 1;
					coarse = true;
				} else {
					c.coarse_suppressed += 1;
				}
			}
			if selected && c.details < config.limit {
				c.details += 1;
				detail = true;
			} else if config.enabled {
				c.suppressed += 1;
			}
		});
		let coarse = coarse.then(|| {
			tracing::trace_span!(target: "maybraid.generation.profile", "hcsg.phase",
            phase = kind, family = family(ty))
			.entered()
		});
		let detail = detail.then(|| {
			tracing::trace_span!(target: "maybraid.generation.profile", "hcsg.dependency",
            phase = kind, family = family(ty), scheme = ty, depth)
			.entered()
		});
		Some(Self { kind, started: Instant::now(), _coarse: coarse, _detail: detail })
	}
}
impl Drop for Phase {
	fn drop(&mut self) {
		let elapsed = nanos(self.started.elapsed());
		update(|c| match self.kind {
			"discover" => c.discovery_inclusive_ns += elapsed,
			"build" => c.build_inclusive_ns += elapsed,
			"publish" => c.publish_inclusive_ns += elapsed,
			_ => {}
		});
	}
}

/// Acquisition time is separate from the lifetime of the protected operation.
/// These guards never extend a lock's lifetime past the original lexical scope.
pub struct LockWait {
	kind: &'static str,
	started: Option<Instant>,
}
impl LockWait {
	pub fn new(kind: &'static str) -> Self {
		Self {
			kind,
			started: (enabled() && ACTIVE.with(|a| !a.borrow().is_empty())).then(Instant::now),
		}
	}
	pub fn acquired<G>(self, guard: G) -> MeasuredGuard<G> {
		let held = self.started.map(|started| {
			let wait = nanos(started.elapsed());
			update(|c| {
				if self.kind == "registry" {
					c.registry_acquires += 1;
					c.registry_wait_ns += wait;
				} else {
					c.store_acquires += 1;
					c.store_wait_ns += wait;
				}
			});
			Instant::now()
		});
		MeasuredGuard { guard, kind: self.kind, held }
	}
}
pub struct MeasuredGuard<G> {
	guard: G,
	kind: &'static str,
	held: Option<Instant>,
}
impl<G: Deref> Deref for MeasuredGuard<G> {
	type Target = G::Target;
	fn deref(&self) -> &Self::Target {
		&self.guard
	}
}
impl<G: DerefMut> DerefMut for MeasuredGuard<G> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.guard
	}
}
impl<G> Drop for MeasuredGuard<G> {
	fn drop(&mut self) {
		if let Some(started) = self.held {
			let elapsed = nanos(started.elapsed());
			update(|c| {
				if self.kind == "registry" {
					c.registry_held_ns += elapsed;
				} else {
					c.store_held_ns += elapsed;
				}
			});
		}
	}
}

pub fn enqueue(
	origin: &'static str,
	scheme: &'static str,
	subscription: u64,
	epoch: u64,
	enqueue_age_ns: u64,
) {
	tracing::trace!(target: "maybraid.generation.profile", origin, family = family(scheme),
        subscription, epoch, enqueue_age_ns, "hcsg.enqueue");
}

pub fn quantum_complete(
	subscription: u64,
	epoch: u64,
	quantum: u64,
	done: bool,
	cancelled: bool,
	ids: usize,
	elapsed: Duration,
	overrun: bool,
) {
	tracing::trace!(target: "maybraid.generation.profile", subscription, epoch, quantum, done,
        cancelled, visited_ids = ids, elapsed_ns = nanos(elapsed), overrun, "hcsg.quantum.complete");
}

pub fn cancel(subscription: u64, epoch: u64, reason: &'static str) {
	tracing::trace!(target: "maybraid.generation.profile", subscription, epoch, reason, "hcsg.cancel");
}

#[cfg(test)]
mod tests;

/// Optional bounded exact publication detail. Summary endpoints are not a
/// contiguous version list when multiple operations publish concurrently.
pub fn publication(version: u64, scheme: &'static str, node: &impl std::fmt::Debug) {
	if !enabled() {
		return;
	}
	let config = config();
	if !config.enabled {
		return;
	}
	let selected = config.ty.as_ref().is_none_or(|s| scheme.contains(s));
	let mut granted = false;
	update(|c| {
		if selected && c.details < config.limit {
			c.details += 1;
			granted = true;
		} else {
			c.suppressed += 1;
		}
	});
	if granted {
		let operation = ACTIVE.with(|active| active.borrow().last().map(|entry| entry.operation));
		tracing::trace!(target: "maybraid.generation.profile", operation, version, family = family(scheme), scheme, node = ?node, "hcsg.publish");
	}
}
