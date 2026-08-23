//! Non-interactive copy that sits with a menu.

pub mod description;
pub mod hint;

pub use description::{set_description_for_menu, TextMenuDescription};
pub use hint::{set_hint_for_menu, TextMenuHint, TextMenuHintLabel};
