# `chico`

Umbrella crate for downstream dependents that want a single manifest dependency.

Re-exports:

- **`chico-sdf`** — SDF-facing types bridging [`sdf-common`](../../sdf/common).
- **`chico-sbs-trees`** — stalk / ball-stick tree construction ([RFC-183 §3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#31-stalk-and-ball-stick-trees), [#186](https://github.com/ramate-io/maybraid/issues/186)).

Prefer depending on **`chico-*`** leaf crates directly when you only need one layer.
