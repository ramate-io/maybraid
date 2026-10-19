# Mythical chain playground

Preview host for [`chain-shaders`](../chain) / [#1124](https://github.com/ramate-io/maybraid/issues/1124).
Two independently parameterized placeholder chains, a continuous bent tube, a
posed skinned tube (halo skinning contract), a mundane wood cube, durability
sweep, CPU bend, day/night lighting, and the **real** Native / Sharp / Retro /
Antediluvian offscreen present path (`FollowCamera` + `VisualStylePlugin`).
The left chain is an [`OwnedChainMaterial`](../chain/src/runtime.rs)
so localized hits update a unique uniform instead of restamping the cache.

```text
cargo run -p chain-playground
cargo run -p chain-playground -- show pair
cargo run -p chain-playground -- lighting night
cargo run -p chain-playground -- deform on
cargo run -p chain-playground -- hit 6
cargo run -p chain-playground -- style retro
cargo run -p chain-playground -- capture /tmp/chain-previews
cargo run -p chain-playground -- video /tmp/chain-video
```

The default camera matches gameplay third-person (`FollowCamera` distance
3.6 m, 45° FOV). Gameplay cameras have no bloom; the halo is an additive
skinned shell. Video capture advances simulation time at **30 Hz**
(`TimeUpdateStrategy::ManualDuration(1/30 s)`) after shader warmup; do not
claim smoothness from a sparser frame dump.

In-game (press `/`):

- `show pair` — intact + damaged (default)
- `show intact|damaged|near-breaking|released` — both chains that state
- `left <state>` / `right <state>` — set one side
- `lighting day|night`
- `deform on|off` — sine-bend the placeholder mesh
- `sweep` / `sweep stop` — animate left-chain durability
- `seed <n>` — restamp both seeds (`right` uses `n + 18`)
- `glow <gain>` — glow intensity on both
- `hit <link>` — persistent per-link damage + hit flash on the left chain
- `style native|sharp|retro|antediluvian` — real world visual style
- `reverse` / `freeze` — signed / zero flow speed
- `capture <dir>` — damage strip, emission-off, day/night, style stills, then exit
- `video <dir>` — 30 Hz traveling-energy frames + stills, then exit

Geometry and parameter ranges: [`chain-shaders` README](../chain/README.md).
