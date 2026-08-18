//! Optional verbose LOD timing (`LOD_CHUNK_TRACE=1` or feature `chunk-trace`).

use std::sync::OnceLock;

/// Verbose per-job / per-frame chunk fulfill logs (`LOD_CHUNK_TRACE=1` or feature `chunk-trace`).
#[inline]
pub fn lod_chunk_trace() -> bool {
	#[cfg(feature = "chunk-trace")]
	{
		true
	}
	#[cfg(not(feature = "chunk-trace"))]
	{
		static CELL: OnceLock<bool> = OnceLock::new();
		*CELL.get_or_init(|| {
			std::env::var("LOD_CHUNK_TRACE")
				.map(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "yes"))
				.unwrap_or(false)
		})
	}
}
