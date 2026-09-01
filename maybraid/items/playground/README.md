# Item playground

Viewer for kit-assembled items. Starts with the shared firearm receiver and a [`FirearmKit`](../firearms/src/kit.rs) (required body; optional barrel / trigger-box / grip / stock).

```bash
cargo run -p items-playground
```

Press `/` then:

- `show bullpup` — named concept preset (`silopup`, `keelripe`, …)
- `kit --body silopup --barrel laznard --grip none` — set slots; omitted flags keep the current kit; `none` clears an optional part

`L` toggles look. WASD + Space fly.
