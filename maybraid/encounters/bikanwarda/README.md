# Bikanwarda encounter outcomes

Maps the chained Bikanwarda identity onto Lubikan unlocks. Progress lives in
[`encounters`](../README.md). This crate does not generate world placement
([#1116](https://github.com/ramate-io/maybraid/issues/1116) /
[#1117](https://github.com/ramate-io/maybraid/issues/1117)) and does not depend
on combat-hud.

| Outcome | Unlock |
|---------|--------|
| Kill | Enraged (`LubikanDefinitionId` 1) |
| Chain break / free | Ward (`LubikanDefinitionId` 2) |

One occurrence awards exactly one of them. Duplicate signals do not add a
second ledger account. Separate occurrences can award the other, so the
inventory can hold both.

After a free, this crate emits [`BikanwardaDeparted`](src/outcomes.rs) with
`Transcend` (glow and leave). Kill uses ordinary death. No VFX here.

Unlocks go through [`LubikanLedger::ensure_owned`](../../lubikans/src/ledger.rs)
on the signal source. They do **not** clone [`LubikanSessionStore`](../../lubikans/src/plugin.rs).

Sleep/recovery is owned by [`encounters::EncounterSleepPolicy::bikanwarda`](../src/sleep.rs).
This plugin enables it. Presentation uses the Epiphant sleep/wake clips from
[#1141](https://github.com/ramate-io/maybraid/pull/1141) when `AnimRefRoot` is
present; fixtures without a clip still run gameplay.

### Shared content catalog

Unlocks resolve the stable character ledger through the player's body link.
Their ids and default placeholder balance row now come from
`bikanwarda-lubikans`: Ward=1, Enraged=2, capacity100 / cost25 / stock4. This
displaces the older reversed ids and isolated512 /512 defaults; it prevents
charge, unlock and summon registration from describing different accounts.
Balance remains configurable and provisional.
