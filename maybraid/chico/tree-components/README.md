# `chico-tree-components`

Tree-shaped compositions built from sticks and balls ([RFC-183 §3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#31-stalk-and-ball-stick-trees), anchors/chains in subsections **3.1.3–3.1.4**).

Depends on **`chico-sdf`** for geometry continuity with **`chico-stick-components`** and **`chico-ball-components`**. **`chico-sbs-trees`** aggregates this crate with **`chico-sbs-geometry`** for full constructions ([#186](https://github.com/ramate-io/maybraid/issues/186)).

## High-bush shoots ([#225](https://github.com/ramate-io/maybraid/issues/225))

[`high_bush_shoots`](src/high_bush_shoots/README.md) — trunkless upward radial shoots from a ground anchor with plane-splay or tuft foliage (RFC-183 §3.1.6.3). Common High Bush preset constants live in [`preset.rs`](src/high_bush_shoots/preset.rs) ([#233](https://github.com/ramate-io/maybraid/issues/233)).

## Jungle growths ([#226](https://github.com/ramate-io/maybraid/issues/226))

[`jungle_growth`](src/jungle_growth/README.md) — inner dirt/wood mass plus frond crown + Buddha's-hand tuft at one anchor (RFC-183 §3.1.6.4). Node selection is owned by the composing tree recipe.
