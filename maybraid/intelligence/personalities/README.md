# Personalities playground

Run:

```sh
cargo run -p personalities-playground --release
```

A **400 m square** pad (not 400 m²) with proto-mobs at ranges that matter to
the NPC stack: spotting (~80 m), High (~200 m), and beyond. The later mob
brain is still a roster; this playground is High fulfill with a shared tether
host per pack.

## Controls

WASD fly, mouse look, Space / Shift up / down, Ctrl sprint. The white capsule
is **public** and follows the camera look-at on the pad, so flying toward a
pack is how you change distance.

## Proto-mobs

| Pack | Where | Mix |
|---|---|---|
| herd | ~26 m | Grazers — Evade public |
| watch | ~33 m | Brawlers / guards — Combat inside spotting |
| flock | ~23 m | Grazers that do **not** antagonize public — stay Ignore and meander |
| occupy | ~56 m | Grazers + unarmed civilians |
| guard | ~82 m | Gate brawlers, small discovery radius |
| roam | ~127 m | Grazers / civilians, Ignore until you approach |
| ffa | ~126 m | Brawlers who antagonize each other |
| hunt | ~151 m | Predators + assassins, `keep_tether_in_combat` |
| monk | ~240 m | Lone grazer, wildlife affiliations — Ignore even up close |

Cyan ring = 80 m spotting. Magenta ring = 200 m High. Dots on NPCs are the
current threat tactic (green Ignore, yellow Evade, red Combat). HUD lists
per-pack counts and distance to public.

Each proto-mob has a **local POI cluster** inside its leash so Ignore can
meander between camp / forage / gate / pit. Visit cooldown ranks novelty; it
does not freeze a pack that only knows those local destinations. Grazers linger
about six seconds at a reached POI; brawlers about two.

Personalities keep their own spotting / discovery horizons (grazer ~40 m,
assassin ~72 m). Closing distance is what flips Ignore → Evade or Combat.
