# 3.5.4.11: Sun's Barren

Sun's Barren is an almost-empty layering. Most layers select `None`; Grassy Mounds rarely appears as the main visible vegetation.

```rust
pub struct SunsBarren {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 64.0),
            (GrassyMounds, 0.75),
        ],
        flop: [
            (None, 64.0),
            (FleckingBed, 0.25),
        ],
    },
    tufts: TuftsLayer [
        (None, 64.0),
        (CommonTufts, 0.25),
    ],
    understory: UnderstoryLayer [
        (None, 12.0),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 12.0),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 12.0),
    ],
}
```

## Intent

Use Sun's Barren for exposed, mostly vegetation-free areas where the occasional mound or fleck is enough.
