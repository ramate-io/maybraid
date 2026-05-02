# 3.5.4.6: Mi Robles

Mi Robles is an open oak layering with common Rolling Oaks over Allbed ground cover. It keeps understory rare, using Low Bush, Spotty Bushes, and Riverine Green as occasional accents.

```rust
pub struct MiRobles {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.0),
            (Allbed, 3.0),
            (GrassyMounds, 0.5),
        ],
        flop: [
            (None, 5.0),
            (FleckingBed, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 3.0),
        (CommonTufts, 0.75),
        (WildGrass, 0.5),
    ],
    understory: UnderstoryLayer [
        (None, 5.0),
        (LowBush, 0.5),
        (SpottyBushes, 0.5),
        (RiverineGreen, 0.5),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 5.0),
        (GoettingenFollow, 0.5),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 1.5),
        (RollingOaks, 3.0),
    ],
}
```

## Intent

Use Mi Robles for open oak land with a simple ground plane and only occasional brushy interruption.
