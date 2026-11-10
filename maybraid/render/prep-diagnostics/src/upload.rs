//! Policy for the explicitly patched native binned inputs; no GPU types or hashing.
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UploadMode {
	Force,
	Dirty,
}

impl UploadMode {
	pub fn parse(raw: Option<&str>) -> Self {
		if raw == Some("dirty") {
			Self::Dirty
		} else {
			Self::Force
		}
	}

	pub fn runtime() -> Self {
		static MODE: OnceLock<UploadMode> = OnceLock::new();
		*MODE.get_or_init(|| {
			Self::parse(std::env::var("MAYBRAID_NATIVE_BINNED_UPLOADS").ok().as_deref())
		})
	}

	pub fn name(self) -> &'static str {
		match self {
			Self::Force => "force",
			Self::Dirty => "dirty",
		}
	}
}

#[derive(Clone, Copy, Default, Debug)]
pub struct UploadOutcome {
	pub queued: bool,
	pub queued_equal: bool,
	pub skipped: bool,
	pub shadow_checked: bool,
	pub shadow_mismatch: bool,
}

/// Mutations invalidate BEFORE lending mutable access. Dropping a pending
/// submission on every mutation also prevents generation-wrap aliasing.
#[derive(Debug)]
pub struct UploadState<D> {
	revision: u64,
	queued: Option<(u64, D, usize)>,
}

impl<D> Default for UploadState<D> {
	fn default() -> Self {
		Self { revision: 0, queued: None }
	}
}

impl<D: Copy + Eq> UploadState<D> {
	pub fn changed(&mut self) {
		self.revision = self.revision.wrapping_add(1);
		self.queued = None;
	}

	pub fn invalidate_destination(&mut self) {
		self.queued = None;
	}

	pub fn needs_upload(&self, mode: UploadMode, destination: Option<D>, len: usize) -> bool {
		if len == 0 {
			return false;
		}
		mode == UploadMode::Force
			|| destination.is_none()
			|| self.queued != destination.map(|destination| (self.revision, destination, len))
	}

	/// Records a full CPU queue request after its Rust API returned. This does
	/// not claim GPU completion or make a missing/empty destination valid.
	pub fn queued(&mut self, destination: Option<D>, len: usize) {
		if len > 0 {
			self.queued = destination.map(|destination| (self.revision, destination, len));
		}
	}

	pub fn revision(&self) -> u64 {
		self.revision
	}
}

/// Independent reference of the bytes actually submitted, diagnostic-only.
#[cfg(feature = "diagnostics")]
#[derive(Default)]
pub struct SubmittedBytes {
	bytes: Option<Vec<u8>>,
}

#[cfg(feature = "diagnostics")]
impl SubmittedBytes {
	pub fn matches(&self, bytes: &[u8]) -> bool {
		self.bytes.as_deref() == Some(bytes)
	}
	pub fn submitted(&mut self, bytes: &[u8]) {
		let submitted = self.bytes.get_or_insert_with(Vec::new);
		submitted.clear();
		submitted.extend_from_slice(bytes);
	}
}

#[cfg(test)]
mod tests;
