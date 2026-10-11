# Mythical chain playground

Preview host for [`chain-shaders`](../chain) / [#1124](https://github.com/ramate-io/maybraid/issues/1124).
Two independently parameterized placeholder chains, durability sweep,
CPU bend (stand-in for A04 skinning), and day/night lighting.

```text
cargo run -p chain-playground
cargo run -p chain-playground -- show pair
cargo run -p chain-playground -- lighting night
cargo run -p chain-playground -- deform on
cargo run -p chain-playground -- capture /tmp/chain-previews
```

In-game (press `/`):

- `show pair` — intact + damaged (default)
- `show intact|damaged|near-breaking|released` — both chains that state
- `left <state>` / `right <state>` — set one side
- `lighting day|night`
- `deform on|off` — sine-bend the placeholder mesh
- `sweep` / `sweep stop` — animate left-chain durability
- `seed <n>` — restamp both seeds (`right` uses `n + 18`)
- `glow <gain>` — glow intensity on both
- `capture <dir>` — write pair day/night, near-breaking, released, and mixed-pair PNGs, then exit

Geometry and parameter ranges: [`chain-shaders` README](../chain/README.md).
