//! Parser for Princeton WordNet `index.*` / `data.*` files.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::concept::{Concept, ConceptId, ConceptRelation, Pos, RelationKind};
use crate::error::LanguageError;
use crate::wordnet::normalize_lemma;

/// Directory containing the crate's vendored WordNet 3.1 extract.
pub fn bundled_dict_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("wordnet/dict")
}

#[derive(Clone, Debug)]
struct IndexEntry {
	offsets: Vec<u32>,
}

#[derive(Clone, Debug)]
struct Synset {
	id: ConceptId,
	lemmas: Vec<String>,
	relations: Vec<ConceptRelation>,
}

/// In-memory view of a WordNet dict directory.
#[derive(Clone, Debug)]
pub struct WordNetDict {
	synsets: HashMap<ConceptId, Synset>,
	lemmas: HashMap<(String, Pos), IndexEntry>,
}

impl WordNetDict {
	pub fn from_dir(path: impl AsRef<Path>) -> Result<Self, LanguageError> {
		let root = path.as_ref();
		let mut dict = Self { synsets: HashMap::new(), lemmas: HashMap::new() };
		for pos in [Pos::Noun, Pos::Verb, Pos::Adjective, Pos::Adverb] {
			dict.load_data(root, pos)?;
			dict.load_index(root, pos)?;
		}
		Ok(dict)
	}

	pub fn concept(&self, id: ConceptId) -> Option<Concept> {
		self.synsets
			.get(&id)
			.map(|synset| Concept { id, english_glosses: synset.lemmas.clone() })
	}

	pub fn resolve_lemma(&self, lemma: &str) -> Vec<ConceptId> {
		let lemma = normalize_lemma(lemma);
		let mut ids = Vec::new();
		for pos in [Pos::Noun, Pos::Verb, Pos::Adjective, Pos::AdjectiveSatellite, Pos::Adverb] {
			if let Some(entry) = self.lemmas.get(&(lemma.clone(), pos)) {
				for offset in &entry.offsets {
					ids.push(ConceptId::wordnet(*offset, pos));
				}
			}
		}
		ids
	}

	pub fn sense(&self, lemma: &str, pos: Pos, sense: usize) -> Option<ConceptId> {
		self.lemmas
			.get(&(normalize_lemma(lemma), canonical_index_pos(pos)))
			.and_then(|entry| entry.offsets.get(sense).copied())
			.map(|offset| ConceptId::wordnet(offset, pos))
	}

	pub fn has_lemma(&self, lemma: &str, pos: Pos) -> bool {
		self.lemmas.contains_key(&(normalize_lemma(lemma), canonical_index_pos(pos)))
	}

	pub fn relations(&self, id: ConceptId) -> Vec<ConceptRelation> {
		self.synsets.get(&id).map(|synset| synset.relations.clone()).unwrap_or_default()
	}

	fn load_data(&mut self, root: &Path, pos: Pos) -> Result<(), LanguageError> {
		let path = root.join(format!("data.{}", pos.dict_stem()));
		if !path.exists() {
			return Ok(());
		}
		let text = read_to_string(&path)?;
		for (line_no, line) in text.lines().enumerate() {
			if line.is_empty() || line.starts_with(' ') || line.starts_with('\t') {
				continue;
			}
			let synset = parse_data_line(line, &path, line_no)?;
			self.synsets.insert(synset.id, synset);
		}
		Ok(())
	}

	fn load_index(&mut self, root: &Path, file_pos: Pos) -> Result<(), LanguageError> {
		let path = root.join(format!("index.{}", file_pos.dict_stem()));
		if !path.exists() {
			return Ok(());
		}
		let text = read_to_string(&path)?;
		for (line_no, line) in text.lines().enumerate() {
			if line.is_empty() || line.starts_with(' ') || line.starts_with('\t') {
				continue;
			}
			let (lemma, entry_pos, entry) = parse_index_line(line, &path, line_no)?;
			let key_pos =
				if entry_pos == Pos::AdjectiveSatellite { Pos::Adjective } else { entry_pos };
			self.lemmas.insert((lemma, key_pos), entry);
		}
		Ok(())
	}
}

fn canonical_index_pos(pos: Pos) -> Pos {
	match pos {
		Pos::AdjectiveSatellite => Pos::Adjective,
		other => other,
	}
}

fn read_to_string(path: &Path) -> Result<String, LanguageError> {
	fs::read_to_string(path)
		.map_err(|source| LanguageError::Io { path: path.display().to_string(), source })
}

fn parse_index_line(
	line: &str,
	path: &Path,
	line_no: usize,
) -> Result<(String, Pos, IndexEntry), LanguageError> {
	let parts: Vec<&str> = line.split_whitespace().collect();
	if parts.len() < 6 {
		return Err(parse_err(path, line_no, "index line too short"));
	}
	let lemma = normalize_lemma(parts[0]);
	let pos = Pos::from_letter(parts[1].chars().next().unwrap_or('?'))
		.ok_or_else(|| parse_err(path, line_no, "unknown index POS"))?;
	let p_cnt: usize = parts[3]
		.parse()
		.map_err(|_| parse_err(path, line_no, "invalid pointer count"))?;
	let rest = 4 + p_cnt;
	if parts.len() < rest + 2 {
		return Err(parse_err(path, line_no, "index offsets missing"));
	}
	let mut offsets = Vec::new();
	for token in &parts[rest + 2..] {
		let offset =
			token.parse().map_err(|_| parse_err(path, line_no, "invalid synset offset"))?;
		offsets.push(offset);
	}
	Ok((lemma, pos, IndexEntry { offsets }))
}

fn parse_data_line(line: &str, path: &Path, line_no: usize) -> Result<Synset, LanguageError> {
	let (record, gloss) = match line.split_once('|') {
		Some((record, gloss)) => (record, gloss.trim()),
		None => (line, ""),
	};
	let parts: Vec<&str> = record.split_whitespace().collect();
	if parts.len() < 6 {
		return Err(parse_err(path, line_no, "data line too short"));
	}
	let offset: u32 = parts[0]
		.parse()
		.map_err(|_| parse_err(path, line_no, "invalid synset offset"))?;
	let ss_type = parts[2].chars().next().unwrap_or('n');
	let pos =
		Pos::from_letter(ss_type).ok_or_else(|| parse_err(path, line_no, "unknown ss_type"))?;
	let w_cnt = u32::from_str_radix(parts[3], 16)
		.map_err(|_| parse_err(path, line_no, "invalid word count"))? as usize;
	let mut i = 4;
	let mut lemmas = Vec::new();
	for _ in 0..w_cnt {
		if i + 1 >= parts.len() {
			return Err(parse_err(path, line_no, "truncated lemma list"));
		}
		lemmas.push(normalize_lemma(parts[i]));
		i += 2;
	}
	if i >= parts.len() {
		return Err(parse_err(path, line_no, "missing pointer count"));
	}
	let p_cnt: usize = parts[i]
		.parse()
		.map_err(|_| parse_err(path, line_no, "invalid pointer count"))?;
	i += 1;
	let id = ConceptId::wordnet(offset, pos);
	let mut relations = Vec::new();
	for _ in 0..p_cnt {
		if i + 3 >= parts.len() {
			return Err(parse_err(path, line_no, "truncated pointer"));
		}
		let symbol = parts[i];
		let target_offset: u32 = parts[i + 1]
			.parse()
			.map_err(|_| parse_err(path, line_no, "invalid pointer offset"))?;
		let target_pos = Pos::from_letter(parts[i + 2].chars().next().unwrap_or('?'))
			.ok_or_else(|| parse_err(path, line_no, "unknown pointer POS"))?;
		i += 4;
		if let Some((kind, weight)) = map_pointer(symbol) {
			relations.push(ConceptRelation {
				source: id,
				target: ConceptId::wordnet(target_offset, target_pos),
				kind,
				weight,
			});
		}
	}
	let _ = gloss;
	Ok(Synset { id, lemmas, relations })
}

fn map_pointer(symbol: &str) -> Option<(RelationKind, f32)> {
	match symbol {
		"!" => Some((RelationKind::Antonym, 0.4)),
		"@" | "@i" => Some((RelationKind::Hypernym, 0.75)),
		"~" => Some((RelationKind::Hyponym, 0.6)),
		"~i" => Some((RelationKind::Hyponym, 0.2)),
		"#m" | "#s" | "#p" | "%m" | "%s" | "%p" => Some((RelationKind::PartOf, 0.55)),
		"+" => Some((RelationKind::Associated, 0.4)),
		"&" => Some((RelationKind::Synonym, 0.85)),
		"$" | "*" | ">" | "=" | "^" | ";c" | ";r" | ";u" | "-c" | "-r" | "-u" => {
			Some((RelationKind::Associated, 0.3))
		}
		_ => None,
	}
}

fn parse_err(path: &Path, line_no: usize, detail: &str) -> LanguageError {
	LanguageError::Parse {
		path: path.display().to_string(),
		detail: format!("{line_no}: {detail}"),
	}
}
