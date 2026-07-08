//! Crozon-specific typed character menus.

pub mod character;
pub mod characters;
pub mod event;
pub mod focus;
pub mod section_open;
pub mod shared;

pub use character::{CharacterMenu, ConceptSpecies, SectionOpenState};
pub use characters::{
	braidman::BraidmanMenu, brenal::BrenalMenu, brodler::BrodlerMenu, caole::CaoleMenu,
	dui::DuiMenu, lero::LeroMenu, mygr::MygrMenu, spibmom::SpibmomMenu, wumbus::WumbusMenu,
};
pub use event::{AssetValue, CharacterField, MenuEvent, SectionId, SwatchValue};
pub use focus::BODY_FOCUS;

pub use character_ui_menu::{AssetOption, LabelOption, ListValues, StringIdentified, SwatchOption};

pub(crate) fn cycle_value<T: ListValues>(value: T, delta: i32) -> T {
	let values = T::values();
	let current = values.iter().position(|candidate| *candidate == value).unwrap_or(0);
	let next = (current as i32 + delta).rem_euclid(values.len() as i32) as usize;
	values[next]
}

#[cfg(test)]
mod tests;
