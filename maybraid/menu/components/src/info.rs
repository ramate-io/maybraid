//! Non-interactive copy that sits with a menu.

pub mod brand;
pub mod description;
pub mod hint;
pub mod objective;
pub mod text_card;

pub use brand::{BRAND_NAME, BrandModeCorner, BrandModeLine, BrandModeTitle, set_brand_mode_title};
pub use description::{TextMenuDescription, set_description_for_menu};
pub use hint::{TextMenuHint, TextMenuHintLabel, set_hint_for_menu};
pub use objective::{MenuObjective, spawn_menu_objective};
pub use text_card::{
	HUD_TEXT_CARD_FACE_PX, HudTextCard, spawn_hud_text_card, spawn_hud_text_card_label,
};
