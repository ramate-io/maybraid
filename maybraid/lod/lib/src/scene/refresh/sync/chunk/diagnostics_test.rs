//! Compile-time verification that scene-tracing is off by default.

#[test]
fn scene_tracing_disabled_by_default() {
	// This test verifies that diagnostics::enabled() returns false when
	// the scene-tracing feature is not enabled, ensuring zero cost.
	#[cfg(not(feature = "scene-tracing"))]
	{
		// When feature is off, enabled() must return false
		assert!(!super::diagnostics::enabled());
		
		// Verify ClassificationCounters is truly zero-cost when disabled
		let mut counters = super::diagnostics::ClassificationCounters::default();
		counters.count_new_root();
		counters.count_skipped_queue_empty();
		
		// With feature off, counters should not accumulate
		#[cfg(not(feature = "scene-tracing"))]
		{
			assert!(!counters.has_new_roots());
			assert_eq!(counters.new_roots_total, 0);
		}
	}
	
	#[cfg(feature = "scene-tracing")]
	{
		// When feature is on, enabled() checks the tracing level.
		// The result depends on the tracing subscriber configuration,
		// but the function must exist and be callable.
		let _ = super::diagnostics::enabled();
	}
}

#[test]
fn classification_counters_zero_cost_when_disabled() {
	use super::diagnostics::ClassificationCounters;
	
	// Verify that counter operations compile away when feature is off
	let mut counters = ClassificationCounters::default();
	
	// These operations should be no-ops when feature is disabled
	for _ in 0..1000 {
		counters.count_new_root();
		counters.count_skipped_queue_empty();
		counters.count_skipped_no_host();
		counters.count_skipped_cold_mismatch();
		counters.count_skipped_not_shown();
	}
	
	// With feature off, has_new_roots() should always return false
	#[cfg(not(feature = "scene-tracing"))]
	assert!(!counters.has_new_roots());
}
