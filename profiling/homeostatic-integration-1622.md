# Homeostatic population integration validation

This unpublished integration combines [#838](https://github.com/ramate-io/maybraid/pull/838), the experimental retention/diagnostic child [#1625](https://github.com/ramate-io/maybraid/pull/1625), and [#1622](https://github.com/ramate-io/maybraid/pull/1622) through e708ae9b. Validated source checkpoint: `d46e8c01f1b405c5ae8908dd208818ad4b026c7c`. Latest inspected incoming cdf55bd70 adds documentation/query/goal-vector tests, without production suppression or session fixes; reconciliation is separate.

The population controller owns a matching shared POI goal hold. Generic pursuit respects that hold and clears its transient grant bookkeeping while preserving threat knowledge. Arrival does not release retreat near the neighborhood edge: the default enter margin is40m and release requires80m safely inside. Three200m cells span a600m square. Bounded generation uses the home cell extent and at most one mob, preserving existing generator defaults/RNG behavior. Installed gameplay/streaming capability owns lifetime: pause preserves the encounter; Home/session replacement retires it even when Player survives.

## Validation

Native Nix Rust1.97.1/clang, release, locked/offline, serial compilation, shared established target. No game/GPU run. Every wrapper checks clean source head/tree/file hashes and rejects zero matching tests. The source-only API fix reborrowed optional Bevy `Mut<PoiGoalState>` for the shared API; no policy change.

| Suite at final checkpoint | Passed | Existing ignored |
|---|---:|---:|
| POI library, including three owned-hold/replacement/race regressions |50|3|
| Mob intelligence, including held pursuit/threat preservation and actual routing travel |53|0|
| Barking, including bounded generation and retreat geometry/hysteresis |42|1|
| World intelligence |11|1|
| Selected NPC diagnostics |5|0|
| Strict world activation/reentry/pause/Home integration |5|0|

A separate focused installed-session lifecycle test also passed before fixture-only changes. Full-world tests assert real `MemberOf` bodies and Health, damaged identity/HP across pause/resume, Home cleanup with Player still present, and reentry without duplicate encounters. Player-transform transitions are automated; native locomotion, rendering and kilometer-walk performance remain pending.

The strict fixture required actual headless base owners (mesh, input/window, transforms, gizmos, world serialization), documented world material/camera prerequisites, all material domains, and the existing player-fixture AudioClip asset registration. No warning handler or fail-open assertion is installed. Bootstrap failures remain in the external validation logs, distinct from gameplay assertion failures. Full backtraces identified world-instance skin remapping and gizmo asset requirements without rebuilding vendor debug features.

The reentry case observed the original host tagged `RetiredHost` after frame1, with156 retired hosts queued, then physically despawned on frame143. The fixture uses manual100ms time and a hard200-frame cleanup bound; this measures policy tagging versus shared-budget scheduling in the fixture, **not native gameplay latency**. Fixed20-frame cleanup was insufficient. A destination may activate its own legitimate encounter, so the test follows the original entity identity separately.

## Artifact provenance and remaining work

The shared target reused the intervening boss build's `maybraid-agent-ba1fb736280be388` artifact despite candidate source containing the diagnostic fields. Cargo freshness trace showed older candidate source mtimes compared with the newer artifact. Exact candidate agent blobs matched validated2d23. Audit against the boss918 Cargo artifact paths found only agent selected the same differing artifact; other differing packages recompiled or selected different identities. Refreshing only its source timestamps forced a narrow rebuild, preserving content hashes, and resolved the mismatch. This audit does not establish every historic artifact's provenance.

Local evidence: `/private/tmp/homeostatic-integration-final-receipt.json`, `homeostatic-agent-stale-freshness-receipt.json`, `homeostatic-shared-cache-package-audit.json`, and validation directories `homeostatic-validation-{reentry-r7,poi-lib-final,mob-lib-final,barking-lib-final,world-intelligence-final,world-npc-final}`. Earlier failing logs are preserved under the same prefix. Future cross-checkout compiler handoffs must review and quarantine workspace-owned fingerprint variants only, preserving vendor fingerprints and outputs; the external dry-run helper requires an unchanged plan and exclusive lease.

Runtime actor counts, churn, CPU/update/spotting/physics/mesh-prep cost, memory footprint and kilometer-walk continuity are still unmeasured. Original generated-cell ownership can still retire a nearby moving encounter when its origin cell unloads. That separate lifecycle issue remains; wider actor hysteresis does not solve it. The linear-cull optimization is a separate child checkpoint and has no measured performance claim.
