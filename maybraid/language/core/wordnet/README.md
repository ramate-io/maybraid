# WordNet extract

This directory vendors a **bounded extract** of the [Princeton WordNet 3.1](https://wordnet.princeton.edu/) `dict` files used by [`WordNetConceptUniverse`](../src/wordnet.rs).

The extract is a derived copy of canonical `index.*` / `data.*` records for the [lexicalization POC](https://github.com/ramate-io/maybraid/issues/897) test lemmas and a one-hop pointer neighborhood. It is not a complete WordNet distribution. The loader still reads the Princeton dict format, so a full 3.1 `dict` directory can be substituted at runtime.

WordNet is included under the terms of [`LICENSE-WORDNET`](LICENSE-WORDNET). Princeton University does not endorse this project.

## Citation

Princeton University, *About WordNet*, WordNet, Princeton University, 2010. <https://wordnet.princeton.edu/>
