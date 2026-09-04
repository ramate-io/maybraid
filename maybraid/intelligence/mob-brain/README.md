# Mob-brain playground

Run:

```sh
cargo run -p mob-brain-playground --release
```

A **360 m pad** mixing stationary packs with a **roaming herd** and a **hunt**
that tracks it. Each host is the tether. Hunt does not journey waypoints: the
playground points that host at a standoff on the herd tether. Members keep the
NPC mixer. Hunt antagonizes grazers; the herd antagonizes hunt. Occupy and
watch stay wildlife so they do not join the chase.

No `LodSceneHost`. Personalities live on the members.

## What to watch

| Pack | Host | Members |
|---|---|---|
| occupy | green orb, stays put | grazers + civilians on camp/forage |
| watch | orange orb, stays put | brawlers on a gate cluster |
| herd | slow blue orb (~1.7 m/s) | grazers browsing forage, then fleeing when hunt closes |
| hunt | red orb (~4 m/s) | predators + assassin; stalk the host, Combat the herd |

Sequence: magenta line = hunt tracking the herd center. Predators Combat
inside ~48 m. Grazers Evade inside ~24 m and peel off (tether drops on Evade).
Dots: green Ignore, yellow Evade, red Combat. HUD `gap` is host-to-host
distance; `I E C` are per-pack tactic counts.

Yellow line = host travel goal (waypoints for the herd, standoff for hunt).
Green = member local POI. White = leash.

## Controls

WASD fly, mouse look, Space / Shift up / down, Ctrl sprint.
