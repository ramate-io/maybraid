# Mob-brain playground

Run:

```sh
cargo run -p mob-brain-playground --release
```

A **360 m pad** mixing **stationary** packs from the personalities playground
with **traveling** hosts. Each host is the tether. Members keep their NPC
mixer: Ignore + a satisfied leash meanders local POIs; an unsatisfied tether
drops that goal so they catch up. Hunt members have empty interests, so they
stay on the stalk.

No `LodSceneHost`. Personalities live on the members; the mob brain moves the
tether when the pack travels.

## What to watch

| Pack | Host | Members |
|---|---|---|
| occupy | green orb, stays put | grazers + unarmed civilians on camp/forage in the leash |
| watch | orange orb, stays put | brawlers on a gate cluster |
| roam | slow blue orb (~1.7 m/s) | grazers + civilians; linger on forage along the pad, then catch the host |
| hunt | fast red orb (~5.4 m/s) | predators + assassin; stalk the host, no local meander |

Yellow line = host journey `PoiGoal`. Green line = member local `PoiGoal`.
White lines = members to host. Small green/blue/amber spheres = local POIs;
larger yellow spheres = waypoints. HUD `poi` is how many members currently
have a local goal; `stretch` is the farthest member from the host.

Roam is the mixer demo: grazer linger (~6 s) plus a slow host so people peel
off to forage, then the leash pulls them back. Hunt is the tight contrast.

## Controls

WASD fly, mouse look, Space / Shift up / down, Ctrl sprint.
