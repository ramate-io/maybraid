# Experimental packed woody rendering

`MAYBRAID_PACKED_GROVES=compatible` enables a Level 1 resident instance path selected by presentation capability. `on`, `true`, `1` and `woody` are aliases; explicit `orchard` retains the earlier narrow family selector. Unset/off uses ordinary rendering. The switch is read at launch; use one immutable binary for comparisons. Neither mode changes world recipes, density, seeds or placements.

The implementation ports the shared-kit orchard work from [PR964](https://github.com/ramate-io/maybraid/pull/964) toward [issue956](https://github.com/ramate-io/maybraid/issues/956) and [discussion862](https://github.com/ramate-io/maybraid/discussions/862). It keeps one stable ordinary `Mesh3d` proxy per tile, LOD, kit and material batch, and uses the existing Material, prepass and shadow phases. All LOD instance arrays/buffers remain resident while the source revision lives. Per-view CPU selection changes draw references, without rebuilding instance payloads on camera movement.

This reduces per-instance visual ECS/prepare work, but retains ordinary phase machinery per coarse proxy. Compared with existing global Bevy instancing, per-tile proxies **may increase bins or issued draws**. CPU preparation improvement alone is not evidence of a GPU or whole-frame win.

## Supported admission

All 35 woody `ForestGroveTile` families expose typed posed-plant adapters. The adapter is shared with ordinary lazy nesting, preserving the original Arc, placement and materials. Each grove retains its authored structural bands, layer dropout and `WoodyGroveLod` policy: nested detail versus proxy IR, custom low canopies, Rory trunks at effective Low even in UltraLow, sparse unbinned canopies and palm KeepLowPlants. This does not imply all 35 families are admitted: actual all-band geometry/material/kit capability decides atomically per tile.

Storybook, Braid Oak, High Bush, Rory and ball-based conifers can use the existing Leaf/Stick path. Any frond-containing tile, including mixtures with otherwise compatible plants, stays wholly ordinary. The six dedicated tuft/grass families have no packed adapter yet. Their blade thinning, carpets and frond shaders require a later parity stage. Non-grove vegetation and richer GLTF presentations remain on their existing paths.


- One active 3D scene camera rendering the default grove layer. Disjoint UI cameras do not count. Multiple overlapping scene cameras retain the whole ordinary tile until their distinct initial pipeline readiness can be tracked.
- At most one shadowed directional light relevant to the admitted scene camera's **full** RenderLayers; relevant shadowed point/spot lights retain ordinary rendering. Disjoint preview/UI lights do not block the world camera. No energy/visibility relaxation is used.
- Canopy shadow mode Proxy or Off. Full canopy and frond-containing tiles retain ordinary rendering.
- Exact Leaf/Stick material identities; unknown recipes or raster/scalar extensions retain the entire ordinary tile.
- Original shared kits with one scene, one identity mesh node and one triangle primitive; unskinned, unmorphed and no authored COLOR channel. Richer scenes remain ordinary, without dropping primitives.
- Finite, nonsingular, positive-determinant instance poses. The leaf adapter reproduces current merged-root wind/lighting semantics, including rotated/nonuniform collection members; it does not revise the baseline shader.

Forward handoff waits for resident buffers and actual custom draw-prefix readiness. Semantic plant anchors and existing per-plant collision compounds survive the visual handoff. A one-time ownership transfer cancels old queued hide/show visits before reparenting and respects the new owner's Disabled state. Reverse handoff keeps packed visuals while hidden ordinary content streams and physics ownership transfers back. Render extraction snapshots activation state so pipelined frames do not observe main-world transitions early.

The default 256 MiB admission budget bounds instance payloads, not all process/driver memory. `MAYBRAID_PACKED_GROVE_BUDGET_MB` changes this limit. Live admitted tiles remain pinned; denied tiles retry after capacity/source changes rather than camera motion. Asset/source revisions retire/rebuild their payloads. Replacement authored bounds and structural bands invalidate the cache even when the plant Arc is retained; reverse readiness checks the actual replacement producer bag rather than the old plant count. Empty tiles remain ordinary with an explicit empty-content classification. Kit/material/driver allocations and reverse handoff under real assets still require runtime acceptance.

## Diagnostics and comparisons

The optional game feature `packed-diagnostics` installs the dedicated `packed-grove-diagnostics` subscriber crate. `MAYBRAID_PACKED_GROVE_METRICS=1` is a secondary switch. Its durable structured once-per-second events separate global mesh/opaque queue counts from packed residency and issued draw counters. Packed instance IR arrays (`packed_source_ir_bytes`) and retained encoded CPU arrays are separate byte scopes; the former does not count all shared authored tree objects, semantic owners or allocator overhead. Explicit buffer creation reports submitted payload/allocation bytes; neither is measured device-bus traffic or physical GPU memory. Missing exported events are unknown, not zero.

Clean CPU/GPU timing runs omit this feature/env and GPU capture injection. Core preparation and culling have lightweight stable Tracy zones (`maybraid.packed.profile`). A separate diagnostic run and one-frame hardware capture establish actual visible migration, bins/draws and pass coverage.

No performance improvement has been established. Hardware gates include default Proxy/Off shading/depth/normal/shadow parity, supported natural woody census and mixed-world coverage, move-away/LOD-return with unchanged instance upload counters, camera/light/pipeline changes and initial→fallback→readmission. Match settings/route/population evidence and use counterbalanced repeats of the same switchable binary before judging a win.

### Targeted coverage diagnostics

The optional crate propagates chico's `packed-diagnostics` feature. Extra per-batch draw-counter allocation/cloning and increments are absent when that feature is disabled. `packed_view_lod_candidates` reports CPU frustum/distance candidates, not actual phase visibility. Disjoint UI camera layers are excluded. Actual main-draw totals in `packed_tile_lod` are cumulative across all main views and must not be summed across repeated view rows.

Once a tile has actually issued a main draw, the diagnostic subscriber pins a nearby candidate for the launch. `MAYBRAID_PACKED_GROVE_DIAGNOSTIC_TILE` can target explicit entity bits instead; these bits are not a cross-launch world identity. Correlate center/bounds and source revision. Each LOD records at most the lowest64 batch ordinals' buffer IDs/byte sizes, with explicit truncation. A target can remain resident while off-camera; an absent target is reported explicitly. Completion sample sequence and view truncation fields make missing coverage visible. Unchanged target revision and exact buffer IDs across observed LOD return support residency; global uploads can increase legitimately as different world tiles enter.

`packed_family_admission` and `packed_admission_complete` classify loaded presented host ownership by finite family/reason, including unsupported ordinary hosts and retained active tiles whose ordinary source component was removed. Entity-union dedup counts each tile once, including disabled warm hosts. This is not a viewport visibility denominator. Reasons distinguish unsupported adapter/material/geometry/kit/pose, empty content, view/shadow policy, budget, pending assets/GPU readiness, restoring and active state. At most 128 sorted family/reason rows are emitted per second with explicit total/truncation; actual candidate/draw evidence remains separate.

The crate's separate `fixture` example runs real census formatters on an empty headless world without a renderer. Such export-format checks prove no real candidates, issued geometry or pass coverage. The generalized natural-scene census must validate its actual admission and completion events before clean A/B timing.

### View policy evidence

The first generalized natural-origin census reached 1,464–1,606 loaded host tiles but all ten woody families were rejected by the old global light-count guard. The normal game installs a shadowed character-preview key light isolated from world layer0. Bevy filters lights by their intersection with each camera's actual layers, so a disjoint preview key is not a second world shadow light. Admission and required prepass/shadow readiness now use the same sole scene-camera policy; a camera that includes proxy layer30 correctly makes a preview light on1+30 relevant again.

Optional `packed_view_policy`, bounded `packed_policy_camera`/`packed_policy_light`, and `packed_policy_complete` records expose condition counts, the rejected subreason, full-layer relevance, active state, target/viewport, energy and visibility observations. No zero-energy or visibility value bypasses the conservative light guard. These records classify admission, not final rendering correctness. Unrelated preview shadow views still follow generic shadow queues and may share proxy-layer casters; this correction does not suppress or establish their full parity. Live changes to an unsupported relevant light configuration restore ordinary ownership before retiring packed content.

Admitted readiness is keyed by sole-camera identity/full layers, distinct Depth/NormalPrepass state and relevant shadow-light identity. Supported-to-supported changes retire preparing masks or stage active ordinary restoration/repreparation, so an old mask cannot approve a newly required pass. Camera movement does not change this key.
