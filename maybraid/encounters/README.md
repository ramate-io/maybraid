# Encounters

Sole owner of **mutable encounter progress**, keyed by a stable identity that is
not an HCSG generated value.

```text
EncounterId (slug bytes)
        ↓
EncounterProgressStore   ← this crate is the only mutator
        ↓
EncounterRecord (lifetime, phase, outcome, live instance)
```

HCSG generation ([#1116](https://github.com/ramate-io/maybraid/issues/1116)) may
later reuse the same slug as `lod::gen::Id::Bytes`. This crate does not generate,
place, or present world hosts. Develop against [`fixtures`](src/fixtures.rs).

Gameplay does **not** depend on combat-hud. Presentation teardown is not a death
or reward event.

## Public types

| Type | Role |
|------|------|
| [`EncounterId`](src/identity.rs) | Stable slug identity. Not cell bounds. |
| [`EncounterInstance`](src/instance.rs) | Live fixture/runtime body for one id. |
| [`EncounterProgressStore`](src/store.rs) | The one progress owner. |
| [`EncounterRecord`](src/progress.rs) | Lifetime, phase, terminal outcome. |
| [`EncounterSignal`](src/signal.rs) | Chain-break, player death, session reset, explicit stream-out. |
| [`EncounterResolved`](src/signal.rs) | One terminal outcome per encounter. |
| [`EncounterLifetimePolicy`](src/lifetime.rs) | Stream-out, death, session, disk — explicit, not silent. |
| [`EncounterSleepPolicy`](src/sleep.rs) | Explicit 20%/15s/50%/10% knobs. Disabled unless Bikanwarda enables it. |
| [`EncounterSystems`](src/lib.rs) | `Ingest` → `Resolve` → `Phase`. Content awards order after `Resolve`. |

[`EncountersPlugin`](src/plugin.rs) is the only place that mutates the store.
Other lanes write [`EncounterSignal`](src/signal.rs) or spawn
[`EncounterInstance`](src/instance.rs). Boss [`damage::Died`](../damage/src/lib.rs)
on a bound instance is ingested as a kill.

## Policy (not silently finalized)

Defaults are labeled. Liam’s Oct 12 notes on
[#1121](https://github.com/ramate-io/maybraid/issues/1121) are applied as
[`EncounterLifetimePolicy::bikanwarda`](src/lifetime.rs). Open questions stay
in that file and in the PR body.

Disk persistence is [`EncounterDiskPersistence::SessionOnly`](src/lifetime.rs):
the store snapshots in memory. Character saves today cover appearance,
inventory bags, and position, not world encounter flags. Do not claim a disk
format here.

Sleep ([#1120](https://github.com/ramate-io/maybraid/issues/1120)) stores phase
on the progress owner. PLACEHOLDER Bikanwarda choices: one sleep per
occurrence, restore at end of 15 s, early wake cancels remaining recovery.
Phase is never inferred from current HP after a stream.
