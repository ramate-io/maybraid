# WordNet extract

This directory vendors a **bounded extract** of the [Princeton WordNet 3.1](https://wordnet.princeton.edu/) `dict` files used by [`WordNetConceptUniverse`](../src/wordnet.rs).

The extract is a derived copy of canonical `index.*` / `data.*` records for the [lexicalization POC](https://github.com/ramate-io/maybraid/issues/897) test lemmas and a one-hop pointer neighborhood (~1.4k synsets, not the full ~117k-lemma 3.1 database). The full `wn3.1.dict.tar.gz` was used to build this extract and is not vendored. The loader still reads the Princeton dict format, so a complete 3.1 `dict` directory can be substituted at runtime via [`WordNetConceptUniverse::from_dir`](../src/wordnet.rs).

WordNet is included under the terms of [`LICENSE-WORDNET`](LICENSE-WORDNET). Princeton University does not endorse this project.

## Citation

Princeton University, *About WordNet*, WordNet, Princeton University, 2010. <https://wordnet.princeton.edu/>
