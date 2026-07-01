//! Generic typed menu primitives for character creation UIs.
//!
//! This crate owns the renderer-independent data vocabulary. Crozon-specific
//! menu instances and trait implementations live in `crozon-character-ui-menus`.

pub mod camera_focus;
pub mod primitives;
pub mod root;
pub mod section_open;
pub mod traits;

pub use camera_focus::{CameraFocus, FocusRig};
pub use primitives::{
	AssetSingleSelect, IdentifiedAsset, MultiSelect, Section, SingleSelect, Slider,
	SwatchSingleSelect, ThumbnailCamera, VecSelect,
};
pub use root::Root;
pub use section_open::SectionOpen;
pub use traits::{
	AssetOption, AssetThumbnailDisplay, LabelOption, ListValues, StringIdentified,
	SwatchOption, ThumbnailRequest, ThumbnailSources,
};
