# Bikanwarda Ward / Enraged combat policies

Form-specific target selection and area-damage filters for summoned Ward and
Enraged ([#1137](https://github.com/ramate-io/maybraid/issues/1137)). Gameplay
is the reusable [`attacks`](../../attacks/README.md) crate plus the authored
Parade / God Beam / Bray ids in [`bikanwarda-attacks`](../../attacks/bikanwarda/README.md).

This crate is **not** installed by `maybraid-world`, the Self loop, or
`combat-hud`. Hosts that want the forms add [`BikanwardaLubikanPlugin`](src/plugin.rs).

```text
owner CombatTargeting / Affiliations / contacts
        │
        ▼
BikanwardaSummonPolicy (inspectable)
        │
        ├─ select intended target     (selection tests)
        └─ BeginAttack.allies         (victim / area-damage tests)
                │
                ▼
        bikanwarda.parade | god_beam | bray
```

Target choice and victim filtering are separate. Membership without
[`CombatContact`](../../intelligence/combat/targeting/src/contact.rs) is not
aimable. This crate never fabricates a sighting from a live transform.

## Agreed vs placeholder

| Fact | Source |
|---|---|
| Ward attacks the summoner's enemies | [#1137](https://github.com/ramate-io/maybraid/issues/1137) |
| Enraged may select and damage its summoner | [#1137](https://github.com/ramate-io/maybraid/issues/1137) |
| Both forms reuse Parade, God Beam, Bray | [#1106](https://github.com/ramate-io/maybraid/issues/1106), [#1230](https://github.com/ramate-io/maybraid/pull/1230) |
| Victim policy is `BeginAttack::allies` + authored `FriendlyFire` | [`bikanwarda-attacks` README](../../attacks/bikanwarda/README.md) |
| Recipes are not copied | same |

Open questions stay labeled PLACEHOLDER on [`BikanwardaSummonConfig`](src/policy.rs)
and in the crate rustdoc. They are not silently closed.

```shell
cargo test -p bikanwarda-lubikan --all-targets
```

The policy crate is located in `bikanwarda-combat` and consumes the shared
`bikanwarda-lubikans` charge catalog. Ward=1 and Enraged=2, capacity 100,
activation cost 25 and stock 4 are the same labeled placeholders used by charge
and encounter unlocks. The former separate 0xB1A0 ids are displaced so an
encounter unlock is the account that this summon policy consumes. Balance and
selection/victim policy questions above remain open.
