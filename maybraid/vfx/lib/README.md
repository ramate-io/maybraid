# Maybraid VFX

Real-time VFX: GPU particles, flipbooks, and short-lived composite hosts.

Shaders and [`MaterialRef`](../../material-ref) recipes stay in their domain crates. This crate is the effect graph. First delivery is the fiery explosion from [#945](https://github.com/ramate-io/maybraid/issues/945): reusable `flash`, `fireball`, `smoke`, and `sparks` layers, composed as `firey_explosion`.

```text
cargo test -p maybraid-vfx
cargo run -p vfx-playground
```

Spawn through `VfxLibrary` and `Commands::spawn_vfx`. Scale is spatial only; intensity scales particle count. Shared Hanabi assets are never mutated per instance.
