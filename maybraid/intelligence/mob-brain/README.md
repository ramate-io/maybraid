# Mob-brain playground

Run:

```sh
cargo run -p mob-brain-playground --release
```

A **360 m pad** where pack **hosts** journey between global waypoints. Each
host is the tether. Members install with that host as subject, so when the
center walks, NPCs are pulled along (leash for grazers, stalk for hunt).

This is still a flat High-fulfill stand-in: no `LodSceneHost`. Personalities
live on the members; the mob brain moves the tether.

## What to watch

| Pack | Host | Members |
|---|---|---|
| herd | slow green orb (~2.4 m/s) | five grazers, tight leash |
| roam | blue orb (~4.2 m/s) | four grazers, looser leash |
| hunt | red orb (~6 m/s) | predators + assassin, stalk the host |

Yellow line = current journey `PoiGoal`. White lines = members to host.
Yellow spheres = waypoints. HUD lists host xz, destination, count, and how
far the farthest member has stretched.

Members do **not** pick those long-range waypoints themselves. Ignore + tether
is the idle stack, so reroute is the host walking out of the leash.

## Controls

WASD fly, mouse look, Space / Shift up / down, Ctrl sprint.
