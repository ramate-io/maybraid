# Spawn orchestration

Tracking: [#1468](https://github.com/ramate-io/maybraid/issues/1468).
This document is the stacked phase 1–5 contract
([#1469](https://github.com/ramate-io/maybraid/issues/1469),
[#1470](https://github.com/ramate-io/maybraid/issues/1470),
[#1471](https://github.com/ramate-io/maybraid/issues/1471),
[#1472](https://github.com/ramate-io/maybraid/issues/1472),
[#1473](https://github.com/ramate-io/maybraid/issues/1473)).
Later phases stack on it; they should update symbol mappings when they move
a writer, not invent a second inventory.

A **controller** decides what should happen and emits a request. An
**interpreter** realizes it. A controller can consume an intermediate
outcome and issue the next request.

There is no universal `SpawnIntent` enum and no registry of every spawnable
type. [`spawns/core`](core/) is optional for simple consumers: shared
identity and outcome vocabulary only. Domain requests stay in their spawn
crates.

## Flow

Player arrival (from [#1468](https://github.com/ramate-io/maybraid/issues/1468)):

```text
Session start / player retirement
  → Player arrival controller          (BeginPlayerArrival)
  → Spawn-picker controller            (UI-owned MenuNav, no actor)
       destination selected
  → Arrival                            (ResolvePlacement — phase 2)
  → Placement interpreter              (PlacementReady / Failed)
  → Arrival                            (SpawnActor — phase 3)
  → Actor interpreter                  (ActorSpawned / failed)
  → Player handoff controller
       → Input assignment interpreter
       → Camera interpreter
       → Picker / arrival UI interpreter
```

Projectile / grenade (phase 4):

```text
Firearm trigger + cadence → WeaponShotRequest → SpawnProjectile
Grenade cook / release / drop → SpawnThrownGrenade
  → simulation interpreter (body / flight / fuse)
  → presentation interpreter (mesh / audio / VFX)
```

Population (phase 5):

```text
Cell / encounter demand → placement + actor requests
Retirement → cancellation / despawn
```

## Dependency direction

```text
bevy / maybraid-input / controllers/core
        ▲
        │
spawns/core          (identity + outcomes — no game deps)
        ▲
        │
spawns/player        spawns/placement        spawns/actors
spawns/projectiles   spawns/grenades         spawns/population
        ▲
        │
gameplay/lib adapters + world/lib presentation / first-wave / map
```

Rules:

- Spawn crates depend on lower-level world query / presentation APIs
  (`world-player`, `poi-intelligence`, `durham`, `projectiles`).
  They do **not** depend on the assembled `world/lib` crate.
- Persistent state lives in the owning domain crate.
- `gameplay/lib` assembles spawn plugins and thin cross-domain adapters.
- Production and headless / test paths share the same authoritative
  decision systems.

## Crate layout

| Directory | Package | Owns | Phase |
|---|---|---|---|
| `spawns/core` | `maybraid-spawns-core` | `SpawnRequestId`, `SpawnSessionId`, `SpawnOwner`, cancel / failure reasons | 1 |
| `spawns/player` | `maybraid-spawns-player` | Arrival controller, picker, destination selection, handoff requests | 1 |
| `spawns/placement` | `maybraid-spawns-placement` | Floor / clearance / support | **2** |
| `spawns/actors` | `maybraid-spawns-actors` | `SpawnActor` / `ActorSpawned` | **3** |
| `spawns/projectiles` | `maybraid-spawns-projectiles` | Simulation projectile spawn | **4** |
| `spawns/grenades` | `maybraid-spawns-grenades` | Thrown-grenade body / fuse / source | **4** |
| `spawns/population` | `maybraid-spawns-population` | Cell / encounter demand and retirement | 5 |

`world/lib` keeps terrain generation, POI indexing, streamed presentation,
first-wave HCSG gates, and the locked map sheet. Loadout persistence stays
with its persistence owner. Application / session transitions stay in
`game` / `gameplay/session`.

## Role inventory

| Role | Symbol | Now | Phase |
|---|---|---|---|
| Request identity | `SpawnIdentity` | `spawns/core` | 1 |
| Player arrival controller | `begin_player_arrivals`, `WorldPlayerRespawnState` | `spawns/player` | 1 |
| Spawn-picker controller | `SpawnPickerController` | `spawns/player` | 1 |
| Arrival phases | `ArrivalPhase` | `spawns/player` | 1 |
| Destination selected | `DestinationSelected`, `PlayerChoseRespawnPoi` | `spawns/player` | 1 |
| First-wave / resume gates | `FirstWave`, `PlayerPositionWaypoints` | `world/lib` adapters | 1 |
| Map presentation | `WorldMapView` | `world/lib` | 1 |
| Placement interpreter | `ResolvePlacement`, `PlacementWork`, `PlacementSupport` | `spawns/placement` | **2** |
| Actor interpreter | `SpawnActor`, `ActorSpawned` | `spawns/actors` | **3** |
| Player handoff | `ActivatePlayerArrival` | `spawns/player`; world applies motor / camera | **3** |
| Projectile interpreter | `SpawnProjectile` | `spawns/projectiles` | **4** |
| Grenade interpreter | `SpawnThrownGrenade` | `spawns/grenades` | **4** |
| Population controller | cell demand | `barking` / `mob-scenes` | 5 |

## Player arrival

Each arrival has a request ID and session owner. Requests and outcomes
carry that identity. A late result from a previous life or session cannot
activate an actor in the new session. Messages announce transitions;
pending work lives in `WorldPlayerRespawnState`.

```text
ArrivalPhase {
  ChoosingDestination,
  ResolvingPlacement,
  AwaitingSupport,
  SpawningActor,
  ActivatingActor,
}
```

The picker works without a player entity. Its input target is the picker
UI / session. A/Confirm selects a destination only while that picker owns
focus. It never depends on Jump, actor assignment, or locomotion. Closing
the picker releases its own capture token; the confirming button press
must not become a jump on the newly assigned actor.

This builds on the UI-owned [`MenuNavPad`](../input/src/surface/menu.rs)
picker from [#1461](https://github.com/ramate-io/maybraid/issues/1461).
It does not restore character intents without an actor or exempt Jump from
map capture.

## Invariants

1. Explicit request / session identity. Stale outcomes are ignored.
2. Picker confirm is UI-owned `MenuNav`. No actor required.
3. Closing the picker releases only its capture token.
4. Placement requests persist across generation that outlives messages (phase 2).
5. Construction and activation are separate (phase 3).
6. Projectile / grenade interpreters do not rediscover the current player (phase 4).
7. Production and headless paths share the same decision systems.
8. Regression tests drive the real input path (button → confirm → outcome).
