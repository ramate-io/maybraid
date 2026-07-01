//! Generic typed menu primitives for character creation UIs.
//!
//! This crate owns the renderer-independent data vocabulary. Crozon-specific
//! option traits and enum implementations live in `crozon-character-ui-menus`.

pub mod camera_focus;
pub mod primitives;

pub use camera_focus::CameraFocus;
pub use primitives::{
	AssetSingleSelect, IdentifiedAsset, MultiSelect, Section, SingleSelect, Slider,
	SwatchSingleSelect, ThumbnailCamera, VecSelect,
};
