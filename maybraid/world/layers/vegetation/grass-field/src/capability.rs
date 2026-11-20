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

impl GrassGpuCapability {
	pub fn detect(adapter: &RenderAdapter) -> Self {
		let flags = adapter.get_downlevel_capabilities().flags;
		let compute = flags.contains(DownlevelFlags::COMPUTE_SHADERS);
		let mode = if compute { GrassGpuMode::GpuDriven } else { GrassGpuMode::CpuSelect };
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
