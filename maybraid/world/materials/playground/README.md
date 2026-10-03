# Maybraid World Materials Playground

Standard sphere plus a [`MaterialRef`](../../../material-ref). Uses the composed world material lib so named recipes resolve the same way they do in Maybraid World.

```text
cargo run -p world-materials-playground
cargo run -p world-materials-playground -- show pulse
cargo run -p world-materials-playground -- shoot
```

In-game (press `/`):

- `show hex|pulse|tail|muzzle-flame|standard` — authored energy / flame / Standard looks
- `show <recipe>` — any world recipe name (`furniture_wood`, …)
- `shoot` — fire repeating cylinders with the current recipe
- `shoot stop` — cancel the launcher
