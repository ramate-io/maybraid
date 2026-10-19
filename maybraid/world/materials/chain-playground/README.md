# Mythical chain playground

Preview host for [`chain-shaders`](../chain) / [#1124](https://github.com/ramate-io/maybraid/issues/1124).
Two independently parameterized placeholder chains, durability sweep,
CPU bend (stand-in for A04 skinning), day/night lighting, and Sharp / Retro /
Antediluvian window sizes. The left chain is an [`OwnedChainMaterial`](../chain/src/runtime.rs)
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
skinned shell.

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
- `style sharp|retro|antediluvian` — window resolution stand-in for visual styles
- `capture <dir>` — write pair day/night, near-breaking, released, and mixed-pair PNGs, then exit
- `video <dir>` — gameplay-distance hit + pulse stills, then exit (host records ffmpeg)

Geometry and parameter ranges: [`chain-shaders` README](../chain/README.md).
