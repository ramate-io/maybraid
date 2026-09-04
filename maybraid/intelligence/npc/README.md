# NPC intelligence

Shared NPC mixer plus named personality constructors.

Personalities are **data and which components exist**. They do not classify
threats or pick POI kinds. [`ThreatManagementIntelligence`](../threat-management)
still grants Ignore | Evade | Combat. This crate only exclusive-grants the
idle stack:

```text
1. Threat tactic ≠ Ignore  → combat or evade actuators
2. Tether unsatisfied      → leash / stalk (when Ignore, or Combat if kept)
3. Meander                 → local POI goal
```

[`mix_npc_brains`](src/plugin.rs) runs after threat-management selection.
Combat and Evade disable meandering and drop an active `PoiGoal`. Tether is
disabled during Evade and during Combat unless `keep_tether_in_combat` is set
(Hunt can flip that without changing the personality). Firearm movement and
flee/hide already no-op when their grants are retracted.

## Personalities

| Kind | Combat brains | Evade | Idle tether |
|---|---|---|---|
| Grazer | none | flee + hide, far ignore | large leash slack |
| Civilian | optional (skip if unarmed) | flee + hide, weak combat scores | medium leash |
| Predator | yes | evade when wounded | stalk → inner leash stored for combat |
| Brawler | scrappy firearm | flee only | short leash |
| Assassin | patient firearm | hide-biased | stalk annulus |

Mobs override `threat_override`, affiliations, POI interests, the tether
subject, rules of engagement, and `armed`. See [HORIZON.md](HORIZON.md) for
groups, mob hosts, and LodScene.

The [personalities playground](../personalities) is a 400 m square High-fulfill
smoke: each pack is a [`mob-intelligence`](../mob) host, members bind through
`MobSlot`, and flying the camera moves public so spotting distance can flip
tactics.
