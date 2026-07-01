//! Generic typed menu primitives for character creation UIs.
//!
//! This crate owns the renderer-independent data vocabulary. Crozon-specific
//! option traits and enum implementations live in `crozon-character-ui-menus`.

pub mod camera_focus;
pub mod event;
pub mod primitives;

pub use camera_focus::CameraFocus;
pub use event::{AssetValue, CharacterField, MenuEvent, SectionId, SwatchValue};
pub use primitives::{
	AssetSingleSelect, IdentifiedAsset, MultiSelect, Section, SingleSelect, Slider,
	SwatchSingleSelect, VecSelect,
};
