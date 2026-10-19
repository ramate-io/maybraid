//! Baked dust playback: asset loading, timeline, local disturbance, and authored bounds.
//!
//! This is the compiled runtime for the milestone 2.1 Houdini samples. The
//! playground example is a consumer; review copies under `vfx/showcase` are
//! generated from these sources so they cannot drift.

pub mod assets;
pub mod bounds;
pub mod disturbance;
pub mod material;
pub mod playback;

pub use assets::{DustAssetError, DustBundle, DustMultiView};
pub use bounds::{DustBounds, DustObstacle, DustPlaybackFrames};
pub use disturbance::DustDisturbance;
pub use material::{DustPlaybackMaterial, DustPlaybackPlugin};
pub use playback::{DustCaptureBasis, DustExperiment, DustPlayback, DustRepresentation};

#[cfg(test)]
mod delivery_tests {
	use std::fs;
	use std::path::PathBuf;

	fn lib_root() -> PathBuf {
		PathBuf::from(env!("CARGO_MANIFEST_DIR"))
	}

	fn assert_copy(canonical: &str, delivered: &str) {
		let left = lib_root().join(canonical);
		let right = lib_root().join(delivered);
		let left_bytes = fs::read(&left)
			.unwrap_or_else(|error| panic!("missing canonical {}: {error}", left.display()));
		let right_bytes = fs::read(&right)
			.unwrap_or_else(|error| panic!("missing generated copy {}: {error}", right.display()));
		assert_eq!(left_bytes, right_bytes, "{} diverged from {}", right.display(), left.display());
	}

	#[test]
	fn generated_shaders_match_canonical() {
		assert_copy("src/dust/shader.wgsl", "../showcase/runtime/shaders/shader.wgsl");
		assert_copy(
			"src/dust/shader.wgsl",
			"../showcase/source/changed-files/maybraid/vfx/lib/src/dust/shader.wgsl",
		);
	}

	#[test]
	fn generated_runtime_sources_match_canonical() {
		for name in [
			"dust.rs",
			"dust/assets.rs",
			"dust/bounds.rs",
			"dust/disturbance.rs",
			"dust/material.rs",
			"dust/playback.rs",
		] {
			assert_copy(
				&format!("src/{name}"),
				&format!("../showcase/source/changed-files/maybraid/vfx/lib/src/{name}"),
			);
		}
		let example = lib_root().join("../playground/examples/matched_dust.rs");
		let delivered = lib_root().join(
			"../showcase/source/changed-files/maybraid/vfx/playground/examples/matched_dust.rs",
		);
		assert_eq!(
			fs::read(&example).expect("example"),
			fs::read(&delivered).expect("generated example"),
			"generated example diverged from playground/examples/matched_dust.rs"
		);
	}
}
