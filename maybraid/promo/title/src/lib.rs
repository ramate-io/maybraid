//! Scripted Maybraid promotional title sequence.
//!
//! Timeline math is pure and unit-tested. The Bevy plugin reuses the furniture
//! rockadder / cosmos shaders, the HUD Maybraid mark, and the menu header face.
//! The binary captures a fixed-timestep PNG sequence and encodes MP4 with ffmpeg.

pub mod args;
pub mod capture;
pub mod encode;
pub mod plugin;
pub mod timeline;

pub use args::Args;
pub use capture::{CaptureIo, TitleCapturePlugin};
pub use encode::{encode_png_sequence, ffmpeg_args};
pub use plugin::{
	window_plugins, SequenceClock, TitleSequenceConfig, TitleSequencePlugin, TitleSequenceSystems,
};
pub use timeline::{
	Keyframe, LabelPose, StarPose, TitleFrame, TitleSequence, TrailBead, DEFAULT_FPS,
	DEFAULT_HEIGHT, DEFAULT_WIDTH, DURATION_SECS, STAR_SIZE_PX, TRAIL_BEADS,
};
