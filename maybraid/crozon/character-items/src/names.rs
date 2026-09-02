//! Display names for owned items.
//!
//! Each mesh contributes nouns, each look and color adjectives. A stable hash of
//! the triple picks one word from each list so the same item always has the same
//! name (`Celestial Red Tide Joggers`).

use crate::{ClothingMaterial, ClothingMesh, ItemColor};

/// Material adjective, then color adjective, then clothing noun.
pub fn hashed_item_name(
	mesh: ClothingMesh,
	material: ClothingMaterial,
	color: ItemColor,
) -> String {
	let hash = mix(mix(mix(0xC0FF_EE42_D00D_A5E5, mesh.label()), material.label()), color.label());
	format!(
		"{} {} {}",
		pick(material.adjectives(), hash),
		pick(color.adjectives(), hash >> 17),
		pick(mesh.nouns(), hash >> 33),
	)
}

fn mix(seed: u64, label: &str) -> u64 {
	let mut hash = seed ^ 0x9E37_79B9_7F4A_7C15;
	for byte in label.as_bytes() {
		hash = hash.wrapping_mul(0x0100_0000_01B3).wrapping_add(*byte as u64);
	}
	hash ^ hash >> 33
}

fn pick<'a>(words: &'a [&'a str], hash: u64) -> &'a str {
	words[(hash as usize) % words.len()]
}

impl ClothingMesh {
	/// Nouns a rolled item of this mesh may be called.
	pub const fn nouns(self) -> &'static [&'static str] {
		match self {
			Self::TankTop => &["Tank Top", "Tank", "Undershirt", "Singlet", "Vest"],
			Self::Tunic => &["Tunic", "Smock", "Tabard", "Shift", "Blouse"],
			Self::LongDress => &["Long Dress", "Gown", "Train", "Evening Dress", "Robe Dress"],
			Self::ShortDress => &["Short Dress", "Frock", "Sundress", "Shift Dress", "Mini"],
			Self::FittedCoat => &["Fitted Coat", "Coat", "Jacket", "Peacoat", "Blazer"],
			Self::RobeCoat => &["Robe Coat", "Overrobe", "Mantle", "Cloak Coat", "Greatcoat"],
			Self::Robe => &["Robe", "Vestment", "Wrap", "Kimono", "Cassock"],
			Self::Pants => &["Pants", "Trousers", "Slacks", "Bottoms", "Breeches"],
			Self::KneeHighBoots => {
				&["Knee-High Boots", "Boots", "Stompers", "Waders", "High Boots"]
			}
			Self::HaremPants => &["Harem Pants", "Pants", "Pantaloons", "Sweats", "Joggers"],
			Self::HaremPantsUpper => {
				&["Harem Top", "Balloon Top", "Drop Crotch", "Peg Top", "Harem Rise"]
			}
			Self::HaremPantsLowerWrap => {
				&["Lower Wrap", "Ankle Wrap", "Leg Wraps", "Bindings", "Cuffs"]
			}
		}
	}
}

impl ClothingMaterial {
	/// Adjectives a rolled item of this look may be called.
	pub const fn adjectives(self) -> &'static [&'static str] {
		match self {
			Self::SpaceSuit => &["Vacuum", "Orbital", "Pressurized", "Starfarer", "Sealed"],
			Self::Tattered => &["Tattered", "Ragged", "Worn", "Threadbare", "Frayed"],
			Self::Hawaiian => &["Tropical", "Floral", "Island", "Hibiscus", "Vacation"],
			Self::Cloth => &["Cloth", "Woven", "Plain", "Humble", "Cotton"],
			Self::Scales => &["Scaled", "Serpent", "Iridescent", "Draconic", "Plated"],
			Self::WizardsVeins => &["Arcane", "Celestial", "Bearded", "Misty", "Runed"],
			Self::Glitter => &["Glittering", "Sparkling", "Sequined", "Dazzling", "Shimmering"],
		}
	}
}

impl ItemColor {
	/// Adjectives a rolled item of this color may be called.
	pub const fn adjectives(self) -> &'static [&'static str] {
		match self {
			Self::Natural => &["Natural", "Bare", "Earth", "Undyed", "Raw"],
			Self::Warm => &["Warm", "Amber", "Sunset", "Honeyed", "Bronze"],
			Self::Cool => &["Cool", "Frost", "Slate", "Steel", "Winter"],
			Self::Dark => &["Dark", "Shadow", "Midnight", "Soot", "Umbral"],
			Self::Light => &["Light", "Pale", "Ivory", "Bleached", "Dawn"],
			Self::Red => &["Red", "Crimson", "Bloody", "Red Tide", "Scarlet"],
			Self::Blue => &["Blue", "Azure", "Cobalt", "Navy", "Cerulean"],
			Self::Green => &["Green", "Verdant", "Moss", "Emerald", "Forest"],
			Self::Gold => &["Gold", "Golden", "Auric", "Aureate", "Sunlit"],
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn name_is_stable_for_a_triple() {
		let a = hashed_item_name(
			ClothingMesh::HaremPants,
			ClothingMaterial::WizardsVeins,
			ItemColor::Red,
		);
		let b = hashed_item_name(
			ClothingMesh::HaremPants,
			ClothingMaterial::WizardsVeins,
			ItemColor::Red,
		);
		assert_eq!(a, b);
		let material = ClothingMaterial::WizardsVeins.adjectives();
		let color = ItemColor::Red.adjectives();
		let noun = ClothingMesh::HaremPants.nouns();
		assert!(material.iter().any(|word| a.starts_with(*word)));
		assert!(noun.iter().any(|word| a.ends_with(*word)));
		assert!(color.iter().any(|word| a.contains(*word)));
	}

	#[test]
	fn every_catalog_entry_has_words() {
		for mesh in ClothingMesh::VALUES {
			assert!(!mesh.nouns().is_empty(), "{}", mesh.label());
		}
		for material in ClothingMaterial::VALUES {
			assert!(!material.adjectives().is_empty(), "{}", material.label());
		}
		for color in ItemColor::VALUES {
			assert!(!color.adjectives().is_empty(), "{}", color.label());
		}
	}
}
