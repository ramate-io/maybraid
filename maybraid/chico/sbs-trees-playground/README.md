# Stalk and Ball-stick Trees Playground

> [!NOTE]
> This is based on `sdf-common-playground` prior to further standardization.

## Run

```bash
cargo run -p chico-sbs-trees-playground
```

The default preview is **Liam's Conifer** (sticks only until tufts are wired). Press **`/`** for the command console.

### Examples

```text
/render liams-conifer
/render liams-conifer --stalk-height 40 --res-2 5
/render sopes-banyan
/help
```

Startup argv uses the same `chico-sbs` CLI (no leading slash):

```bash
cargo run -p chico-sbs-trees-playground -- render liams-conifer --stalk-height 35
``` 