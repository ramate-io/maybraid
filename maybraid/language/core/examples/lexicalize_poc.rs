//! Render the [#897](https://github.com/ramate-io/maybraid/issues/897) test
//! utterances through both POC lexicalizers.

use maybraid_language_core::{
	poc_universe, CompositionalLexicalizer, InMemoryLexicalGraph, LanguageOutput, PocLexicon,
	Profile, RootHeavyLexicalizer, SurfaceGrammar, Utterance,
};

fn main() -> Result<(), maybraid_language_core::LanguageError> {
	let universe = poc_universe()?;
	let lex = PocLexicon::from_universe(&universe)?;
	let utterances = [
		("John gave the book to Mary.", Utterance::john_gave_the_book_to_mary(&lex)),
		(
			"John gave the book to Mary, who was a witch that had many helpers.",
			Utterance::john_gave_the_book_to_mary_the_witch(&lex),
		),
		(
			"The stream carried water from the mountain to the river.",
			Utterance::stream_carried_water(&lex),
		),
	];

	for (title, utterance) in utterances {
		println!("=== {title} ===\n");
		print_language(
			"LANGUAGE A — compositional",
			&CompositionalLexicalizer::default(),
			SurfaceGrammar::compositional(),
			utterance.clone(),
			&universe,
		);
		print_language(
			"LANGUAGE B — root-heavy",
			&RootHeavyLexicalizer::default(),
			SurfaceGrammar::root_heavy(),
			utterance,
			&universe,
		);
	}
	Ok(())
}

fn print_language(
	title: &str,
	lexicalizer: &impl maybraid_language_core::Lexicalizer,
	grammar: SurfaceGrammar,
	utterance: Utterance,
	universe: &impl maybraid_language_core::ConceptUniverse,
) {
	let mut graph = InMemoryLexicalGraph::new();
	let output =
		LanguageOutput::render(utterance, lexicalizer, universe, &mut graph, &Profile::neutral());
	println!("{title}");
	println!("  {}", output.realize(grammar));
	println!();
	print!("{}", output.debug_report("  lexicon", universe, &graph));
}
