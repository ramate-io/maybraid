use std::{cell::Cell, sync::OnceLock};

const MAX_VIEWS: usize = 32;
const MAX_DETAIL_SPANS: usize = 64;

#[derive(Clone, Copy)]
struct Settings {
	enabled: bool,
	interval: u32,
}

impl Settings {
	fn from_env() -> Self {
		Self {
			enabled: std::env::var("MAYBRAID_RENDER_PREP_DIAGNOSTICS").as_deref() == Ok("1"),
			interval: Self::interval(
				std::env::var("MAYBRAID_RENDER_PREP_SAMPLE_FRAMES").ok().as_deref(),
			),
		}
	}

	fn interval(raw: Option<&str>) -> u32 {
		raw.and_then(|value| value.parse().ok()).unwrap_or(120).max(1)
	}

	fn due(self, frame: u32) -> bool {
		self.enabled && frame.is_multiple_of(self.interval)
	}
}

/// One sampled writer invocation. All limits are per phase per sampled frame.
/// No allocations or scans occur when the runtime switch is off or cadence skips.
pub struct Capture {
	frame: u32,
	phase: &'static str,
	views: Cell<usize>,
	details: Cell<usize>,
}

impl Capture {
	pub fn begin(frame: u32, phase: &'static str) -> Option<Self> {
		static SETTINGS: OnceLock<Settings> = OnceLock::new();
		SETTINGS.get_or_init(Settings::from_env).due(frame).then_some(Self {
			frame,
			phase,
			views: Cell::new(0),
			details: Cell::new(0),
		})
	}

	pub fn writer_span(&self) -> tracing::Span {
		tracing::info_span!("render_prep_writer", frame = self.frame, phase = self.phase)
	}

	pub fn view(&self, view: impl FnOnce() -> String) -> Option<Census> {
		let seen = self.views.get();
		self.views.set(seen + 1);
		(seen < MAX_VIEWS).then(|| {
			let mut census = Census::default();
			census.frame = self.frame;
			census.phase = self.phase;
			census.view = view();
			census
		})
	}

	pub fn buffer_span(&self, kind: BufferKind, snapshot: BufferSnapshot) -> Option<tracing::Span> {
		let details = self.details.get();
		self.details.set(details + 1);
		if details >= MAX_DETAIL_SPANS {
			return None;
		}
		Some(tracing::info_span!(
			"render_prep_buffer",
			kind = kind.name(),
			entries = snapshot.entries,
			logical_bytes = snapshot.logical_bytes(),
			destination_capacity = snapshot.capacity,
			destination_bytes = snapshot.destination_bytes
		))
	}
}

impl Drop for Capture {
	fn drop(&mut self) {
		if self.details.get() > MAX_DETAIL_SPANS {
			tracing::info!("render_prep_detail_truncated frame={} phase={} detail_spans_emitted={} detail_spans_omitted={}",
				self.frame, self.phase, MAX_DETAIL_SPANS, self.details.get() - MAX_DETAIL_SPANS);
		}
		if self.views.get() > MAX_VIEWS {
			tracing::info!(
				"render_prep_truncated frame={} phase={} views_omitted={}",
				self.frame,
				self.phase,
				self.views.get() - MAX_VIEWS
			);
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub enum BufferKind {
	Instances,
	Metadata,
	Lookup,
	Fan,
}

impl BufferKind {
	fn name(self) -> &'static str {
		match self {
			Self::Instances => "instances",
			Self::Metadata => "metadata",
			Self::Lookup => "lookup",
			Self::Fan => "fan",
		}
	}
	fn index(self) -> usize {
		match self {
			Self::Instances => 0,
			Self::Metadata => 1,
			Self::Lookup => 2,
			Self::Fan => 3,
		}
	}
}

/// Exact native vector lengths/capacities and public destination byte size.
#[derive(Clone, Copy, Default, Debug)]
pub struct BufferSnapshot {
	pub entries: usize,
	pub item_size: usize,
	pub capacity: usize,
	pub destination_bytes: u64,
}

impl BufferSnapshot {
	fn logical_bytes(self) -> u64 {
		self.entries as u64 * self.item_size as u64
	}
}

#[derive(Default, Debug, PartialEq, Eq)]
struct BufferCounts {
	entries: u64,
	logical_bytes: u64,
	queue_calls: u64,
	upload_bytes: u64,
	capacity_before: u64,
	capacity_after: u64,
	destination_bytes_before: u64,
	destination_bytes_after: u64,
	capacity_growths: u64,
	observed_handle_changes: u64,
}

impl BufferCounts {
	fn record(
		&mut self,
		kind: BufferKind,
		before: BufferSnapshot,
		after: BufferSnapshot,
		handle_changed: bool,
	) {
		self.entries += before.entries as u64;
		self.logical_bytes += before.logical_bytes();
		// Called only AFTER the native write returns. The pinned RawBufferVec
		// implementation invokes queue.write_buffer once for every nonempty
		// vector; fan is UninitBufferVec scratch and performs no CPU upload.
		if !matches!(kind, BufferKind::Fan) && before.entries > 0 {
			self.queue_calls += 1;
			self.upload_bytes += before.logical_bytes();
		}
		self.capacity_before += before.capacity as u64;
		self.capacity_after += after.capacity as u64;
		self.destination_bytes_before += before.destination_bytes;
		self.destination_bytes_after += after.destination_bytes;
		self.capacity_growths += u64::from(after.capacity > before.capacity);
		self.observed_handle_changes += u64::from(handle_changed);
	}
}

/// Completed values are emitted on drop, including every native early return.
/// All fields describe THIS writer invocation, never retained jobs from an older frame.
#[derive(Default, Debug)]
pub struct Census {
	frame: u32,
	phase: &'static str,
	view: String,
	pub mode: &'static str,
	pub candidate_sets: usize,
	pub bins: usize,
	pub instances: usize,
	pub batchable_bins: usize,
	pub batchable_instances: usize,
	pub unbatchable_bins: usize,
	pub unbatchable_instances: usize,
	pub non_mesh_bins: usize,
	pub non_mesh_instances: usize,
	pub lookup_input_sets: usize,
	pub lookup_entries: usize,
	pub eligible_sets: usize,
	pub skipped_mode_sets: usize,
	pub skipped_no_representative: usize,
	pub skipped_no_metadata: usize,
	pub skipped_missing_buffers: usize,
	pub skipped_index_overflow: usize,
	pub indexed_jobs: usize,
	pub non_indexed_jobs: usize,
	buffers: [BufferCounts; 4],
}

impl Census {
	pub fn lookup_span(&self) -> tracing::Span {
		tracing::info_span!(
			"render_prep_lookup",
			frame = self.frame,
			phase = self.phase,
			view = self.view.as_str()
		)
	}

	pub fn upload_jobs_span(&self) -> tracing::Span {
		tracing::info_span!(
			"render_prep_upload_jobs",
			frame = self.frame,
			phase = self.phase,
			view = self.view.as_str()
		)
	}

	pub fn record_buffer(
		&mut self,
		kind: BufferKind,
		before: BufferSnapshot,
		after: BufferSnapshot,
		handle_changed: bool,
	) {
		self.buffers[kind.index()].record(kind, before, after, handle_changed);
	}
}

impl Drop for Census {
	fn drop(&mut self) {
		// Numeric values are complete before emitting this bounded message.
		// Tracy receives them without relying on late span-field recording.
		tracing::info!("render_prep_census {:?}", self);
	}
}

#[cfg(test)]
mod tests;
