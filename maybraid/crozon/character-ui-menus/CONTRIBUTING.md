# Contributing to the `character-ui-menus` crate

## General

## Adding clothing options

`Braidman` and `brodler` clothing menus share one catalog. The species menu structs in
`src/characters/braidman.rs` and `src/characters/brodler.rs` store selected layers as
`MultiSelect<ClothingMesh>`; their renderers in `src/render/braidman.rs` and
`src/render/brodler.rs` call `render_colored_clothing`, which lists every value from
`ClothingMesh::values()`. You do not add per-species menu fields or render rows for a
new garment.

To expose a new clothing mesh in both species:

1. Export the `.glb` under `maybraid/assets/characters/clothes/body/` (or
   `clothes/head/` for hoods and other head wraps).
2. In `maybraid/crozon/character-items/src/clothing.rs`:
   - Add a `CLOTHING_*` path constant for the file.
   - Add a variant to `ClothingMesh`.
   - Append it to `ClothingMesh::VALUES`.
   - Add matching `label()` and `path()` arms. Labels use kebab-case (for example
     `harem-pants-upper`); asset filenames may use snake_case.
3. `ListValues` and `AssetOption` for `ClothingMesh` in `crozon_characters` pick up the
   new variant automatically, so the UI, CLI (`--clothing`), and preview assembly work
   without further changes in this crate.

Verify with:

```bash
cargo test -p crozon-character-ui-menus
cargo check -p crozon-character-concepts-playground
```
