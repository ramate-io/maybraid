# `chico-sbs-geometry`

Pure numerical / geometric utilities for stalk and ball-stick constructions ([RFC-183 §3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#31-stalk-and-ball-stick-trees)).

Keeps shared pose, extents, and chain-friendly math **free of `chico-sdf`** where possible so **`chico-sbs-trees`** can compose primitives without forcing every crate through SDF.

- `chain`: hysteresis rules defining how geometry branches out, similar to an L-system. 
- `anchor`: the rule responsible for placing the first elements in the chain. 
- `sbs`: the geometry frontend for a given tree-like construction, this typically conceals a lot of complexity and ensures reasonable parameters for the intended aesthetic. 