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
wildlife so they do not join the chase. Hunt members forage and camp between
chases: after a herd lock (~45 s of focus) expires, the hunt host **browses**
waypoints for a few seconds before writing the herd goal again. Callers that
copy this should keep a similarly long lock; waypoint hops stay short.

No `LodSceneHost`. Personalities live on the members.

## What to watch

| Pack | Host | Members |
|---|---|---|
| occupy | green orb, stays put | grazers + civilians on camp/forage |
| watch | orange orb, stays put | brawlers on a gate cluster |
| herd | slow blue orb (~1.7 m/s) | grazers; lock onto waypoints after each hop, flee when hunt closes |
| hunt | red orb (~4 m/s) | predators + assassin; ~45 s lock on the herd, then browse forage / waypoints |

Sequence: magenta line = hunt traveling onto the herd. Magenta ring + HUD
`lock` = member tethers sitting on that destination. Red dots = Combat
(firearm range, no leash). Yellow dots = Evade (tether off). After lock
releases, hunt browses other POIs, then re-acquires the herd.

Yellow line = host travel goal. Green = member local POI. White = leash to
current tether subject.

## Controls

WASD fly, mouse look, Space / Shift up / down, Ctrl sprint.
