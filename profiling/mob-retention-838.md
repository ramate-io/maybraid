# Mob pursuit and retention candidate

Source baseline: [main e9542c4a](https://github.com/ramate-io/maybraid/commit/e9542c4a653a0181fdd92afab1569645f9526098). Pursuit: [draft PR #838 f9b4425d](https://github.com/ramate-io/maybraid/pull/838). Clean history-preserving merge prepared in an isolated checkout; no merge to main.

## Existing lifetime policy

`mob-scenes/src/scene.rs` selects High within 200 m (3D) of the host center. `plugin.rs::sync_mob_scene_centers` follows host travel. High owns roster stubs; the bodies are unparented. Dropping High immediately despawns bodies through `roster_ref.rs`; this is actor lifetime, not just rendering. No distance hysteresis exists. Death/corpse/replacement policy is separate. Roster writeback stores health and pose, but respawning an actor is not continuity of its entity or contacts.

Barking presents a snapped XZ neighborhood with 1,000 m half-extent. `hcsg/presentation.rs`’s `retire` compares original generated-cell bounds with demand, retires/hides that cell and eventually recursively despawns its hosts. Barking's `retire_members_with_their_mob` observer removes unparented bodies. A nearby pursuing host can therefore disappear when its ORIGINAL cell leaves demand. Distance changes alone cannot solve that case.

The term load-in covers three independent controls: generated/host neighborhood, High actor acquisition, and High actor retirement. This candidate changes acquisition and retirement only, not prefetch generation latency.

## Pursuit integration

#838 gives Pack/Raider hosts timed player-pursuit grants (24 s nominal with jitter), an 8 s browse cooldown and a 250 m host-to-player XZ opportunity range. First-hand received-fire/damage sources may raise grants. POI/routing/travel remain owners of movement, existing tether locks own member tethering. It does not extend actor lifetime. Its 250 m acquisition exceeds the previous 200 m actor boundary. Player POI assignment is external objective knowledge, not a successful visual spotting probe.

## Experimental candidate, not a measured default

Acquire actors within 300 m of the moving host; retain an existing bound roster within 400 m. `MobScene` owns both radii, and playground `with_high_radius` preserves its old single-boundary override unless `with_retain_radius` is supplied. A PreUpdate snapshot checks actual roster entities against matching `MemberOf(host, slot)`. Optimistic pending High on distant recipes does not authorize outer-ring acquisition. Corpses still bound to the roster count as existing bodies until their ordinary corpse removal (four seconds by default). A vanished or recycled body does not retain its former host.

Existing High fulfill/stub/cull remains the only spawn path. Within the annulus a damaged body keeps its entity and Health. Entry envelope area is 2.25 times the old 200 m disk; the outer envelope is 4 times that disk. These are geometric envelopes, not measured population or memory multipliers. Density, family recipe, terrain and route history matter.

Barking's indexed cube derives from the default outer radius plus its existing 50 m margin (450 m). Keep the 250 ms pulse and 1 s full reconciliation. No extra refresh cadence. `scene_lod_status` has no runtime callers in the current refresh pipeline: producers use `scene_lod_level` / `scene_lod_level_from_levels`. Stationary-viewer host/state changes are picked up by the existing pulse/reconciliation. The one-second scan bounds stale levels after teleports outside the pulse cube.

Host distance is not nearest-member distance. A member separated by combat, evasion, or leash slack may still be close to the player after its host leaves 400 m. This remains a distinct policy decision.

## Static-cell ownership follow-up

Do not silently unparent moving hosts: that would evade recursive session cleanup and duplicate generated hosts on cell reentry. A bounded transfer design needs (1) stable generated identity for each host, (2) a session-owned registry for detached active hosts, (3) suppression/rebinding of the original cell slot on reentry, (4) a live-position demand/expiry rule, (5) cleanup on Home/session restart, and (6) a measured population/physics budget coordinated with homeostatic mobs. Defer this redesign until those owners are specified.

Related [#1328](https://github.com/ramate-io/maybraid/issues/1328) already records Guard High-drop body churn and approximately 9.8 ms retirement waves in older captures. Reuse that context; do not call it a new measured regression. [#1055](https://github.com/ramate-io/maybraid/issues/1055) concerns pure generated-value cache retention extents, not actor identity or encounter residency.

## Verification and measurement plan

Combined-candidate release validation passed 113 tests: agent diagnostics8, Barking Gimme pulse1, mob-intelligence52, mob-scenes36, world NPC diagnostics5, and world intelligence11; one existing world intelligence test remains ignored. Three separately run focused retention tests also passed and are already included in the full mob-scenes count. These results validate the combined pursuit/retention/diagnostic tree, not pursuit alone. Formatting and diff whitespace checks pass. Runtime gameplay/performance/memory comparisons are still pending; no performance or memory improvement is claimed.

The first Barking pulse fixture failed because it omitted the typed production region-message registration. Installing the same LodSceneRefreshRegionPlugin as the production assembler fixed that harness; the original failure log is preserved. The mock ScanIndex cache test and real Gimme pulse test remain distinct. Final local receipts are `/private/tmp/mob-838-validation-r3/`; the exact validated combined patch SHA256 is `6665a5f371efe247f9c9ea7a32f9a6965bde19888863551c0f14ca0cc6d557b3`.

Once the lane is available:

1. Run `cargo test --locked -p mob-intelligence -p mob-scenes -p barking --lib` and the world intelligence/lifecycle tests. Review actual corpse/despawn and pending fulfillment transitions with both refresh modes.
2. Gate the focused boundary tests first. Compare A: main; B: main + #838; C: B + retention. Preserve the existing main benchmark binary/source. Use identical release profile, save fixture, seed, camera, resolution, VSync and cache state. Plain-release footprint controls stay separate from diagnostic builds.
3. Walk at least 2 km using normal grounded gameplay (no noclip/teleport route substitution), including urban Guards, journeying Pack/Raiders and a round trip. Start with a short identical fixture for A/B/C attribution. Only a promising, correct candidate proceeds to replicated clean comparison and the kilometer walk. Measure Home after settling. Record actual traveled distance and maximum displacement.
4. Boundary fixture: injure a named actor, oscillate host/player separation around 300 m (and original 200 m), verify entity bits and HP persist; pass 400 m and verify bounded removal. Repeat with a stationary viewer/moving host, all roster bodies downed, slow/pending fulfillment, and host pursuit beyond its original cell. The last case should document the known ownership limitation, not be labeled fixed.
5. At matched windows record total/High hosts, bound living bodies/corpses, pending stubs, spawn/despawn counts and reasons, stable entity identity, live physics bodies/colliders, and peak/settled OS physical footprint. Main-world inline ECS/asset census is a partial ownership view, never total heap/RSS; take it outside timing windows.
6. Tracy: update median/p95/max and hitch counts; grant_prey_objective, host travel/routing, spotting discovery/retained visibility, physics, roster fulfillment/cull and system_commands, transform propagation, mesh preparation, render extraction and pipeline specialization. Keep pipeline warmup and actor lifetime effects separate. Use existing feature-gated diagnostics/agent event seams; no per-frame hitch logger in production systems.
7. Use the existing #1328 update hitch gate (33 ms, and twice-window-median separately) for boundary retirement frames. Report observed median/p95/max update and owner-zone deltas, peak/settled footprint deltas, and actor-count deltas rather than claiming statistical significance from a single pair. The aim is boundary identity/HP continuity and bounded removal without introducing a new >33 ms actor-retirement hitch. Other pipeline hitches remain separately attributed. Decide final distances from actor/CPU/memory cost and observed continuity. Check settled memory and actor counts return to bounded envelopes on the return route/Home. Do not infer a leak from a high-water allocator footprint alone.

## Optimization candidates to measure

- #838 currently queues a `Poi`/`LocalPoi` insert on the same prey for every granted host every update. A change-detected shared registration may remove redundant commands, but must preserve policy-specific POI ownership and cleanup.
- `cull_members_when_high_drops` scans all members for every non-High host; a host-state change filter or one member pass against host state could reduce O(hosts × members) work. Confirm command-apply/cull cost before changing semantics.
- Roster retention snapshot walks host slots and checks membership once per frame. Profile it before adding change-tracking machinery that could miss entity removal or recycled slots.
- Larger actor envelopes can increase spotting/physics/animation/mesh-prep cost even when grants are cheap. Use measured per-owner costs to choose budgets; do not blanket-retain all actors.

## Existing diagnostic coverage and gaps

`agent.actors` already emits changed total hosts/bodies/brains, pending-support bodies, Y range and ground under the lowest body (`world/lib/src/agent.rs::agent_actor_census`). This is a population snapshot, not a spawn/despawn reason counter; changes can cancel within one frame. `diagnostics npcs RADIUS` and `diagnostics npc ENTITY` expose at most eight explicit NPC snapshots with stable entity bits, cached support, spotting/combat/weapon state. The baseline NPC snapshot does not expose Health or roster slot identity. The separate diagnostic candidate below fills that selected-actor gap. `diagnostics memory` under the default-off memory-census feature supplies selected ECS/asset CPU payload, not whole-process ownership or physics churn. OS footprint sampling and Tracy already supply memory/timing attribution.

Still missing for complete runtime churn accounting: per-reason High-cull vs death vs host/session retirement events, complete host/slot→body histories, living/corpse split, exact physics body/collider counts and pending High-stub count. Do not silently present net body-count changes as churn. Use focused ECS regression assertions for HP/identity now; if the short runtime comparison needs complete churn attribution, add a dedicated default-off diagnostic crate/feature or extend the existing explicit diagnostic seam after review, not production per-frame logging.

## Separate selected-NPC diagnostic candidate

`maybraid/agent/src/diagnostics.rs` and `world/lib/src/agent/diagnostics/npc.rs` extend only explicitly requested NPC snapshots with optional current/max Health and optional membership: textual host entity bits, roster slot, cached MobScene host center and selected LodSceneLevel debug name. A stale membership can report its old host identity while center/level are null; absent Health/membership stays null. Optional serde defaults preserve reading older records. This is not a visual visibility claim and does not probe physics or change AI.

The existing --agent-diagnostics install gate, diagnostics on/off check, explicit nearest/entity requests and eight-actor limit remain. Extra queries run only for selected actors during the existing one-shot capture; no new periodic system, whole-world census, reason logger or visibility probe. Tests cover schema compatibility/null components and read-only damage/identity evidence through host retirement. Both schema and world capture tests pass in the combined validation above.

This diagnostic delta is a separate commit from the #838 merge and gameplay retention change. Boundary fixtures can request an actor's exact identity before/after oscillation, observe HP and host-selected level, and distinguish retirement from a new body identity. This supplies selected-actor continuity evidence without pretending to measure all spawn/despawn reasons.

## Incoming homeostatic owner

[Draft PR #1622](https://github.com/ramate-io/maybraid/pull/1622) introduces independent200m activation and3x3 original-home-cell residency. It has not been merged into this candidate. The [published integration review](https://github.com/ramate-io/maybraid/pull/1622#issuecomment-6962910762) separates neighborhood geometry, retreat objective priority, gameplay/session cleanup, and real routing/reentry tests from these shared actor envelopes. Review its moving head before combining; reported green reduced-scope tests are not proof of actual routing displacement.

A further #838 source caveat remains: first-hand received-fire/damage source bits persist in MobKnowledge, and an alert can renew a grant each update. The nominal timer/cooldown bounds opportunity pursuit, but does not establish expiry after a persistent alert. Characterize grant renewal without clearing genuine threat knowledge before changing that policy.
