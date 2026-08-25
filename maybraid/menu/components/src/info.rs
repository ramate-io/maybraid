//! Non-interactive copy that sits with a menu.

pub mod description;
pub mod hint;

pub use description::{TextMenuDescription, set_description_for_menu};
pub use hint::{TextMenuHint, TextMenuHintLabel, set_hint_for_menu};
