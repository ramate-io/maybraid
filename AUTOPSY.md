# Autopsy: Failed Character Concepts Implementation

**Branch:** `l-monninger/create-character-screen`  
**Restart from:** [`98f67a5`](98f67a55612d466fe7bef04c1600fdddef575e4d) (`chore: improve plan`, 2026-06-28)

The branch never produced a trustworthy body preview. The failure was twofold: **transforms were applied in physically wrong ways**, and the **code that should express species proportions was scattered and unwieldy**. Late refactors (per-asset `base_transform`, `BodyRig::base_bone_transforms`) pointed in the right direction but could not recover from a bad starting architecture.

---

## Intended model (for the rewrite)

Proportions are **bone-scale driven**, layered in order:

```text
GLTF bind pose
  → species base bone scales     (defines species silhouette, e.g. Braidman mean width)
  → gender / build refinements   (further bone-scale or bone-offset adjustments)
  → user rig sliders             (deviations as multiples/ranges *on top of* that base)
  → marshal to Bevy bones → skin remap → animation
```

**Species** set different **bone scales** (per bone or named bone groups), not whole-scene downscale on rig roots. **Gender and build presets** refine those base scales. **User-facing sliders** map to specific bone transforms; each slider’s effect should be defined relative to the species base (typically a multiplier range where `1.0` = “at species baseline”, not “identity transform that wipes bind pose”).

Scene-root transforms on skinned parts should stay minimal (often identity); the mesh follows the rig’s bone scales after skin remap.

Reference spec: [`CHARACTER_FOR_CONCEPTS_SCREEEN.md`](maybraid/crozon/characters/CHARACTER_FOR_CONCEPTS_SCREEEN.md) Stage 2 (`build_rig_pose` → `RigPose::apply_sliders`).

---

## What went wrong physically

| Mistake | Effect |
|--------|--------|
| **`PartScales` / scene-root scale** (`0.3`, `(0.6,0.3,0.3)`, etc.) | Uniform or non-uniform root scale shrinks limbs with torso — wrong mechanism for width |
| **Head `(0.8,1,0.8)` on scene root; body on bone pose** | Two strategies with no spec rule; body/head never aligned |
| **`apply_rig_sliders`: `*transform = target`** | Replaced full bone `Transform`, wiping GLTF bind-pose translation/rotation |
| **Slider at `1.0` ≠ no-op** | Effects like `Transform::from_scale((1,1,1))` still overwrite bind pose |
| **Skipped `RigPose` pipeline** | No bind snapshot; no compose-with-rest; parallel to `crozon_rigs` |
| **Preset offsets hidden in resolve** | Default Male+Slender ≠ neutral; debugging “all 1.0” required many knobs |

Symptom: “body horror” even with constants at `(1,1,1)` — spawn logs showed identity scene roots while bones were destroyed downstream.

---

## What went wrong in code structure

1. **Too verbose, parameters not in clear sections** — scale logic spread across `species.rs` → `resolve.rs` → `braidman/resolve.rs` → `assembly.rs` → `rig_sliders.rs` with no single transform budget per asset.

2. **Free functions over type methods** — `push_part(...)`, `resolve_slider_values`, `combined_bone_transforms`, thin `rig_spawn_transform` wrappers; empty `Braidman` marker while behavior lived elsewhere.

3. **Thin documentation** — spec’s `RigPose` stage not implemented or explained; bind-pose contract missing; `body_rig.rs` (late) was the exception.

4. **Did not reuse `crozon_rigs`** — ignored `RigPose::apply_sliders`, `axis_aware_translation_delta`, `RigPoseDebug`; reinvented `HashMap` + destructive write in the playground.

5. **Base transforms not obvious** — conflated preview root, scene root (`base_transform`), bone pose (`body_rig_base_bones`), and `feature_transform`; field renamed `local_scale` → `base_transform` mid-branch.

---

## Failed concepts (do not revive as-is)

- **`PartScales` / `SpeciesScale`** — species tables disconnected from assets; overlapped concepts; tuned reactively
- **Scene-root downscale for body width** — wrong layer for skinned humanoid
- **Slider bone effects as absolute transforms written to entities** — must compose with bind pose
- **Head scene scale vs body bone scale** — pick bone-scale model consistently unless spec says otherwise

---

## Commits on this branch (after restart base)

Compare with `git diff 98f67a55612d466fe7bef04c1600fdddef575e4d..<commit>` or `git show <commit>`.

| Commit | Message |
|--------|---------|
| [`90f2d71`](90f2d71dd57b135d4438cac78c3215a2add1c419) | `feat: character concepts.` |
| [`0688ebd`](0688ebda15cd735bc208a54c433aa573492287e9) | `chore: factor our species scales.` |
| [`c3ae5aa`](c3ae5aa0b69d2484f2f4c6e5bdee15d52af93f0b) | `feat: base transform on the asset to define type-specific asset.` |

**Uncommitted WIP atop `c3ae5aa`** (same failed line of work; not in history): `body_rig.rs`, `BoneTransform` / `body_rig_base_bones`, `apply_rig_sliders` compose path, `DEBUG_BODY_MESH_ONLY`, further asset/resolve/playground edits. Diff: `git diff c3ae5aa`.

---

## Rewrite checklist

- **One pipeline:** resolve → `RigPose` (bind + species bone scales + presets + user sliders) → spawn → marshal bones → skin remap
- **Species proportions:** named bone groups + scale constants on rig asset types (readable, like late `body_rig.rs`)
- **Presets:** gender/build adjust resolved slider values or bone-scale deltas — document that defaults are not “raw 1.0”
- **User sliders:** effects defined as ranges/multiples of species base; `1.0` must mean “species baseline”, not identity overwrite
- **Reuse:** `RigPose::apply_sliders`, articulation helpers, bind-pose debug from day one
- **Types own behavior:** rig assets expose base bone pose; resolved config builds pose — not eight-arg free functions
- **Debug mode:** bind-pose-only preview before any slider/preset apply

---

## Key files (for diff archaeology)

`character-concepts-playground/src/{assembly,rig_sliders}.rs` · `characters/src/braidman/{resolve,species,sliders}.rs` · `characters/src/braidman/assets/{body,body_rig,head}.rs` · `characters/src/resolve.rs`
