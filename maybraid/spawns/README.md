# Specialized spawning systems

These crates are **not** a generic spawn framework. Each one solves a
gameplay problem that appears when a particular kind of object enters
the visual world:

- **Player arrival** — a life must be offered, a destination chosen, and
  the picker closed without assigning input too early.
- **Placement / support** — a downward `SpatialQuery` shape cast must
  hit a supporting surface at *that* destination; AABBs are not occupancy.
- **Actor construction** — a body can be built without choosing a local
  player; activation is a later handoff.
- **Projectiles / grenades** — simulation is realized first (flight,
  fuse, damage). Presentation attaches mesh and flight audio afterward.

There is no `spawns/core` crate and no `spawns/population` crate.
Request IDs stay domain-owned. Population demand stays on barking /
mob-scenes and still calls `character.spawn` directly.

Tracking: [#1468](https://github.com/ramate-io/maybraid/issues/1468)
([#1469](https://github.com/ramate-io/maybraid/issues/1469)–[#1473](https://github.com/ramate-io/maybraid/issues/1473)).

## Flow inventory

```text
Session start / player retirement
  → Player arrival controller          (BeginPlayerArrival)
  → Spawn-picker controller            (UI-owned MenuNav, no actor)
       DestinationSelected
  → Placement interpreter              (ResolvePlacement → pending work)
       SpatialQuery support + capsule clearance
       PlacementReady | PlacementFailed
  → Arrival stays pending on failure   (retry / other dest / explicit fallback)
  → Actor interpreter                  (SpawnActor → ArrivalActorReady)
  → Player handoff controller          (emits ActivatePlayerArrival)
       → world motor / camera / picker close

Firearm trigger + cadence → WeaponShotRequest → SpawnProjectile
Grenade cook / release / drop → SpawnThrownGrenade
  → simulation interpreter (body / flight / fuse)
  → presentation interpreter (mesh / audio / VFX)
```

A **controller** decides and emits a request. An **interpreter** realizes
it. Intermediate outcomes are optional: high-volume projectiles do not
emit per-shot success events.

## Ownership

| Directory | Package | Owns |
|---|---|---|
| `spawns/player` | `maybraid-spawns-player` | Arrival identity, picker, destination, cancellation, `ArrivalActorReady` → `ActivatePlayerArrival` |
| `spawns/placement` | `maybraid-spawns-placement` | Floor / clearance / per-destination `SpatialQuery` support |
| `spawns/actors` | `maybraid-spawns-actors` | Thin `spawn_player_body` interpreter; emits `ArrivalActorReady` |
| `spawns/projectiles` | `maybraid-spawns-projectiles` | Batched flight construction + same-pass visual attach |
| `spawns/grenades` | `maybraid-spawns-grenades` | Thrown-body construction |

`world/lib` keeps first-wave HCSG gates, the locked map sheet, and
activation interpreters (motor, camera, picker close). Placement physics
lives in `spawns/placement`. `gameplay/lib` assembles spawn runtimes.

## Scheduling

| Work | When | Notes |
|---|---|---|
| Picker capture / confirm | `CaptureResolveSystems` | UI-owned `MenuNav`. No actor required. |
| Placement observe | `PlacementSystems::Observe` | Downward support cast + capsule intersection. |
| Placement poll | `PlacementSystems::Poll` | Emits `PlacementReady` / `PlacementFailed`. |
| Arrival + construct | `PlayerLifeSet::Resolve` after poll | World writes `SpawnActor` on a matching ready. |
| Actor realize | `ActorSpawnSystems::Realize` | Emits `ArrivalActorReady`. |
| Player handoff | `PlayerHandoffSystems::Activate` | Exactly-once `ActivatePlayerArrival`. |
| Projectile realize | `FirearmWeaponSystems::Fire` | Same-frame input → fire → flight (+ mesh when cache is present). |
| Projectile presentation | after realize | Fallback attach + flight fizz. Headless sim skips this. |
| Grenade presentation | after thrown-body realize | Mesh / bounce audio. |
| NPC members | existing cell batch | `character.spawn`; no per-member spawn events. |

Persistent work (arrivals, placement) lives in resources so generation
that outlives message retention cannot drop a request. Projectile and
grenade construction is synchronous and untracked.

## Dependency direction

```text
bevy / maybraid-input / controllers/core
        ▲
spawns/player
        ▲
spawns/placement     spawns/actors
spawns/projectiles   spawns/grenades
        ▲
gameplay/lib adapters + world/lib presentation / first-wave / map
barking / mob-scenes issue actor + placement requests directly
```

Spawn crates depend on world-query APIs (`world-player`,
`poi-intelligence`, `durham`, `projectiles`). They do **not** depend on
the assembled `world/lib` crate. Player does not depend on placement or
actors.

## Invariants

1. Explicit request / session identity. Stale outcomes are ignored.
2. Picker confirm is UI-owned `MenuNav`. No actor required.
3. Closing the picker releases only its capture token.
4. Placement requests persist across generation that outlives messages.
5. Support is a downward shape cast onto an upward-facing collider at
   the selected destination. An interior AABB or missing collider is
   not a floor. Clearance tests the standing capsule against world
   geometry and excludes only the supporting surface.
6. Placement failure preserves the arrival. The controller retries,
   picks another destination, or falls back explicitly.
7. Construction and activation are separate. Actors emit
   `ArrivalActorReady`; the player controller emits
   `ActivatePlayerArrival`.
8. Projectile / grenade interpreters do not rediscover the current
   player. Sim can run without presentation.
9. Production and headless paths share the same decision systems.
10. Regression tests drive the real input path (button → confirm →
    outcome) and production registration (sim + visual attachment).
