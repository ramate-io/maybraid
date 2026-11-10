use crate::upload::{UploadMode, UploadState};

#[test]
fn first_then_unchanged_and_same_length_mutations() {
	let mut state = UploadState::default();
	assert!(state.needs_upload(UploadMode::Dirty, Some(1), 2));
	state.queued(Some(1), 2);
	assert!(!state.needs_upload(UploadMode::Dirty, Some(1), 2));
	state.changed();
	assert!(state.needs_upload(UploadMode::Dirty, Some(1), 2));
	state.queued(Some(1), 2);
	assert!(!state.needs_upload(UploadMode::Dirty, Some(1), 2));
	assert!(state.needs_upload(UploadMode::Force, Some(1), 2));
}

#[test]
fn missing_replaced_resized_and_device_destinations_need_uploads() {
	let mut state = UploadState::default();
	state.queued(Some(11), 3);
	assert!(state.needs_upload(UploadMode::Dirty, None, 3));
	assert!(state.needs_upload(UploadMode::Dirty, Some(12), 3));
	assert!(state.needs_upload(UploadMode::Dirty, Some(11), 4));
	state.invalidate_destination();
	assert!(state.needs_upload(UploadMode::Dirty, Some(11), 3));
}

#[test]
fn skipped_empty_and_wrapped_generation_never_acknowledge_dirty_data() {
	let mut state = UploadState::default();
	state.queued(Some(7), 1);
	state.changed();
	assert!(!state.needs_upload(UploadMode::Dirty, Some(7), 0));
	state.queued(Some(7), 0);
	assert!(state.needs_upload(UploadMode::Dirty, Some(7), 1));
	state.revision = u64::MAX;
	state.queued(Some(7), 1);
	state.changed();
	assert_eq!(state.revision(), 0);
	assert!(state.needs_upload(UploadMode::Dirty, Some(7), 1));
}

#[test]
fn process_mode_defaults_to_stock_writes() {
	assert_eq!(UploadMode::parse(None), UploadMode::Force);
	assert_eq!(UploadMode::parse(Some("force")), UploadMode::Force);
	assert_eq!(UploadMode::parse(Some("unknown")), UploadMode::Force);
	assert_eq!(UploadMode::parse(Some("dirty")), UploadMode::Dirty);
}

#[cfg(feature = "diagnostics")]
#[test]
fn independent_shadow_catches_same_length_payload_changes() {
	use crate::upload::SubmittedBytes;
	let mut shadow = SubmittedBytes::default();
	assert!(!shadow.matches(&[1, 2]));
	shadow.submitted(&[1, 2]);
	assert!(shadow.matches(&[1, 2]));
	assert!(!shadow.matches(&[2, 1]));
	assert!(!shadow.matches(&[1]));
}
