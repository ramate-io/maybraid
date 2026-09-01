# Firing range

Kit firearms auto-fire emissive projectiles downrange. Three stations (bolt, bullet, laser) share the bullpup kit. Characters land here later.

```bash
cargo run -p firing-range-playground
```

Press `/` then `pause` / `resume`. `L` toggles look. WASD + Space fly.

| Kind | Behavior |
|------|----------|
| Bolt | Capsule, no gravity, despawns at max range |
| Bullet | Same capsule, gravity on, despawns at max range |
| Laser | Beam grows from the barrel, then resets after `max_time` |
