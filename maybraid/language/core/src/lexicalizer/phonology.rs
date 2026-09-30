//! Deterministic IPA generation. Seeded by language, concept, and generator version.

use crate::concept::ConceptId;
use crate::term::{Ipa, Term};

/// Bump when coined-root phonotactics change.
pub const GENERATOR_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Phonology {
	onsets: &'static [&'static str],
	nuclei: &'static [&'static str],
	codas: &'static [&'static str],
	syllables: usize,
}

impl Phonology {
	pub(crate) fn compositional() -> Self {
		Self {
			onsets: &["p", "t", "k", "s", "n", "m", "l", "r", "w"],
			nuclei: &["a", "e", "i", "o", "u"],
			codas: &["", "n"],
			syllables: 1,
		}
	}

	pub(crate) fn root_heavy() -> Self {
		Self {
			onsets: &["b", "d", "g", "h", "n", "m", "l", "r", "t", "k"],
			nuclei: &["a", "i", "o", "u", "e"],
			codas: &["", "n", "r"],
			syllables: 2,
		}
	}

	pub(crate) fn root(self, seed: u64) -> Term {
		let mut rng = seed;
		let mut out = String::new();
		for _ in 0..self.syllables {
			rng = mix(rng);
			out.push_str(self.onsets[rng as usize % self.onsets.len()]);
			rng = mix(rng);
			out.push_str(self.nuclei[rng as usize % self.nuclei.len()]);
			rng = mix(rng);
			out.push_str(self.codas[rng as usize % self.codas.len()]);
		}
		Term::new(Ipa(out))
	}
}

pub fn mix(mut x: u64) -> u64 {
	x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
	x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
	x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
	x ^ (x >> 31)
}

pub(crate) fn concept_seed(language_seed: u64, concept: ConceptId) -> u64 {
	mix(language_seed ^ mix(concept.as_u64()) ^ u64::from(GENERATOR_VERSION))
}

pub(crate) fn coined_root(language_seed: u64, concept: ConceptId, phonology: Phonology) -> Term {
	phonology.root(concept_seed(language_seed, concept))
}

pub(crate) fn compound(left: &str, right: &str) -> Term {
	let first = first_syllable(left);
	let last = last_syllable(right);
	Term::new(format!("{first}{last}"))
}

pub(crate) fn blend(left: &str, right: &str) -> Term {
	let onset = onset_of(left);
	let remainder = from_first_vowel(right);
	Term::new(format!("{onset}{remainder}"))
}

fn first_syllable(ipa: &str) -> &str {
	split_syllables(ipa).into_iter().next().unwrap_or(ipa)
}

fn last_syllable(ipa: &str) -> &str {
	split_syllables(ipa).into_iter().next_back().unwrap_or(ipa)
}

fn split_syllables(ipa: &str) -> Vec<&str> {
	let bytes = ipa.as_bytes();
	let mut cuts = vec![0];
	let mut i = 1;
	while i < bytes.len() {
		if !is_vowel(bytes[i - 1]) && is_vowel(bytes[i]) && i > 1 {
			let mut cut = i;
			if cut > 0 && !is_vowel(bytes[cut - 1]) {
				cut -= 1;
			}
			if cut > cuts[cuts.len() - 1] {
				cuts.push(cut);
			}
		}
		i += 1;
	}
	cuts.push(bytes.len());
	cuts.windows(2).map(|w| &ipa[w[0]..w[1]]).filter(|s| !s.is_empty()).collect()
}

fn onset_of(ipa: &str) -> &str {
	let end = ipa.find(|c: char| "aeiou".contains(c)).unwrap_or(ipa.len());
	if end == 0 {
		&ipa[..ipa.len().min(1)]
	} else {
		&ipa[..end]
	}
}

fn from_first_vowel(ipa: &str) -> &str {
	match ipa.find(|c: char| "aeiou".contains(c)) {
		Some(i) => &ipa[i..],
		None => ipa,
	}
}

fn is_vowel(b: u8) -> bool {
	matches!(b, b'a' | b'e' | b'i' | b'o' | b'u')
}
