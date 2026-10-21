# Specialized spawning systems

These crates are **not** a generic spawn framework. Each one solves a
gameplay problem that appears when a particular kind of object enters
the visual world:

- **Player arrival** — a life must be offered, a destination chosen, and
  the picker closed without assigning input too early.
- **Placement / support** — the chosen floor is not ready until the
  supporting terrain or building collider exists at *that* destination.
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
       per-destination support / clearance
       PlacementReady | PlacementFailed
  → Arrival stays pending on failure   (retry / other dest / explicit fallback)
  → Actor interpreter                  (SpawnActor → ActorSpawned)
  → Player handoff controller          (consumes ActorSpawned)
       → motor / camera / picker close

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
| `spawns/player` | `maybraid-spawns-player` | Arrival identity, picker, destination, cancellation, handoff contracts |
| `spawns/placement` | `maybraid-spawns-placement` | Floor / clearance / per-destination support |
| `spawns/actors` | `maybraid-spawns-actors` | Thin `spawn_player_body` interpreter; consumes `ActorSpawned` for handoff |
| `spawns/projectiles` | `maybraid-spawns-projectiles` | Batched flight construction + visual spec |
| `spawns/grenades` | `maybraid-spawns-grenades` | Thrown-body construction |

`world/lib` keeps first-wave HCSG gates, the locked map sheet, and
destination-specific support probes (terrain trimesh / building walk
colliders). It does not invent a second spatial index. `gameplay/lib`
assembles spawn runtimes. Item presentation plugins attach meshes and
audio.

## Scheduling

| Work | When | Notes |
|---|---|---|
| Picker capture / confirm | `CaptureResolveSystems` | UI-owned `MenuNav`. No actor required. |
| Placement observe | `PlacementSystems::Observe` | One readiness per pending pose. |
| Placement poll | `PlacementSystems::Poll` | Emits `PlacementReady` / `PlacementFailed`. |
| Arrival + construct | `PlayerLifeSet::Resolve` after poll | Construction waits for a matching ready. |
| Actor handoff | `Update` on actor runtime | Consumes `ActorSpawned` exactly once. |
| Projectile realize | `FirearmWeaponSystems::Fire` | Same-frame input → fire → flight. |
| Projectile presentation | after realize | Cached mesh + flight fizz. Headless sim skips this. |
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
5. Support is evaluated at the selected destination's collider, not a
   global ready bit. An interior floor coordinate does not prove its
   building collider is ready.
6. Placement failure preserves the arrival. The controller retries,
   picks another destination, or falls back explicitly.
7. Construction and activation are separate. Handoff consumes
   `ActorSpawned`.
8. Projectile / grenade interpreters do not rediscover the current
   player. Sim can run without presentation.
9. Production and headless paths share the same decision systems.
10. Regression tests drive the real input path (button → confirm →
    outcome) and production registration (sim + visual attachment).
