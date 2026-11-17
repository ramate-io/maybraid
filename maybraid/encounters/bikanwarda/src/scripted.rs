//! Scripted<T> pattern for authored selection and configuration.
//!
//! Wraps authored instances that flow through the same generation and
//! presentation paths as future generated content. Deterministic from session
//! roots only.

use std::sync::Arc;

/// Authored selection and configuration that feeds shared generation.
///
/// Scripted values pass through HCSG as `Arc<Scripted<T>>` so generated and
/// authored paths share the same runtime types. Future procedural generators
/// can produce `T` directly without needing a wrapper.
#[derive(Clone, Debug, PartialEq)]
pub struct Scripted<T> {
	/// The authored configuration or selection.
	pub value: T,
}

impl<T> Scripted<T> {
	pub fn new(value: T) -> Self {
		Self { value }
	}

	pub fn into_arc(self) -> Arc<Self> {
		Arc::new(self)
	}
}

impl<T> From<T> for Scripted<T> {
	fn from(value: T) -> Self {
		Self::new(value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone, Debug, PartialEq)]
	struct TestConfig {
		name: String,
		scale: f32,
	}

	#[test]
	fn scripted_wraps_authored_config() {
		let config = TestConfig { name: "test".to_string(), scale: 2.0 };
		let scripted = Scripted::new(config.clone());
		assert_eq!(scripted.value, config);
	}

	#[test]
	fn scripted_converts_to_arc() {
		let config = TestConfig { name: "test".to_string(), scale: 2.0 };
		let scripted = Scripted::new(config.clone());
		let arc_scripted = scripted.into_arc();
		assert_eq!(arc_scripted.value, config);
	}
}
