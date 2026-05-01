# 3.5.4.22: West Maui

West Maui is an open tropical scrub layering. Floor Scrub, Wild Grass, Bush Scrub, and Tropical Tufts are common. Wandering Acacia is less common.

```rust
pub struct WestMaui {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.0),
            (FloorScrub, 2.0),
        ],
        flop: [
            (None, 4.0),
            (FleckingBed, 0.5),
            (GrassyMounds, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 1.0),
        (WildGrass, 2.0),
        (BushScrub, 2.0),
        (TropicalTufts, 2.0),
    ],
    understory: UnderstoryLayer [
        (None, 5.0),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 8.0),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 3.0),
        (WanderingAcacia, 1.0),
    ],
}
```

## Intent

Use West Maui for warm, open, wind-exposed terrain where grasses and scrub dominate, and small dry trees appear occasionally.
