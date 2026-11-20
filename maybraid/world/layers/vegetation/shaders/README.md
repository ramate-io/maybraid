# Vegetation shaders

Bevy materials (`leaf` / `frond` / `stick` / bump-out) plus a reusable **WESL
vegetation library** under [`src/wesl/`](src/wesl/).

## WESL library

Modules are embedded by [`VegetationWeslPlugin`](src/wesl.rs). Bevy import paths
are `vegetation_shaders::wesl::<stem>`:

| Module | Responsibility |
| --- | --- |
| `hash` | Stable plant IDs and nested density ranks |
| `curve` | Rest curves, taper, `stable_frame`, `attachment_frame`, ribbons, woody tubes, teardrop leaves |
| `wind` | Root-anchored field + stiffness-aware deformation |
| `lod` | Projected height (actual render resolution), nested keep, buckets |
| `shading` | Painterly two-sided lighting, wood albedo, leaf wrap / crown gradient |
| `layout` | Compact Rust/WESL buffer contracts (grass). Bush descriptors live in `vegetation-bush-field`. |

Grass GPU entry points (and the [#1638](https://github.com/ramate-io/maybraid/issues/1638)
bush follow-up) must import these functions instead of copying them. Species
recipes stay outside this crate.

Runnable composition check: [`src/wesl/import_smoke.wesl`](src/wesl/import_smoke.wesl).

```bash
nix develop --command cargo test -p vegetation-shaders --release --locked
```
