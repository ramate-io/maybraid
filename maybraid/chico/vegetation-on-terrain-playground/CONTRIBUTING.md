# Contributing to vegetation-on-terrain

Character / player host and diagnostics library for [`maybraid-world`](../../world/).
Assembled world runs [`maybraid-world-playground`](../../world/playground/). This
crate is no longer a standalone binary; restore the retired app from
[PLAYGROUNDS.md](../../PLAYGROUNDS.md).

Forest stream knobs live on `VegetationLayerConfig`. Mode, mesh stats, and
character attach stay here until they move to a non-playground crate.

## Verify

```bash
cargo check -p chico-vegetation-on-terrain-playground
```
