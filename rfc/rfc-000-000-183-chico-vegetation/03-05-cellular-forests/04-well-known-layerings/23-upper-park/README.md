# 3.5.4.23: Upper Park

Upper Park is an open parkland scrub layering. Floor Scrub, Wild Grass, and Bush Scrub are common. Rolling Oaks is less common.

```rust
pub struct UpperPark {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.0),
            (FloorScrub, 2.0),
            (Allbed, 0.5),
        ],
        flop: [
            (None, 5.0),
            (GrassyMounds, 0.5),
            (FleckingBed, 0.25),
        ],
    },
    tufts: TuftsLayer [
        (None, 1.0),
        (WildGrass, 2.0),
        (BushScrub, 2.0),
        (CommonTufts, 0.5),
    ],
    understory: UnderstoryLayer [
        (None, 6.0),
        (LowBush, 0.35),
        (SpottyBushes, 0.35),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 8.0),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 3.0),
        (RollingOaks, 1.0),
    ],
}
```

## Intent

Use Upper Park for open upland parkland where grass and scrub are the base, with oaks appearing as scattered canopy.
