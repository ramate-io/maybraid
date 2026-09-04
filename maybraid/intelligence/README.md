# Intelligence

Higher-order brains write objectives; lower-order crates field them.

## Movement

- [`movement-intelligence`](movement/lib) — [`MovementIntelligence`](movement/lib/src/user.rs) on a capsule. It fields [`MovementObjective`](movement/lib/src/objective.rs) and writes [`MoveWish`](../player/src/body.rs). It does not lock onto other entities.
- [`movement-intelligence-avian`](movement/avian) — collider-backed surface over Avian `Fixed` geometry.
- [`movement-intelligence-richmond`](movement/richmond) — composes the Avian surface with Les Halles storey / stairwell IR.

A higher-order system writes the objective and inserts [`ReplanMovement`](movement/lib/src/user.rs) when it wants a new plan. Budget and vantage *sampling* live on [`MovementAbility`](movement/lib/src/ability.rs). Hide / sightline *policy* belongs on the writer (firearm movement, etc.).

Walk colliders for Richmond IR live in [`richmond-building-physics`](../richmond/building-physics).

## Spotting

- [`spotting-intelligence`](spotting/lib) — semantic subjects, persistent interests and
  explicit subject hints, bounded discovery / respotting policy, and per-user visibility memory.
- [`spotting-intelligence-avian`](spotting/avian) — `Animated` broadphase discovery and Fixed-only sightline probes.

Spotting deliberately resolves a known subject's exact live location at probe time. Its memory records when visibility last succeeded and when another attempt is due; fresh contacts can satisfy a directive and skip discovery work. Position uncertainty is deferred to a higher-fidelity model.

## Combat

- [`combat-targeting`](combat/targeting) — combat contact memory, source-owned active-set membership, factor algebra, decaying influences, continuity, and cached weight ranking.
- [`firearm-intelligence`](combat/firearm) — adapts spotted character contacts into combat targets, contributes firearm opportunity, writes movement / look, validates posed-muzzle aim trajectories, and gates the actual trigger.

The layers form `(semantic broadphase + explicit hints) → visual contact memory → combat contact and weighted target set → firearm trajectory choice`. Applications own cadence; the reusable plugins remain cadence-neutral.

## Evasion

- [`evasion-intelligence`](evasion) — assailant memory, source-owned membership, and an exclusive hide | flee signal. This is the civilian analogue of combat targeting, not a movement mixer.
- [`fleeing-intelligence`](fleeing) — writes `FleeFrom` while the signal is flee.
- [`hiding-intelligence`](hiding) — writes `Reach` to a nearby low-vantage, low-occupancy pocket while the signal is hide.

The layers form `(semantic broadphase + explicit hints) → visual contact memory → assailant rank + signal → hide | flee → movement objective`. Combat `CHARACTER` subjects and civilian subjects stay on distinct interest layers so firearm targeting does not discover bystanders.
