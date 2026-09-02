//! Non-interactive copy that sits with a menu.

pub mod brand;
pub mod description;
pub mod hint;

pub use brand::{BRAND_NAME, BrandModeCorner, BrandModeLine, BrandModeTitle, set_brand_mode_title};
pub use description::{TextMenuDescription, set_description_for_menu};
pub use hint::{TextMenuHint, TextMenuHintLabel, set_hint_for_menu};
