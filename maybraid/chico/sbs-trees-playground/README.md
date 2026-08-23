# Stalk and Ball-stick Trees Playground

> [!NOTE]
> This is based on `sdf-common-playground` prior to further standardization.

## Run

```bash
cargo run -p chico-sbs-trees-playground
```

The default preview is **Liam's Conifer** (sticks plus tuft foliage at each joint). Press **`/`** for the command console.

### Examples

```text
/render liams-conifer
/render liams-conifer --stalk-height 40 --res-2 5
/render friends-conifer --stalk-height 30 --projection 0.12..0.03
/render northern-conifer --stalk-height 30
/render northern-conifer --stalk-height 28 --ring-heights 0.12..0.95 --splay-radius-fraction-of-height 0.02
/render temperate-conifer --stalk-height 12 --frond-spawn-fraction 0.6
/render penmarch-torch --tree-height 24
/render kamakura-torch --tree-height 24
/render rorys-head-trained --tree-height 18
/render rorys-head-trained --tree-height 12 --projection 0.60..0.60
/render spear-tuft
/render frond-crown
/render frond-crown --translate 0,2,0 --spine-segments 16
/render high-bush-shoots --height 12
/render high-bush-shoots --height 12 --shoot-count 8 --foliage-style tuft
/render common-high-bush --height 10
/render moderate-lod-frond-crown --translate 0,2,0
/render sopes-banyan
/show sopes-banyan
/render honu-banyan
/help
```

`/show sopes-banyan` presents via [`VegetationComponents`](../vegetation-components/) / `LodScene`.
`/render sopes-banyan` uses the same LodScene adapter for now (legacy `RenderItem` is unimplemented).

Startup argv uses the same `chico-sbs` CLI (no leading slash):

```bash
cargo run -p chico-sbs-trees-playground -- render frond-crown --translate 0,2,0
cargo run -p chico-sbs-trees-playground -- render liams-conifer --stalk-height 35
cargo run -p chico-sbs-trees-playground -- show monster-grass-plains
cargo run -p chico-sbs-trees-playground -- show vast-orchards
cargo run -p chico-sbs-trees-playground -- show vast --grove-name goettingen-follow
cargo run -p chico-sbs-trees-playground -- show vast --grove-name rolling-oaks
```

`/show monster-grass-plains` tiles a centered 21×21 of default 100 m Monster Grass groves.
`/show vast --grove-name <grove>` does the same for any playground grove (`orchard`, `goettingen-follow`, `rolling-oaks`, …).
`/show vast-orchards` is an alias for `/show vast --grove-name orchard` (flattened tree hosts, shared stick/ball kits).
Vegetation LOD uses the modern refresh stack: bullseye (50 m / 500 m) + spotlight (20 m)
region messages → Avian index → structural level fold → chunk sync.
