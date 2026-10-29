# WordNet 3.1

Full Princeton WordNet 3.1 `dict` files used by [`maybraid-language-core`](../../../language/core).

Loaded through [`wordnet-db`](https://crates.io/crates/wordnet-db) (`LoadMode::Mmap`). Languages still import only a bounded semantic neighborhood; the files stay here as shared assets. English lookup runs WordNet Morphy over `noun.exc` / `verb.exc` / `adj.exc` / `adv.exc` plus the official suffix rules so inflected forms (`struck`, `children`, `strikes`) resolve to citation synsets.

WordNet is included under [`LICENSE-WORDNET`](LICENSE-WORDNET). Princeton University does not endorse this project.

## Citation

Princeton University, *About WordNet*, WordNet, Princeton University, 2010. <https://wordnet.princeton.edu/>
