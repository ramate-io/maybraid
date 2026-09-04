# Mob-brain playground

Run:

```sh
cargo run -p mob-brain-playground --release
```

A **360 m pad** mixing stationary packs with a **roaming herd** and a **hunt**
that tracks it. Each host is the pack tether. After a host arrives, member
tethers **lock** onto the destination (waypoint or herd host) for the linger,
then restore to the own host. Combat and Evade still own NPC movement; tether
does not write during those tactics.

Hunt antagonizes grazers; the herd antagonizes hunt. Occupy and watch stay
wildlife so they do not join the chase.

No `LodSceneHost`. Personalities live on the members.

## What to watch

| Pack | Host | Members |
|---|---|---|
| occupy | green orb, stays put | grazers + civilians on camp/forage |
| watch | orange orb, stays put | brawlers on a gate cluster |
| herd | slow blue orb (~1.7 m/s) | grazers; lock onto waypoints after each hop, flee when hunt closes |
| hunt | red orb (~4 m/s) | predators + assassin; lock onto the herd host after closing |

Sequence: magenta line = hunt traveling onto the herd. Magenta ring + HUD
`lock` = member tethers sitting on that destination. Red dots = Combat
(firearm range, no leash). Yellow dots = Evade (tether off). After lock
releases, members gather on their own host again.

Yellow line = host travel goal. Green = member local POI. White = leash to
current tether subject.

## Controls

WASD fly, mouse look, Space / Shift up / down, Ctrl sprint.
