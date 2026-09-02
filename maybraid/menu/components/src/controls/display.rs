//! Display-name formatting for HUD labels.

/// Title-case a catalog id or kebab label (`braidman` → `Braidman`).
pub fn menu_display_name(raw: &str) -> String {
	raw.split(|c: char| c == '-' || c == '_')
		.flat_map(|part| part.split_whitespace())
		.filter(|word| !word.is_empty())
		.map(title_word)
		.collect::<Vec<_>>()
		.join(" ")
}

fn title_word(word: &str) -> String {
	if !word.chars().any(|c| c.is_alphabetic()) {
		return word.to_string();
	}
	let mut chars = word.chars();
	let Some(first) = chars.next() else {
		return String::new();
	};
	let mut titled = first.to_uppercase().collect::<String>();
	titled.push_str(&chars.as_str().to_lowercase());
	titled
}

#[cfg(test)]
mod tests {
	use super::menu_display_name;

	#[test]
	fn title_cases_kebab_and_plain() {
		assert_eq!(menu_display_name("braidman"), "Braidman");
		assert_eq!(menu_display_name("fitted-coat"), "Fitted Coat");
		assert_eq!(menu_display_name("still"), "Still");
		assert_eq!(menu_display_name("Head & Features"), "Head & Features");
		assert_eq!(menu_display_name("2 worn"), "2 Worn");
	}
}
