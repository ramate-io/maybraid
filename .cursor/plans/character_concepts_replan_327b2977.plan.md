---
name: Character Concepts Replan
overview: Replan the remaining Character Concepts work around early animation validation, then asset expansion, complete Braidman resolution, UI controls, and finally camera/object preview polish. This replan references the original Character Concepts plan and `CHARACTER_FOR_CONCEPTS_SCREEEN.md`; recent `git status --short`, `git diff --stat`, and `git diff --name-status` reported no current working-tree diff.
todos:
  - id: animations-first
    content: Add concepts-playground animation selection/playback using the existing humanoid animation pattern, then validate sockets under motion.
    status: completed
  - id: hair-clothes-pause
    content: Add a small hair and clothing slice through the existing resolved assembly API, then pause for visual/runtime inspection.
    status: completed
  - id: complete-braidman
    content: Fill out the complete Braidman asset, preset, and slider model while keeping resolver code simple and explicit.
    status: completed
  - id: first-ui-cut
    content: Implement first UI controls for sliders, single selects, and multi-selects without camera suggestions.
    status: pending
  - id: camera-previews
    content: Add camera suggestions and small object renderings after the core UI and model are stable.
    status: pending
isProject: false
---

# Character Concepts Replan

## Guiding Constraints
- Keep the current API shape simple: `BraidmanConfig` carries selected values, `BraidmanAssets::resolve` builds a `ResolvedCharacterAssembly`, and playground systems consume that assembly.
- Prefer adding small enum fields and resolver helpers over introducing registries, dynamic schemas, or broad trait hierarchies until the UI proves they are needed.
- Preserve the clear split between authored asset normalization, species/preset/slider proportion effects, socket placement, skin remap, and animation.
- Treat the current tuned Braidman code as a live source of truth. The preset tables in the spec are guidance for the eventual model, not instructions to naively overwrite calibrated constants in `species/braidman/pose.rs`, `assets.rs`, or `sliders.rs`.

## Source Documents
- Original plan: [/Users/l-monninger/.cursor/plans/character_concepts_c3182a0f.plan.md](/Users/l-monninger/.cursor/plans/character_concepts_c3182a0f.plan.md). Preserve its completed foundation: workspace wiring, lean character modules, command-driven playground, resolved assembly spawning, socket placement, skin remap, and bind-pose composition.
- Spec: [maybraid/crozon/characters/CHARACTER_FOR_CONCEPTS_SCREEEN.md](maybraid/crozon/characters/CHARACTER_FOR_CONCEPTS_SCREEEN.md). Use it for the staged data flow, assembly hierarchy, authored asset normalization, sliders/presets semantics, full Braidman asset list, animations, clothes, hair, UI concepts, and camera/object preview direction.
- Current implementation: use the checked-in Braidman tuning before applying spec tables. When spec values and code differ, inspect the code and preserve the tuned behavior unless a change is explicitly part of the phase goal.

## Phase 1: Add Animations First
Use animation to validate socket stability before adding more parts. Port the existing playback pattern from [maybraid/crozon/playground/src/animation.rs](maybraid/crozon/playground/src/animation.rs) into the concepts playground, but keep it scoped to the body rig and current `BraidmanConfig`.

- Add an `AnimationMode`-like selection to the concepts playground command path in [maybraid/crozon/character-concepts-playground/src/commands/braidman.rs](maybraid/crozon/character-concepts-playground/src/commands/braidman.rs) and [maybraid/crozon/character-concepts-playground/src/preview.rs](maybraid/crozon/character-concepts-playground/src/preview.rs).
- Add a small concepts-local animation system module, borrowing the existing `HumanoidV0Rig` marshal/apply pattern from [maybraid/crozon/playground/src/animation.rs](maybraid/crozon/playground/src/animation.rs).
- Make pose maintenance and animation ordering explicit so proportion scales are preserved while animation writes rotation/translation-driven transforms.
- Validate sockets by cycling through walk/run/jump/tuck and using `dump-bones` plus visual inspection of head/eye/nose/mouth/ear attachment.

## Phase 2: Add Clothes And Hair, Then Pause
Expand the resolved assembly without changing the underlying API much.

- Add `HairMesh` and a minimal clothing selection model in [maybraid/crozon/characters/src/species/braidman/assets.rs](maybraid/crozon/characters/src/species/braidman/assets.rs), using the existing `CharacterPartSlot::Hair` and `CharacterPartSlot::Clothing` slots from [maybraid/crozon/characters/src/assembly.rs](maybraid/crozon/characters/src/assembly.rs).
- Add config fields to [maybraid/crozon/characters/src/species/braidman.rs](maybraid/crozon/characters/src/species/braidman.rs) and CLI flags in [maybraid/crozon/character-concepts-playground/src/commands/braidman.rs](maybraid/crozon/character-concepts-playground/src/commands/braidman.rs).
- Start with one or two representative hair assets and clothing assets before filling the matrix.
- Pause after this phase to inspect skin remap failures, socket behavior, normalization scales, and whether clothing should be single-select or multi-select in the API.

## Phase 3: Complete The Braidman Model
Fill in the Braidman asset and proportion surface after the animated/socketed preview is trustworthy.

- Add the remaining Braidman body/head/eye/nose/mouth/ear/hair/clothing variants listed in [maybraid/crozon/characters/CHARACTER_FOR_CONCEPTS_SCREEEN.md](maybraid/crozon/characters/CHARACTER_FOR_CONCEPTS_SCREEEN.md).
- Move preset behavior toward the documented model in [maybraid/crozon/characters/src/species/braidman/presets.rs](maybraid/crozon/characters/src/species/braidman/presets.rs): shared preset IDs remain, Braidman owns the slider-offset tables. Do this by translating the current tuned code into clearer preset/slider resolution, not by copying spec percentages over the existing calibrated silhouette.
- Expand [maybraid/crozon/characters/src/species/braidman/sliders.rs](maybraid/crozon/characters/src/species/braidman/sliders.rs) only as needed for the complete model; keep the resolved path readable.
- Keep [maybraid/crozon/characters/src/species/braidman/pose.rs](maybraid/crozon/characters/src/species/braidman/pose.rs) focused on mapping resolved slider values to rig effects.

## Phase 4: First UI Cut
Add a practical UI only after the model surface exists.

- Implement Bevy UI controls in [maybraid/crozon/character-concepts-playground/src/ui.rs](maybraid/crozon/character-concepts-playground/src/ui.rs): sliders, single selects, and multi-selects.
- Bind UI controls directly to `ConceptPreviewConfig` rather than creating a new abstraction layer.
- Keep camera suggestions out of this first UI pass.
- Use the command status/debug output as a cross-check that UI state resolves to the same config shape as CLI commands.

## Phase 5: Camera Suggestions And Object Thumbnails
Polish the creator experience after the core controls are validated.

- Add camera focus metadata for relevant controls, then wire camera suggestions into the concepts playground camera module usage from [maybraid/crozon/playground/src/lib.rs](maybraid/crozon/playground/src/lib.rs).
- Add small renderings/previews for relevant character objects only after selections and sockets are stable.
- Keep thumbnails as a presentation feature; do not let them alter the resolver API.

## Validation
- After each phase, run focused checks for `crozon-rigs`, `crozon-characters`, and `crozon-character-concepts-playground`.
- For Phase 1 and Phase 2, also run interactive smoke tests through `crozon-concepts braidman preview` with animation and asset variants, plus `dump-bones` when sockets or skin remaps look wrong.
- Treat runtime warnings from `NoMatchingArmature` as blockers before expanding the asset matrix further.