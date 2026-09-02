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

1. The blend-export pre-commit hook writes the canonical `.glb` under
   `maybraid/assets/characters/clothes/body/`.
   Menu thumbnails always use this catalog path. Body garments also have host-fit
   GLBs at `clothes/body/{body_stem}/{file_stem}.glb` from
   `scripts/clothes-fit/fit.sh`; blend-export must not overwrite those.
2. In `maybraid/crozon/character-items/src/clothing.rs`:
   - Add a `CLOTHING_*` path constant for the file.
   - Add a variant to `ClothingMesh`.
   - Append it to `ClothingMesh::VALUES`.
   - Add matching `label()`, `file_stem()`, `path()`, and `nouns()` arms. Labels use
     kebab-case (for example `harem-pants-upper`); asset filenames may use snake_case.
     `nouns()` feeds hashed item names with look/color adjectives
     (`hashed_item_name` in `crozon-character-items`).
   - Body garments always resolve through `path_on(host)` as
     `clothes/body/{body_stem}/{file_stem}.glb`. Generate those files with
     `scripts/clothes-fit/fit.sh`.
   - Clothing color and surface recipe are independent of the mesh. Species
     menus share `ClothingMenu`: color swatches and look tiles (`space-suit`,
     `tattered`, `hawaiian`, `cloth`, `scales`, `wizards-veins`, `glitter`) per
     worn layer. Each look also has `adjectives()` used by hashed item names.
     Unset layers fall back to `clothing_material` / `clothing_default`.
     Assembly stamps `MaterialRef::named(recipe_id).with_palette([color])`; Crozon’s
     MaterialLib claims those names and packs palette / noise / scalars / rasters
     onto the clothing shader uniform.
3. `ListValues` and `AssetOption` for `ClothingMesh` in `crozon_characters` pick up the
   new variant automatically, so the UI, CLI (`--clothing`), and preview assembly work
   without further changes in this crate.

Verify with:

```bash
cargo test -p crozon-character-ui-menus
cargo check -p crozon-character-concepts-playground
```
