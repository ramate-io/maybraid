//! Adapter feature checks. Opt out instead of copying the example's global culling disable.

use bevy::prelude::*;
use bevy::render::render_resource::DownlevelFlags;
use bevy::render::renderer::RenderAdapter;

/// How the field renderer will submit work on this adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrassGpuMode {
	/// Compute compaction + `draw_indexed_indirect`.
	GpuDriven,
	/// Same selection math on CPU, direct `draw_indexed` with CPU instance counts.
	CpuSelect,
	/// Shaders/storage/indirect missing: no grass geometry.
	Unsupported,
}

/// Recorded at render-app startup. Diagnostics stay default-off.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrassGpuCapability {
	pub mode: GrassGpuMode,
}

/// `MAYBRAID_EXPERIMENT_TREE_FIELD=gpu|cpu` forces the shared vegetation GPU mode.
pub fn requested_vegetation_gpu_mode() -> Option<GrassGpuMode> {
	match std::env::var("MAYBRAID_EXPERIMENT_TREE_FIELD").as_deref() {
		Ok("gpu") => Some(GrassGpuMode::GpuDriven),
		Ok("cpu") => Some(GrassGpuMode::CpuSelect),
		_ => None,
	}
}

impl GrassGpuCapability {
	pub fn detect(adapter: &RenderAdapter) -> Self {
		let flags = adapter.get_downlevel_capabilities().flags;
		let compute = flags.contains(DownlevelFlags::COMPUTE_SHADERS);
		Self::from_compute(compute).with_requested_mode(requested_vegetation_gpu_mode())
	}

	pub fn from_compute(compute: bool) -> Self {
		Self { mode: if compute { GrassGpuMode::GpuDriven } else { GrassGpuMode::CpuSelect } }
	}

	/// Env `cpu` must win even when the adapter can run compute, or the CPU arm
	/// is encoded as `GpuDriven`.
	pub fn with_requested_mode(self, requested: Option<GrassGpuMode>) -> Self {
		let mode = match requested {
			Some(GrassGpuMode::CpuSelect) => GrassGpuMode::CpuSelect,
			Some(GrassGpuMode::GpuDriven) => {
				if self.mode == GrassGpuMode::GpuDriven {
					GrassGpuMode::GpuDriven
				} else {
					GrassGpuMode::Unsupported
				}
			}
			Some(GrassGpuMode::Unsupported) | None => self.mode,
		};
		Self { mode }
	}

	pub fn submits(self) -> bool {
		self.mode != GrassGpuMode::Unsupported
	}
}

impl Default for GrassGpuCapability {
	fn default() -> Self {
		Self { mode: GrassGpuMode::CpuSelect }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cpu_request_keeps_cpu_select_when_compute_is_available() -> anyhow::Result<()> {
		let cap = GrassGpuCapability::from_compute(true)
			.with_requested_mode(Some(GrassGpuMode::CpuSelect));
		if cap.mode != GrassGpuMode::CpuSelect {
			anyhow::bail!("expected CpuSelect, got {:?}", cap.mode);
		}
		Ok(())
	}

	#[test]
	fn gpu_request_stays_gpu_driven_when_compute_is_available() -> anyhow::Result<()> {
		let cap = GrassGpuCapability::from_compute(true)
			.with_requested_mode(Some(GrassGpuMode::GpuDriven));
		if cap.mode != GrassGpuMode::GpuDriven {
			anyhow::bail!("expected GpuDriven, got {:?}", cap.mode);
		}
		Ok(())
	}

	#[test]
	fn gpu_request_is_unsupported_without_compute() -> anyhow::Result<()> {
		let cap = GrassGpuCapability::from_compute(false)
			.with_requested_mode(Some(GrassGpuMode::GpuDriven));
		if cap.mode != GrassGpuMode::Unsupported {
			anyhow::bail!("expected Unsupported, got {:?}", cap.mode);
		}
		Ok(())
	}
}
