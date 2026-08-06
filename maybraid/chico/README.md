# Chico vegetation crates (`RFC-183`)

Workspace crates under **`maybraid/chico/`** implement [RFC-183: Chico Vegetation](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation). Epic: [#185 — Implement RFC-183: Chico Vegetation](https://github.com/ramate-io/maybraid/issues/185).

## Layout

| Path | Crate | Role |
| --- | --- | --- |
| [`lib/`](lib/) | **`chico`** | Convenience re-exports (`chico-sdf`, `chico-sbs-trees`). |
| [`sdf/`](sdf/) | **`chico-sdf`** | Chico-facing SDF helpers; depends on shared [`sdf-common`](../sdf/common). |
| [`sbs-geometry/`](sbs-geometry/) | **`chico-sbs-geometry`** | Pure geometry for stalk / ball-stick (no SDF yet). |
| [`stick-components/`](stick-components/) | **`chico-stick-components`** | Stick and stalk primitives ([§3.1.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/01-stick-and-stalk-components/README.md)). |
| [`ball-components/`](ball-components/) | **`chico-ball-components`** | Ball and plane canopy primitives ([§3.1.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/README.md)). |
| [`tree-components/`](tree-components/) | **`chico-tree-components`** | Tree-level composition components ([chains / constructions](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#31-stalk-and-ball-stick-trees)). |
| [`vegetation-components/`](vegetation-components/) | **`chico-vegetation-components`** | Domain IR (`StickNode` / `FoliageNode`) + `VegetationComponents` / `LodScene` (Richmond-style). |
| [`sbs-trees/`](sbs-trees/) | **`chico-sbs-trees`** | Integrates the above for **stalk and ball-stick trees** ([§3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#31-stalk-and-ball-stick-trees)). Tracks [#186](https://github.com/ramate-io/maybraid/issues/186). |

## Dependency direction

```text
sdf-common
    └── chico-sdf ──┬── chico-ball-components
                    ├── chico-stick-components
                    └── chico-tree-components

chico-sbs-geometry   (leaf for now)

chico-sbs-trees ──► chico-sdf, chico-*-components, chico-sbs-geometry

chico (lib) ──► chico-sdf, chico-sbs-trees
```

Lower crates stay small and reusable; **`chico-sbs-trees`** is the integration point for milestone **RFC-183 4.1** / issue **#186**.
