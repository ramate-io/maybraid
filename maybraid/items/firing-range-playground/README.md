# Firing range

Kit firearms auto-fire emissive projectiles downrange. Three stations (bolt, bullet, laser) share the bullpup kit. Characters land here later.

```bash
cargo run -p firing-range-playground
```

Press `/` then `pause` / `resume`. `L` toggles look. WASD + Space fly.

| Kind | Behavior |
|------|----------|
| Bolt | Capsule, no gravity; dies on path / through-solid / age |
| Bullet | Same capsule, gravity on; same budgets (thinner penetration) |
| Laser | Beam grows from the barrel, then resets after `max_time` |
