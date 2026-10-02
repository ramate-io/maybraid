//! GPU device selection for `mistral.rs`.

use mistralrs::Device;

use crate::error::MistralLanguageError;

/// Where the loaded GGUF runs.
pub struct InferenceDevice;

impl InferenceDevice {
	/// Metal when that backend is compiled, otherwise CUDA if present, else CPU.
	pub fn select(force_cpu: bool) -> Result<Device, MistralLanguageError> {
		mistralrs::best_device(force_cpu).map_err(|error| {
			MistralLanguageError::load("inference device", format!("failed to open GPU: {error}"))
		})
	}

	pub fn is_gpu(device: &Device) -> bool {
		!device.is_cpu()
	}
}
