---
name: Fix bone-map pollution
overview: Stop rig BoneMaps from absorbing bones of socket-attached parts and nested rigs, which is the root cause of the persistent head deformation; then clean up compiler warnings.
todos:
  - id: bound-dfs
    content: Bound build_rig_bone_map DFS at CharacterRig/CharacterPart boundaries
    status: completed
  - id: warnings
    content: Fix unused-variable and dead-code warnings in focus.rs and preview.rs
    status: completed
  - id: verify
    content: cargo check and manual head-switch verification
    status: completed
isProject: false
---

# Fix BoneMap pollution from socket-attached parts

## Root cause

`build_rig_bone_map` in [maybraid/crozon/character-concepts-playground/src/skinning.rs](maybraid/crozon/character-concepts-playground/src/skinning.rs) does an unbounded DFS from each `CharacterRig` root. Since parts are socket-attached as `ChildOf(bone)`, the DFS descends into:

- the head rig nested under the body rig (`upper_neck`) — body map polluted with head bones (`root` collides), and
- the gaunt/full head mesh's **embedded duplicate armature** nested under the head rig — head map entries overwritten with duplicate bones.

Result: pose maintenance scales the wrong bones every frame (persistent deformation), skin remap points joints at duplicate bones that prune then despawns, and bind scales get captured from wrong bones. Meerkat is rigid (no embedded armature), so it never collides. CLI vs UI difference is just which side wins the hydration race.

## Changes

### 1. Bound the DFS at rig/part boundaries (the fix)

In `build_rig_bone_map`, skip descending into any child entity that is itself a `CharacterRig` or `CharacterPart`:

- Add a boundary query, e.g. `boundaries: Query<(), Or<(With<CharacterRig>, With<CharacterPart>)>>`.
- When traversing, do not push an entity (or its subtree) onto the stack if `boundaries.get(entity).is_ok()`.

This keeps each rig's `BoneMap` scoped to its own armature only. It covers both preview rigs and the shadow focus-reference rigs (shadow head rig has `CharacterRig`).

### 2. Keep prior hardening

The earlier changes remain valid defense (scene-ready gating in `remap_part_skin_to_rig`, reveal gating on socket/remap/prune in [preview.rs](maybraid/crozon/character-concepts-playground/src/preview.rs), system ordering in [lib.rs](maybraid/crozon/character-concepts-playground/src/lib.rs)). No revert needed.

### 3. Clean up loose ends (compiler warnings)

- [focus.rs](maybraid/crozon/character-concepts-playground/src/focus.rs): `should_pulse(slot)` unused variable — rename to `_slot`.
- [preview.rs](maybraid/crozon/character-concepts-playground/src/preview.rs): dead `ConceptSpecies::label` — remove or `#[allow(dead_code)]` (prefer removal if truly unused workspace-wide).

## Verification

1. `cargo check -p crozon-character-concepts-playground` — no warnings.
2. Run the playground; switch Standard → Gaunt → Full via UI. Heads should render correctly immediately, without clicking another component.
3. Confirm CLI startup with `--head gaunt` still works.
4. Optionally confirm camera focus on head selection settles cleanly (the body-map `root` collision fix should also help shadow-rig framing).