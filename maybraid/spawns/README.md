# Specialized spawning systems

These crates are **not** a generic spawn framework. Each one solves a
gameplay problem that appears when a particular kind of object enters
the visual world:

- **Player arrival** — a life must be offered, a destination chosen, and
  the picker closed without assigning input too early.
- **Placement / support** — one bounded vertical search column per
  destination. Generated heights define the interval; collider geometry
  decides the pose. Interior uses the same probe with a tighter range.
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
  → Placement interpreter              (ResolvePlacement → pending column)
       wait for relevant collision presentation
       bounded support cast + standing-capsule clearance
       PlacementReady | PlacementFailed
  → Arrival stays pending on failure   (retry / other dest / explicit fallback)
  → Actor interpreter                  (SpawnActor → ActorSpawned)
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
| `spawns/player` | `maybraid-spawns-player` | Arrival identity, `PlacementAttempt`, picker, destination, cancellation, `ActorSpawned` → `ActivatePlayerArrival` |
| `spawns/placement` | `maybraid-spawns-placement` | Bounded column probe + presentation wait + standing clearance |
| `spawns/actors` | `maybraid-spawns-actors` | Thin `spawn_player_body` interpreter at the accepted pose; emits `ActorSpawned` |
| `spawns/projectiles` | `maybraid-spawns-projectiles` | Batched flight construction + same-pass visual attach |
| `spawns/grenades` | `maybraid-spawns-grenades` | Thrown-body construction |

`world/lib` keeps first-wave HCSG gates, the locked map sheet, and
activation interpreters (motor, camera, picker close). Placement physics
lives in `spawns/placement`. `gameplay/lib` assembles spawn runtimes.

## Scheduling

| Work | When | Notes |
|---|---|---|
| Picker capture / confirm | `CaptureResolveSystems` | UI-owned `MenuNav`. No actor required. |
| Placement observe | `PlacementSystems::Observe` | Column presentation + bounded support cast + capsule clearance. |
| Placement poll | `PlacementSystems::Poll` | Emits `PlacementReady` / `PlacementFailed`. |
| Arrival + construct | `PlayerLifeSet::Resolve` after poll | World writes `SpawnActor` on a matching ready + attempt. |
| Actor realize | `ActorSpawnSystems::Realize` | Emits `ActorSpawned`. |
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

1. Explicit request / session / placement-attempt identity. Stale outcomes are ignored.
2. Picker confirm is UI-owned `MenuNav`. No actor required.
3. Closing the picker releases only its capture token.
4. Placement requests persist across generation that outlives messages.
5. Support is a bounded downward cast onto an upward-facing collider
   inside the destination column. Missing presentation keeps the
   arrival pending with no body. Clearance tests the standing
   `LocomotionCapsule` and excludes only the supporting surface.
6. Placement failure preserves the arrival. The controller retries,
   picks another destination, or falls back explicitly.
7. Construction and activation are separate. Actors emit
   `ActorSpawned`; the player controller emits
   `ActivatePlayerArrival`. No live dynamic player while placement
   is pending.
8. Projectile / grenade interpreters do not rediscover the current
   player. Sim can run without presentation.
9. Production and headless paths share the same decision systems.
10. Regression tests drive the real input path (button → confirm →
    outcome) and production registration (sim + visual attachment).
