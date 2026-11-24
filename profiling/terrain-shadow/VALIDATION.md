# Combined shadow receiver validation

The production change copies Bevy mesh flags into terrain and stick PBR input, preserving `NotShadowReceiver`. It changes receiving only: in the tested/default Proxies mode, spherical proxies remain the only vegetation casters and real trunks/foliage retain `NotShadowCaster`. Existing optional Full mode, building caster policy, light settings and cascades are unchanged. The seven production additions and portable diagnostic overlay are consolidated in [#1632](https://github.com/ramate-io/maybraid/pull/1632); #1639 is superseded.

The combined-receivers ON vegetation image shows localized spherical-proxy shadows and the player/weapon ground shadow. This does not isolate stick-surface receiving. Close architectural wall/roof/character shadows were already visible on architectural paving with both custom receivers OFF, so this is not evidence of a building-caster repair.

## Accepted CPU comparison

Both receiver flags OFF → both ON; spherical casters ON in both. One ordered 15-second vegetation pair, fullscreen 1920×1080 Metal Immediate/VSync OFF, noon, High 2048×4 cascades, ordinary supported HP100 player. App-entry intervals are CPU pacing, not GPU timings. Both diagnostic arms share the added uniform layout: the results measure the receiver switch in that diagnostic build. The shipping seven-line patch adds no toggle or uniform.

| Vegetation app-entry interval | OFF | ON | Change |
|---|---:|---:|---:|
| Mean | 11.0558 ms | 11.4527 ms | +0.3969 ms (+3.59%) |
| p95 | 11.5227 ms | 12.0043 ms | +0.4816 ms |
| p99 | 11.8158 ms | 12.3945 ms | +0.5786 ms |
| Maximum | 14.0943 ms | 15.7389 ms | +1.6447 ms |
| Gaps >20 ms | 0 | 0 | — |

There were 1,355/1,308 entry intervals and 29 fresh healthy supported player samples per arm. No >1 ms pipeline-queue overlap required exclusions. Camera-visible mesh counts were 1,587→1,573 (−0.88%); cascade counts matched. The pair is short and ordered; these screens do not prove identical GPU draw workloads. It describes this fixed view only. These short ordered-pair observations are not statistical bounds or a universal frame budget.

Building first pair is admitted at the flat ordinary spawn XZ (1753,1884), with the same HP100 supported player and matched wall/roof bounds. Its baseline already averaged about 19 ms per app-entry interval. The earlier hillside attempt failed stationary admission and is excluded.

| Building app-entry interval, first pair | OFF | ON | Change |
|---|---:|---:|---:|
| Mean | 19.1408 ms | 19.7649 ms | +0.6242 ms |
| p50 | 19.1336 ms | 19.6256 ms | +0.4920 ms |
| p95 | 20.5313 ms | 21.5755 ms | +1.0441 ms (+5.09%) |
| p99 | 21.9514 ms | 23.2502 ms | +1.2988 ms |
| Maximum | 42.6827 ms | 45.4675 ms | +2.7848 ms |
| Gaps >33.33 ms | 1 | 4 | +3 |
| Gaps >50 / >100 ms | 0 / 0 | 0 / 0 | — |

The reverse-order ON attempt was rejected before timing: the near-cascade count was 125→132, exceeding the 5% matching tolerance; no OFF repeat ran. It contributes no timing pair. The observed 1→4 hitch count is unresolved short-sample difference, not a proven repeatable or causal effect. Each first-pair window was about 15 seconds (782/757 entry intervals, 29 healthy support reads each). The phase began at different game ages, about 104→126 seconds, and camera-visible meshes were 46,776→46,955 (+0.38%). These are timing/workload qualifications, not proof that receiving caused the tail difference. Existing architectural wall/eave/courtyard shadows remained visible in both arms.

## GPU and memory limits

The eight-second Metal OFF pilot finalized with a valid target-process interval inventory. The matched ON attempt failed the same 90-second record/save bound. There is **no accepted comparative GPU result** and no GPU-headroom claim; CPU RenderApp spans cannot fill that gap.

Vegetation memory medians used only three PID/start-identity-bound samples per arm at five-second cadence: ON minus OFF was +18.94 MiB footprint and +9.41 MiB RSS. The first building pair similarly had only three samples per arm: +97.48 MiB footprint and +58.69 MiB RSS median. These coarse, short-window samples establish neither a memory regression/saving nor transient peaks; no allocation-owner cause is inferred.

## Provenance and remaining validation

The scene pairs use the same immutable binary within each pair, on main `05885cd` and published Bevy `dfbb7d0`, with `debug-commands` and dependency Tracy instrumentation. Vegetation uses `28d606d9`; the building readout correction uses `1047b838`. Shader/material/owner source hashes and feature/lock graph are identical; only the explicit building observer changed. It ranks transformed real mesh AABBs, not rotation-compensation origins, and runs outside timing windows. Both native builds passed; the corrected observer fixtures remain unrun, with no extra dev-graph rebuild.

Source and native evidence do not establish complete opt-out image coverage, isolated stick receiving, all building LODs, dense canopy scenes or temporal cascade/LOD stability. No caster or shadow-quality change is justified by these observations.

Local evidence retained: `SCENE-RESULTS.md`, `analysis-vegetation-r1/summary.json`, `analysis-building-r3/summary.json`, both immutable manifests, per-run bindings/foreground/health records, bounded export receipts and the accepted OFF/failed ON Metal receipts. Raw assets are not part of the PR.
