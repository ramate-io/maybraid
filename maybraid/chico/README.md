# Chico vegetation crates (`RFC-183`)

Workspace crates under **`maybraid/chico/`** implement [RFC-183: Chico Vegetation](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation). Epic: [#185 — Implement RFC-183: Chico Vegetation](https://github.com/ramate-io/maybraid/issues/185).

## Layout

| Path | Crate | Role |
| --- | --- | --- |
| [`lib/`](lib/) | **`chico`** | Convenience re-exports (`chico-sdf`, `chico-sbs-trees`). |
| [`sdf/`](sdf/) | **`chico-sdf`** | Chico-facing SDF helpers; depends on shared [`sdf-common`](../sdf/common). |
| [`sbs-geometry/`](sbs-geometry/) | **`chico-sbs-geometry`** | Stalk / ball-stick geometry plus tuft, frond, and bush shape IR. |
| [`vegetation-components/`](vegetation-components/) | **`chico-vegetation-components`** | Domain IR (`StickNode` / `FoliageNode`) + `VegetationComponents` / `LodScene` (Richmond-style). |
| [`sbs-trees/`](sbs-trees/) | **`chico-sbs-trees`** | VegetationComponents plants for **stalk and ball-stick trees** ([§3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#31-stalk-and-ball-stick-trees)). Tracks [#186](https://github.com/ramate-io/maybraid/issues/186). |

## Dependency direction

```text
sdf-common
    └── chico-sdf

chico-sbs-geometry   (shape IR + chain / anchors / SBS)

chico-sbs-trees ──► chico-sdf, chico-sbs-geometry, chico-vegetation-components

chico (lib) ──► chico-sdf, chico-sbs-trees
```

Lower crates stay small and reusable; **`chico-sbs-trees`** is the integration point for milestone **RFC-183 4.1** / issue **#186**.
