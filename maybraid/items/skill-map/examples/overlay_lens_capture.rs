//! CPU preview of the RB-held overlay lens (idle braid, fire film, mid-uncurl).
//!
//! ```sh
//! cargo run -p maybraid-skill-map --example overlay_lens_capture --release --locked
//! ```

use std::path::PathBuf;

use maybraid_skill_map::{OverlayParams, SkillMapOverlay, SkillMapOverlayLens};

fn captures_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("captures")
}

fn main() {
	let dir = captures_dir();
	let (width, height) = (960, 540);
	write("overlay-lens-idle.png", SkillMapOverlayLens::default(), width, height);
	write("overlay-lens-fire-flash.png", SkillMapOverlayLens::peak_flash(), width, height);
	write("overlay-lens-debraid-uncurl.png", SkillMapOverlayLens::mid_uncurl(), width, height);
	println!("wrote {}", dir.display());
}

fn write(name: &str, lens: SkillMapOverlayLens, width: u32, height: u32) {
	let path = captures_dir().join(name);
	OverlayParams::from_overlay_lens(SkillMapOverlay::Expanded, lens)
		.write_preview_png(&path, width, height)
		.unwrap_or_else(|error| panic!("{}: {error}", path.display()));
	println!("{}", path.display());
}
