use crate::capture::{
	BufferKind, BufferSnapshot, Capture, Census, Settings, MAX_DETAIL_SPANS, MAX_VIEWS,
};
use std::cell::Cell;

fn capture(phase: &'static str) -> Capture {
	Capture { frame: 120, phase, views: Cell::new(0), details: Cell::new(0) }
}

fn snapshot(entries: usize, item_size: usize, capacity: usize) -> BufferSnapshot {
	BufferSnapshot {
		entries,
		item_size,
		capacity,
		destination_bytes: (capacity * item_size) as u64,
	}
}

#[test]
fn cadence_is_frame_based_and_disabled_switch_has_no_samples() {
	assert_eq!(Settings::interval(Some("0")), 1);
	assert_eq!(Settings::interval(Some("invalid")), 120);
	assert_eq!(Settings::interval(None), 120);
	let enabled = Settings { enabled: true, interval: 3 };
	assert_eq!((0..8).filter(|frame| enabled.due(*frame)).collect::<Vec<_>>(), [0, 3, 6]);
	assert!(!(0..8).any(|frame| Settings { enabled: false, interval: 1 }.due(frame)));
}

#[test]
fn complete_counts_preserve_exact_uploads_and_separate_scratch() {
	let mut census = Census::default();
	census.record_buffer(BufferKind::Instances, snapshot(10, 8, 4), snapshot(10, 8, 10), true);
	census.record_buffer(BufferKind::Metadata, snapshot(2, 12, 8), snapshot(2, 12, 8), false);
	// Lookup length includes holes: use actual vector length, not bin count or capacity.
	census.record_buffer(BufferKind::Lookup, snapshot(7, 4, 12), snapshot(7, 4, 12), false);
	census.record_buffer(BufferKind::Fan, snapshot(3, 4, 0), snapshot(3, 4, 3), true);
	assert_eq!(census.buffers.iter().map(|buffer| buffer.queue_calls).sum::<u64>(), 3);
	assert_eq!(census.buffers.iter().map(|buffer| buffer.upload_bytes).sum::<u64>(), 132);
	assert_eq!(census.buffers[2].logical_bytes, 28);
	assert_eq!(census.buffers[2].destination_bytes_after, 48);
	assert_eq!(census.buffers[3].logical_bytes, 12);
	assert_eq!((census.buffers[3].queue_calls, census.buffers[3].upload_bytes), (0, 0));
	assert_eq!(census.buffers[0].capacity_growths, 1);
	assert_eq!(census.buffers[0].observed_handle_changes, 1);
	assert_eq!(census.buffers[1].capacity_growths, 0);
}

#[test]
fn empty_vectors_do_not_count_queue_calls_or_uploads() {
	let mut census = Census::default();
	for kind in [BufferKind::Instances, BufferKind::Metadata, BufferKind::Lookup, BufferKind::Fan] {
		census.record_buffer(kind, snapshot(0, 4, 8), snapshot(0, 4, 8), false);
	}
	assert!(census
		.buffers
		.iter()
		.all(|buffer| buffer.queue_calls == 0 && buffer.upload_bytes == 0));
	assert_eq!(census.eligible_sets, 0);
}

#[test]
fn phase_samples_and_view_limits_are_independent_and_detail_cap_keeps_counts() {
	let opaque = capture("Opaque3d");
	let shadow = capture("Shadow");
	for _ in 0..MAX_VIEWS {
		assert!(opaque.view(|| "camera".into()).is_some());
	}
	assert!(opaque.view(|| "omitted".into()).is_none());
	assert!(shadow.view(|| "light".into()).is_some());
	let mut census = Census::default();
	for _ in 0..MAX_DETAIL_SPANS + 5 {
		let before = snapshot(2, 8, 2);
		let _span = opaque.buffer_span(BufferKind::Instances, before);
		census.record_buffer(BufferKind::Instances, before, before, false);
	}
	assert_eq!(opaque.details.get(), MAX_DETAIL_SPANS + 5);
	assert!(opaque.buffer_span(BufferKind::Instances, snapshot(1, 8, 1)).is_none());
	assert_eq!(census.buffers[0].queue_calls, (MAX_DETAIL_SPANS + 5) as u64);
	assert_eq!(census.buffers[0].upload_bytes, ((MAX_DETAIL_SPANS + 5) * 16) as u64);
	assert_eq!(shadow.details.get(), 0);
}
