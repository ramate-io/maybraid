# Experimental packed orchard rendering

`MAYBRAID_PACKED_GROVES=orchard` enables a Level 1 resident instance path for supported orchard tiles. Unset/off uses ordinary rendering. The switch is read at launch; use one immutable binary for comparisons.

The implementation ports the shared-kit orchard work from [PR964](https://github.com/ramate-io/maybraid/pull/964) toward [issue956](https://github.com/ramate-io/maybraid/issues/956) and [discussion862](https://github.com/ramate-io/maybraid/discussions/862). It keeps one stable ordinary `Mesh3d` proxy per tile, LOD, kit and material batch, and uses the existing Material, prepass and shadow phases. All LOD instance arrays/buffers remain resident while the source revision lives. Per-view CPU selection changes draw references, without rebuilding instance payloads on camera movement.

This reduces per-instance visual ECS/prepare work, but retains ordinary phase machinery per coarse proxy. Compared with existing global Bevy instancing, per-tile proxies **may increase bins or issued draws**. CPU preparation improvement alone is not evidence of a GPU or whole-frame win.

## Supported admission

- One active 3D scene camera rendering the default grove layer. Disjoint UI cameras do not count. Multiple overlapping scene cameras retain the whole ordinary tile until their distinct initial pipeline readiness can be tracked.
- At most one directional shadow light; point/spot shadow configurations retain ordinary rendering.
- Canopy shadow mode Proxy or Off. Full canopy and frond-containing tiles retain ordinary rendering.
- Original shared kits with one scene, one identity mesh node and one triangle primitive; unskinned, unmorphed and no authored COLOR channel. Richer scenes remain ordinary, without dropping primitives.
- Finite, nonsingular, positive-determinant instance poses. The leaf adapter reproduces current merged-root wind/lighting semantics, including rotated/nonuniform collection members; it does not revise the baseline shader.

Forward handoff waits for resident buffers and actual custom draw-prefix readiness. Semantic plant anchors and existing per-plant collision compounds survive the visual handoff. A one-time ownership transfer cancels old queued hide/show visits before reparenting and respects the new owner's Disabled state. Reverse handoff keeps packed visuals while hidden ordinary content streams and physics ownership transfers back. Render extraction snapshots activation state so pipelined frames do not observe main-world transitions early.

The default 256 MiB admission budget bounds instance payloads, not all process/driver memory. `MAYBRAID_PACKED_GROVE_BUDGET_MB` changes this limit. Live admitted tiles remain pinned; denied tiles retry after capacity/source changes rather than camera motion. Asset/source revisions retire/rebuild their payloads. Kit/material/driver allocations and reverse handoff under real assets still require runtime acceptance.

## Diagnostics and comparisons

The optional game feature `packed-diagnostics` installs the dedicated `packed-grove-diagnostics` subscriber crate. `MAYBRAID_PACKED_GROVE_METRICS=1` is a secondary switch. Its durable structured once-per-second events separate global mesh/opaque queue counts from packed residency and issued draw counters. Source IR and encoded CPU arrays are separate byte scopes. Explicit buffer creation reports submitted payload/allocation bytes; neither is measured device-bus traffic or physical GPU memory. Missing exported events are unknown, not zero.

Clean CPU/GPU timing runs omit this feature/env and GPU capture injection. Core preparation and culling have lightweight stable Tracy zones (`maybraid.packed.profile`). A separate diagnostic run and one-frame hardware capture establish actual visible migration, bins/draws and pass coverage.

No performance improvement has been established. Hardware gates include default Proxy/Off shading/depth/normal/shadow parity, supported production orchard census and mixed-world coverage, move-away/LOD-return with unchanged instance upload counters, camera/light/pipeline changes and initial→fallback→readmission. Match settings/route/population evidence and use counterbalanced repeats of the same switchable binary before judging a win.
