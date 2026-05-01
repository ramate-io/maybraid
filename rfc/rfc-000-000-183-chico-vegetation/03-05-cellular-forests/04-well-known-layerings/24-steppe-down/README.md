# 3.5.4.24: Steppe Down

Steppe Down is a simple open steppe layering. Floor Scrub, Wild Grass, and Bush Scrub are common, while canopy and understory layers are mostly empty.

```rust
pub struct SteppeDown {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.0),
            (FloorScrub, 2.0),
            (Allbed, 0.5),
        ],
        flop: [
            (None, 5.0),
            (GrassyMounds, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 1.0),
        (WildGrass, 2.0),
        (BushScrub, 2.0),
    ],
    understory: UnderstoryLayer [
        (None, 8.0),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 10.0),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 10.0),
    ],
}
```

## Intent

Use Steppe Down for open scrub-steppe transitions where low vegetation carries the whole scene.
