//! Deterministic IPA generation. Seeded by language, concept, and generator version.
//!
//! Inventories are Unicode IPA, not an ASCII stand-in. Segments may be more than
//! one code point (`tʰ`, `t͡ʃ`, `ɑː`).

use crate::concept::ConceptId;
use crate::term::{Ipa, Term};

/// Bump when coined-root phonotactics change.
pub const GENERATOR_VERSION: u32 = 2;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Phonology {
	onsets: &'static [&'static str],
	nuclei: &'static [&'static str],
	codas: &'static [&'static str],
	syllables: usize,
	/// Primary stress on the first syllable (`ˈ`).
	stress_first: bool,
}

impl Phonology {
	/// Light palatal / schwa language: short roots, easy to compound.
	pub(crate) fn compositional() -> Self {
		Self {
			onsets: &["p", "tʰ", "k", "s", "ʃ", "ɲ", "m", "l", "ɾ", "j"],
			nuclei: &["a", "e", "i", "o", "ə"],
			codas: &["", "n", "ŋ"],
			syllables: 1,
			stress_first: false,
		}
	}

	/// Heavier inventory: glottals, lateral fricative, length, initial stress.
	pub(crate) fn root_heavy() -> Self {
		Self {
			onsets: &["b", "d", "ɡ", "ʔ", "ħ", "m", "n", "ɬ", "r", "t͡ʃ"],
			nuclei: &["ɑ", "i", "o", "u", "e", "ɑː"],
			codas: &["", "n", "ʁ", "ʃ"],
			syllables: 2,
			stress_first: true,
		}
	}

	pub(crate) fn root(self, seed: u64) -> Term {
		let mut rng = seed;
		let mut out = String::new();
		if self.stress_first {
			out.push('ˈ');
		}
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
	Term::new(strip_stress(&format!("{first}{last}")))
}

pub(crate) fn blend(left: &str, right: &str) -> Term {
	let onset = onset_of(left);
	let remainder = from_first_nucleus(right);
	Term::new(strip_stress(&format!("{onset}{remainder}")))
}

fn strip_stress(ipa: &str) -> String {
	ipa.chars().filter(|c| !is_stress(*c)).collect()
}

fn first_syllable(ipa: &str) -> &str {
	split_syllables(ipa).into_iter().next().unwrap_or(ipa)
}

fn last_syllable(ipa: &str) -> &str {
	split_syllables(ipa).into_iter().next_back().unwrap_or(ipa)
}

fn split_syllables(ipa: &str) -> Vec<&str> {
	let points: Vec<(usize, char)> = ipa.char_indices().collect();
	if points.is_empty() {
		return Vec::new();
	}
	let nuclei: Vec<usize> = points
		.iter()
		.enumerate()
		.filter(|(_, (_, c))| is_nucleus(*c))
		.map(|(i, _)| i)
		.collect();
	if nuclei.is_empty() {
		return vec![ipa];
	}
	let mut starts_byte = vec![0];
	for &nucleus in nuclei.iter().skip(1) {
		let cut = onset_start_index(&points, nucleus);
		let byte = points[cut].0;
		if byte > *starts_byte.last().unwrap_or(&0) {
			starts_byte.push(byte);
		}
	}
	starts_byte.push(ipa.len());
	starts_byte
		.windows(2)
		.map(|w| &ipa[w[0]..w[1]])
		.filter(|s| !s.is_empty())
		.collect()
}

fn onset_start_index(points: &[(usize, char)], nucleus: usize) -> usize {
	let mut i = nucleus;
	while i > 0 && is_modifier(points[i - 1].1) {
		i -= 1;
	}
	if i > 0 && is_tie(points[i - 1].1) {
		i -= 1;
		if i > 0 && !is_nucleus(points[i - 1].1) && !is_length(points[i - 1].1) {
			i -= 1;
		}
	} else if i > 0
		&& !is_nucleus(points[i - 1].1)
		&& !is_length(points[i - 1].1)
		&& !is_stress(points[i - 1].1)
	{
		i -= 1;
	}
	if i > 0 && is_stress(points[i - 1].1) {
		i -= 1;
	}
	i
}

fn onset_of(ipa: &str) -> &str {
	let stripped = ipa.trim_start_matches(|c: char| is_stress(c));
	match stripped.char_indices().find(|(_, c)| is_nucleus(*c)) {
		Some((0, _)) => &stripped[..stripped.chars().next().map(|c| c.len_utf8()).unwrap_or(0)],
		Some((i, _)) => &stripped[..i],
		None => stripped,
	}
}

fn from_first_nucleus(ipa: &str) -> &str {
	let stripped = ipa.trim_start_matches(|c: char| is_stress(c));
	match stripped.char_indices().find(|(_, c)| is_nucleus(*c)) {
		Some((i, _)) => &stripped[i..],
		None => stripped,
	}
}

fn is_nucleus(c: char) -> bool {
	matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'ə' | 'ɑ' | 'ɔ' | 'ɛ' | 'ɪ' | 'ʊ' | 'æ' | 'ɨ')
}

fn is_length(c: char) -> bool {
	c == 'ː'
}

fn is_stress(c: char) -> bool {
	matches!(c, 'ˈ' | 'ˌ')
}

fn is_modifier(c: char) -> bool {
	matches!(c, 'ʰ' | 'ʷ' | 'ʲ' | 'ˤ') || is_length(c)
}

fn is_tie(c: char) -> bool {
	c == '͡'
}

#[cfg(test)]
mod tests {
	use super::{blend, compound, split_syllables, Phonology};

	#[test]
	fn compositional_roots_use_non_ascii_ipa() {
		let interesting = (0_u64..48).any(|i| {
			let term = Phonology::compositional().root(i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
			term.ipa
				.as_str()
				.chars()
				.any(|c| matches!(c, 'ʃ' | 'ə' | 'ŋ' | 'ɲ' | 'ɾ' | 'ʰ'))
		});
		assert!(interesting, "compositional inventory should emit marked IPA");
	}

	#[test]
	fn root_heavy_roots_use_stress_and_length_or_glottals() {
		let term = Phonology::root_heavy().root(0x5555_6666_7777_8888);
		let ipa = term.ipa.as_str();
		assert!(ipa.contains('ˈ'), "root-heavy roots should carry primary stress, got {ipa}");
		assert!(
			ipa.chars().any(|c| matches!(c, 'ɑ' | 'ː' | 'ʔ' | 'ħ' | 'ɬ' | 'ʁ' | 'ɡ' | '͡')),
			"expected a heavier IPA segment in {ipa}"
		);
	}

	#[test]
	fn syllables_split_on_unicode_nuclei() {
		assert_eq!(split_syllables("ʃəŋ"), vec!["ʃəŋ"]);
		assert_eq!(split_syllables("ˈt͡ʃɑːɬir"), vec!["ˈt͡ʃɑː", "ɬir"]);
		assert_eq!(compound("ʃəŋ", "tʰan").ipa.as_str(), "ʃəŋtʰan");
		assert_eq!(blend("ʃəŋ", "ˈɡɑːʁ").ipa.as_str(), "ʃɑːʁ");
	}
}
