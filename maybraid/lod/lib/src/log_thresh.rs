//! Shared floor / flags for noisy LOD timing `info!` lines.

use std::sync::OnceLock;

const ENV_LOD_LOG_MIN_MS: &str = "LOD_LOG_MIN_MS";
const DEFAULT_LOD_LOG_MIN_MS: f64 = 1.0;

/// Min milliseconds before soft timing logs fire (`LOD_LOG_MIN_MS`, default **1.0**).
///
/// Used when a system would otherwise log every slow-ish frame (spatial queries,
/// level updates, cull scans). Eventful logs (e.g. non-zero enqueue counts) stay
/// independent of this floor.
pub fn lod_log_min_ms() -> f64 {
	static CELL: OnceLock<f64> = OnceLock::new();
	*CELL.get_or_init(|| {
		std::env::var(ENV_LOD_LOG_MIN_MS)
			.ok()
			.and_then(|raw| {
				let ms: f64 = raw.trim().parse().ok()?;
				(ms.is_finite() && ms >= 0.0).then_some(ms)
			})
			.unwrap_or(DEFAULT_LOD_LOG_MIN_MS)
	})
}

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
