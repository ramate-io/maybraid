# Contributing

| Task | Description |
|------|-------------|
| [Upcoming Events](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Aevent%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | High-priority `event` issues with planned completion dates. |
| [Release Candidates](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Arelease-candidate%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | Feature-complete versions linked to events. |
| [Features & Bugs](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Afeature%2Cbug%2Cproposal%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | High-priority `feature` and `bug` issues. |

Each issue should have a priority. All Releases Candidates should eventually link up to Events. All Features and Bugs should eventually link up to Release Candidates. 

Proposals issue labels are generally used to mark requests for research.

All issues should tag the following projects:

- `Ramate`: https://github.com/orgs/ramate-io/projects/2
- `Maybraid`: https://github.com/orgs/ramate-io/projects/17

## Events

Events should be titled as follows:

```
Event: <name of the event>
```

Events should be formatted in Markdown as follows:

```
# Summary
- **Target date:** <insert date>

Two to three line description of the event.

- Bulleted
- Line Item
- Callouts

Any other description as the writer sees fit...
```

## Release Candidates

Release Candidates should be titled as follows:

```
Release Candidate: <name of the release candidate>
```

Release Candidates should be formatted as follows:

```
# Summary

This Release Candidate is intended to enable the following events:

- [Link to Event Issue](https://github.com/ramate-io/gwrdfa/issues/4)
- [Link to Event Issue](https://github.com/ramate-io/gwrdfa/issues/4)
- [Link to Event Issue](https://github.com/ramate-io/gwrdfa/issues/4)

Two to three line description of the release candidate. 

- Bulleted
- Line Item
- Callout

Any other description as the writer sees fit...
```

## Features & Bugs

A Feature or Bug should be titled as follows:

```
<One sentence description of what the bug or feature does.>
```

Features or Bugs should be formatted as follows:

```
# Summary

<One sentence description of what the bug or feature does.>

- Bulleted 
- Line Item
- Callouts

Any other description as the writer sees fit.
```

## Proposals 

A Proposal should be titled as follows:

```
<One sentence description of the problem which should be proposed against.>
```

Proposals should be formatted as follows:

```
# Summary

<One sentence description of the problem which should be proposed against.>.

- Bulleted 
- Line Item
- Callouts

Any other description as the writer sees fit.
```

Proposals will generally be satisfied by additions to repository documentation or inclusion of a written proposal in another knowledge base.

Common external knowledge bases include:

- [OAC](https://github.com/ramate-io/oac)
    - Proposal issue should be followed up with desiderata in the repository.
    - Proposal issue would be closed with a corresponding spec.
- [Ramate](https://github.com/ramate-io/ramate)
    - Proposal issue should be followed up with desiderata in the repository.
    - Proposal issue would be closed with a corresponding spec.
- [Robles](https://github.com/ramate-io/robles)
    - Proposal issue should be followed up with desiderata in the repository.
    - Proposal issue would be closed with a corresponding spec.

> [!TIP]
> **Preferred:** use [`bin/publish-gh-issue.sh`](bin/publish-gh-issue.sh) with a small JSON manifest next to your Markdown body—see [`bin/publish-gh-issue.md`](bin/publish-gh-issue.md). That covers **`gh issue create`**, **sub-issue parent** (UI relationship—not the same as linking in the body), and **`Ramate` / `Maybraid` org projects** (defaults: project **2** and **17** on `ramate-io`). `gh issue create -p …` often fails for org projects; the script uses `gh project item-add` instead (needs `gh auth refresh -s project -s read:project`).
>
> ```bash
> ./bin/publish-gh-issue.sh issues/your-scope/issue.json
> ```
>
> **Manual sequence** (same end state as the script):
>
> ```bash
> # 1) Create issue (repo, title, body, labels—maybraid unless you override in JSON)
> gh issue create -R ramate-io/maybraid --title '…' --body-file body.md -l feature -l priority:medium
>
> # 2) Link new issue as sub-issue of the parent (parent = RC or Jersey epic, child = new)
> PARENT_NODE=$(gh api repos/ramate-io/maybraid/issues/<PARENT#> -q .node_id)
> CHILD_NODE=$(gh api repos/ramate-io/maybraid/issues/<NEW#> -q .node_id)
> gh api graphql -f query='mutation($i: ID!, $s: ID!) { addSubIssue(input: {issueId: $i, subIssueId: $s}) { issue { number } subIssue { number } } }' -f i="$PARENT_NODE" -f s="$CHILD_NODE"
>
> # 3) Org projects (numbers match “All issues should tag…” above: Ramate 2, Maybraid 17)
> gh project item-add 2 --owner ramate-io --url https://github.com/ramate-io/maybraid/issues/<NEW#>
> gh project item-add 17 --owner ramate-io --url https://github.com/ramate-io/maybraid/issues/<NEW#>
> ```

## Rust Style

Prefer **methods on types** over free-floating functions. If logic is about constructing, querying, or transforming a value, put it on the relevant struct/enum (`Foo::build…`, `Foo::from_…`, `foo.sample…`) rather than a module-level helper that takes that type as its first argument. Free functions are fine for true utilities with no natural owner (e.g. tiny pure math), trait impls, and macros—but default to attaching behavior to a type so call sites stay discoverable and naming stays coherent.

Reserve generation **"cell"** terminology for LOD / cellular generation (`OriginCell`, `GenerationScheme`, cell layouts). Bounded rectangles used by shared procedural walks should use names like `Bounds2`, not `*Cell`.

### Modules: never `mod.rs` (hard rule)

> [!CAUTION]
> **Never use `mod.rs`.** Not for new modules, not when splitting a file, not “temporarily.” Every `foo/mod.rs` in this repository is a style bug. The only accepted layout is `foo.rs` next to a `foo/` directory of children.

Agents re-introduce `mod.rs` often — stop and rewrite before continuing. Same rule in nested crates (e.g. Richmond buildings): `monotower.rs` + `monotower/les_halles.rs`, never `monotowers/mod.rs`.

| Wrong | Right |
|-------|--------|
| `foo/mod.rs` + `foo/bar.rs` | `foo.rs` + `foo/bar.rs` |
| `monotowers/mod.rs` + `monotowers/mixed_use_les_halles.rs` | `monotower.rs` + `monotower/les_halles.rs` |
| `usage_plan/mod.rs` + `usage_plan/livable.rs` | `usage_plan.rs` + `usage_plan/livable.rs` |

Rules:

1. The module root is always a **sibling `.rs` file** next to its subdirectory (`foo.rs` declares `pub mod bar;` and lives beside `foo/`).
2. Submodules live **inside** that directory (`foo/bar.rs`, `foo/baz.rs`) — never as `foo/mod.rs`.
3. If you are about to write `mod.rs`, stop and rename: move the contents to `foo.rs` and keep children under `foo/`.
4. Reviewers / agents: treat any new `**/mod.rs` as a **merge-blocking** style violation.

Prefer **`use crate::…`** / concrete paths instead of stitching **`super::`** chains unless there is a compelling reason (e.g., deliberate coupling to an immediate parent in a macro-heavy submodule).

## Rust Tests

Do **not** use [`.unwrap()`](https://doc.rust-lang.org/std/option/enum.Option.html#method.unwrap), [`.expect(...)`](https://doc.rust-lang.org/std/option/enum.Option.html#method.expect), or [`panic!(...)`](https://doc.rust-lang.org/std/macro.panic.html) in test bodies—those snippets are often copied into production code and keep failing habits.

Prefer **`Result`** propagation instead: write helpers that return something like **`anyhow::Result`** (or your crate’s error type), use **`?`**, and declare **`#[test] fn case() -> anyhow::Result<()>`**, so harness failures surface structured errors. [`assert!`](https://doc.rust-lang.org/std/macro.assert.html) / [`assert_eq!`](https://doc.rust-lang.org/std/macro.assert_eq.html) remain appropriate for expectations.

## Performance diagnostics (Tracy first)

Profile LOD and playground hitches with **Tracy**, not in-app `eprintln` / `info!` counters. Bevy already emits `system` / `system_commands` / `par_for_each` zones when built with `trace`. Export a single-frame CSV (“limited to view”) or use `tracy-csvexport` when you need to share a capture.

Do **not** add hitch loggers to `lod`, playgrounds, or `ApplyDeferred` paths. Those scans (`Added<ChildOf>`, `Added<SceneRefRoot>`, …) showed up as hundreds of microseconds on the frames they were meant to explain.

If you need text logs again (composition of a drain wave, command-apply ms, FPS):

1. Put them in a **dedicated diagnostic crate** (for example `maybraid/lod/diagnostics`), not in `lod`’s fulfill systems.
2. Gate the crate and every subscriber behind a **Cargo feature** (compiler flag), default-off. An env var is fine as a secondary switch once the feature is on.
3. Keep playgrounds free of `PlaygroundDiag` / `system_commands` tracing layers. Wire the crate from `main` only when the feature is enabled.

Copies of Tracy CSVs and hitch logs from the orchard work (`frame_*.csv`, `*tracy-zones*.csv`, `vast-orchard-hitch.log`) were removed from the tree. Recover the same kind of capture from **git history** on this branch if you need a worked example of the problems (drain apply, cull DFS, per-type produce, visibility reveal).

## Migrating a grove to the orchard (flattened) approach

Orchard High/Medium plants are **posed kit content**, not a nest of per-stick / per-ball [`LodSceneHost`](maybraid/lod/lib/src/scene/host.rs)s. That is what made `/show vast-orchards` scale: one Avian volume per plant, shared stick/ball [`SceneRef`](maybraid/scene-ref)s, and no fine-phase refresh per kit node.

Canonical example: [`maybraid/chico/groves/src/orchard.rs`](maybraid/chico/groves/src/orchard.rs) (`nest_plant_chunks`) plus helpers in [`grove/vc_compose.rs`](maybraid/chico/groves/src/grove/vc_compose.rs).

1. **Compose with `nest_flattened_plant_chunk`**, not the unused nested-host helpers in [`placed_host.rs`](maybraid/chico/groves/src/grove/placed_host.rs). Those wrap [`ComponentsOnly`](maybraid/chico/vegetation-components/src/lib.rs)`<PlacedVegetation<T>>` and spawn nested [`FoliageNode`](maybraid/chico/vegetation-components/src/foliage/node.rs) / [`StickNode`](maybraid/chico/vegetation-components/src/sticks/node.rs) LOD hosts. Flattened hosts wrap `FlattenedComponentsOnly<PlacedVegetation<T>>` and emit posed kits only.
2. **Share the plant type with `Arc<T>`** when `T` is large (Storybook trees). Orchard stores `Arc<StorybookTree>` so begin/drain does not clone geometry per chunk. Register **that** wrapper in the playground:
   `avian_host!(app, FlattenedComponentsOnly<PlacedVegetation<Arc<YourTree>>>);`
   in [`vegetation_lod.rs`](maybraid/chico/sbs-trees-playground/src/vegetation_lod.rs). Isolated `/show` trees use the same family.
3. **Lazy `SceneChunk` for the plant list.** Build one `SceneChunk::lazy(n, n, …)` that yields `nest_flattened_plant_chunk` per plant (see Orchard `nest_plant_chunks`). Begin must not box every `scene_with_level` up front.
4. **Leave Low / UltraLow as canopy proxies** (`canopy_proxy_site`, `ULTRA_LOW_CANOPY_BIN_METERS`). Flattening is for the High/Medium plant hosts.
5. **Charge kit weight.** Flattened kits use [`FLATTENED_KIT_CHUNK_WEIGHT`](maybraid/chico/vegetation-components/src/lib.rs) so drain does not admit a full SceneRef / `WorldAssetRoot` wave in one frame.
6. **Do not add a second produce plugin per region channel.** `AvianLodSceneRefreshPlugin<T, M, F>` can still be added for bullseye and spotlight; fill/emit are registered once per `T`. Cull stays typed (`AvianLodSceneCullPlugin`).

7. **Quantize + merge kits on the plant.** Step-by-step for other constructions: [`chico-sbs-trees` CONTRIBUTING](maybraid/chico/sbs-trees/CONTRIBUTING.md) (tree / tuft `unit_from_num` + collection merge) and [`chico-groves` CONTRIBUTING](maybraid/chico/groves/CONTRIBUTING.md) (`tree_variants` / flatten). Orchard uses [`StorybookTree::unit_from_num`](maybraid/chico/sbs-trees/src/storybook_tree.rs) (`tree_variants`, default 100). Emission folds sticks and cheap balls into [`MultiSceneMerge`](maybraid/scene-ref) collections. Merge packs kit-local positions into vertex color so [`ChicoLeafMaterial`](maybraid/chico/shaders/src/chico_leaf_material.wgsl) breakup still works. World size stays on the plant [`Placement`](maybraid/chico/vegetation-components/src/placed.rs) scale.

A grove that still uses the nested-host helpers in [`placed_host.rs`](maybraid/chico/groves/src/grove/placed_host.rs) will pay per-node hosts, per-type produce, and a larger visibility/transform wave. Flatten when the plants are kit instances (shared meshes) and fine-phase LOD on each stick/ball is not required.