//! English nouns and adjectives implied by kind enums and stamp families.

/// Durham stamp / hydro families that names may read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeographicKind {
	Massif,
	Plateau,
	Canyon,
	Lake,
	Bog,
	Stream,
	PocketWater,
}

impl GeographicKind {
	pub const ALL: &[Self] = &[
		Self::Massif,
		Self::Plateau,
		Self::Canyon,
		Self::Lake,
		Self::Bog,
		Self::Stream,
		Self::PocketWater,
	];

	pub fn english(self) -> Vec<String> {
		english_words(&format!("{self:?}"))
	}
}

/// Split a kebab or PascalCase ident into lowercase English atoms.
///
/// `RollingOaks` → `rolling`, `oaks`. `old-city-market` → `old`, `city`, `market`.
pub fn english_words(ident: &str) -> Vec<String> {
	let mut words = Vec::new();
	let mut current = String::new();
	for ch in ident.chars() {
		if ch == '-' || ch == '_' || ch.is_whitespace() {
			push_word(&mut words, &mut current);
			continue;
		}
		if ch.is_uppercase() && !current.is_empty() {
			push_word(&mut words, &mut current);
		}
		current.extend(ch.to_lowercase());
	}
	push_word(&mut words, &mut current);
	words.retain(|word| !matches!(word.as_str(), "none" | "empty"));
	words
}

fn push_word(words: &mut Vec<String>, current: &mut String) {
	if current.is_empty() {
		return;
	}
	words.push(std::mem::take(current));
}
