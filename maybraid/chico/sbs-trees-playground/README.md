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
/render temperate-conifer --stalk-height 12 --frond-spawn-fraction 0.6
/render spear-tuft
/render frond-crown
/render frond-crown --translate 0,2,0 --spine-segments 16
/render moderate-lod-frond-crown --translate 0,2,0
/render sopes-banyan
/help
```

Startup argv uses the same `chico-sbs` CLI (no leading slash):

```bash
cargo run -p chico-sbs-trees-playground -- render frond-crown --translate 0,2,0
cargo run -p chico-sbs-trees-playground -- render liams-conifer --stalk-height 35
``` 