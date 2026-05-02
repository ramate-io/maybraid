# 3.5.4.19: Fruit Plains

Fruit Plains is an open, cultivated plain layering with common Huelgoat Pitch and Allbed ground cover. Rolling Oaks, Orchard, Vineyard, and Date Grove are less common canopy events.

```rust
pub struct FruitPlains {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 1.0),
            (HuelgoatPitch, 2.0),
            (Allbed, 2.0),
        ],
        flop: [
            (None, 5.0),
            (FleckingBed, 0.5),
            (GrassyMounds, 0.5),
        ],
    },
    tufts: TuftsLayer [
        (None, 4.0),
        (CommonTufts, 0.75),
        (WildGrass, 0.5),
    ],
    understory: UnderstoryLayer [
        (None, 6.0),
        (BraidGrass, 0.5),
        (LowBush, 0.25),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 8.0),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 3.0),
        (RollingOaks, 0.75),
        (Orchard, 0.75),
        (Vineyard, 0.75),
        (DateGrove, 0.75),
    ],
}
```

## Intent

Use Fruit Plains for broad open land where fruiting or cultivated tree structure appears in patches rather than dominating the whole cell.
