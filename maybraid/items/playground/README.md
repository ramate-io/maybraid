# Item playground

Viewer for kit-assembled items. Starts with the shared firearm receiver and a [`FirearmKit`](../firearms/src/kit.rs) (required body; optional barrel / trigger-box / grip / stock / sight).

```bash
cargo run -p items-playground
```

Press `/` then:

- `show bullpup` — named concept preset (`silopup`, `reltor`, …)
- `kit --body silopup --barrel laznard --trigger-box paddle --grip none --sight holorand` — set slots; omitted flags keep the current kit; `none` clears an optional part
- `scale barrel --length 1.5 --thickness 0.8` — length is bone local Y, thickness is XZ; bones are `body`, `barrel`, `trigger-box`, `grip`, `stock`

`L` toggles look. WASD + Space fly.
