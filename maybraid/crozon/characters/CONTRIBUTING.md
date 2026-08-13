# Contributing to the `crozon-characters` crate

## General

## Adding a new species

A species is a self-contained module under `src/species/` that owns its config,
baseline pose, asset resolver, and color `enums`. Shared mesh catalogs (body, eye,
hair, clothing, etc.) live in `src/species/common/assets.rs`; species-specific
meshes and swatches stay in the species module.

Use **`Brodler`** as the template for a fixed-silhouette species (config + assets
+ pose, no user sliders). Use **`Braidman`** when the species needs gender/build
presets and rig or feature sliders.

Orthograde humanoid species that share the standard head-rig sockets should
implement [`CharacterComponents`](src/components.rs) with builders in
[`species/common/nodes.rs`](src/species/common/nodes.rs) (`eye_left` /
`eye_right`, `ear_left` / `ear_right`, `orthograde_head_rig`, …). Right-side
left-authored GLBs use [`SceneRef::reflected`](../../scene-ref/src/scene_ref.rs)
for handedness; socket locals stay placement-only (no `scale.x = -1`). Clothing
is [`Clothed<T>`](src/components.rs) via [`CharacterRecipe`](src/components.rs)
(`Config::clothed()`), not part of the inner species. Register the playground
host with [`add_character_components_host::<Clothed<T>>`](src/plugin.rs);
[`RigNode`](src/nodes/rig_node.rs) / [`PartNode`](src/nodes/part.rs) are
registered once.

The playground spawn path is LodScene (`Config::clothed()`). Keep
`*Assets::resolve()` / `visual_scene()` until a species has been visually
reviewed against that recipe; do not add new callers of the assembly spawn
path.

### 1. `crozon-characters` (this crate)

1. Create `src/species/<name>.rs` with:
   - `*Config` and `*Colors` structs
   - species-local `enums` (skin, eye, head, mouth, etc.) with `VALUES`, `label()`,
     and `color()` where needed
   - `impl SpeciesConfig for *Config` calling `*Assets::resolve(self)`
2. Create `src/species/<name>/assets.rs`:
   - `*Assets::resolve()` building a [`ResolvedCharacterAssembly`](src/assembly.rs)
   - socket attachments for head features; omit parts the species does not use
     (for example `Mygr` has no nose)
3. Create `src/species/<name>/pose.rs` when the species has a fixed baseline:
   - compose [`RigPoseLayer`](../rigs/src/pose.rs) scales via
     `BraidmanSliders::apply_*` helpers for leg length, thigh thickness, etc.
4. Register the module in [`src/species.rs`](src/species.rs).
5. Add shared asset paths to [`src/species/common/assets.rs`](src/species/common/assets.rs)
   when a mesh may be reused across species.
6. Wire menu traits in [`src/menu_traits.rs`](src/menu_traits.rs):
   - `impl_menu_identity!` for list/cycle `enums`
   - `impl_asset_option!` for thumbnail meshes
   - `SwatchOption` for species color `enums`

### 2. `crozon-character-ui-menus`

1. Add `src/characters/<name>.rs` — menu structs, `From` conversions to/from
   `*Config`, and `camera_focus_for_field`.
2. Register the module in [`src/characters.rs`](../character-ui-menus/src/characters.rs).
3. Extend [`src/character.rs`](../character-ui-menus/src/character.rs):
   - `ConceptSpecies::<Name>`
   - `CharacterMenu::<name>` field
   - `from_<name>`, `apply_<name>`, config accessor
4. Extend [`src/event.rs`](../character-ui-menus/src/event.rs) with any new
   `CharacterField`, `AssetValue`, and `SwatchValue` variants.

### 3. `crozon-character-concepts-playground`

1. Add `src/commands/<name>.rs` for CLI preview `args`.
2. Extend [`src/preview.rs`](../character-concepts-playground/src/preview.rs):
   - `ConceptPreviewConfig::<Name>`
   - `PreviewTarget::<Name>*` variants
   - `preview_color_<name>`, `preview_asset_target` mapping, spawn/`clothed()` /
     `lod_rig_nodes` match arms
3. Update [`src/menu_listeners.rs`](../character-concepts-playground/src/menu_listeners.rs),
   [`src/species_session.rs`](../character-concepts-playground/src/species_session.rs),
   [`src/focus_reference.rs`](../character-concepts-playground/src/focus_reference.rs),
   and [`src/preview_color.rs`](../character-concepts-playground/src/preview_color.rs).

### Verify

```bash
cargo test -p crozon-characters
cargo test -p crozon-character-ui-menus
cargo check -p crozon-character-concepts-playground
```

Spawn from the playground UI (species picker) or CLI:

```bash
crozon-concepts mygr preview --skin ginger --eyes green
```

Preview socket/skin debug:

```bash
CROZON_PREVIEW_DEBUG=1 crozon-concepts hars preview
```

### Socketing, scale, and shear

Parts attach with [`SocketAttachment`](src/assembly.rs): a `ChildOf(bone)` plus a
local `Transform`. Bevy propagates the full parent affine, so **non-uniform scale
on an ancestor combined with rotation on or under that socket shears** the
attached mesh. Intermediate bones do not fix this if the part remains a transform
child of the scaled chain.

Nested armatures use [`SocketRig::Neck`](src/assembly.rs) /
[`CharacterPartSlot::NeckRig`](src/assembly.rs) (+ optional `NeckMesh`):

1. Socket the neck OwnRig to the body `head_socket`.
2. Apply **pitch** (and optional **uniform** scale) via [`ResolvedCharacterPart::pose`].
3. Socket the head to the neck tip `head_socket` (counter-pitch on the tip bone).

With a dedicated neck armature/mesh, **prefer authored length + pitch + uniform
scale** — do not lengthen via `BoneScale::length` / bind translation on that path.
Check armature rest orientation: a 90° export flip can make local-X pitch yaw in
world space (Hars raises about local −Z on the triple-join neck).

Until rigid (no-scale) sockets land ([#516](https://github.com/ramate-io/maybraid/issues/516)):

- Prefer **uniform** scale on any bone that still parents a pitched socket or head.
- Avoid `BoneScale::length` / `thickness` on bones that rotate or parent a rotated
  `head_socket` / feature socket.
- Keep mesh and armature as separate assets when needed (`NeckRig` armature +
  `NeckMesh` skinned to it), matching the head rig / head mesh split.

[#516](https://github.com/ramate-io/maybraid/issues/516) tracks an opt-in rigid
socket path (follow bone translation + orthonormal rotation, ignore scale/shear).
