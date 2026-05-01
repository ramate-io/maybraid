# 3.5.4.20: Damas Edge

Damas Edge is a dry-tropical transition layering. Levantine Scrub and Date Grove are less common, while Palm Shade, Strange Oasis, Tropical Undergrowth, Unending Jungle, and Tropical Thicket are rare.

```rust
pub struct DamasEdge {
    ground_cover: GroundCoverLayer {
        flip: [
            (None, 4.0),
            (FloorScrub, 1.0),
            (Allbed, 0.5),
        ],
        flop: [
            (None, 7.0),
            (FleckingBed, 0.25),
            (GrassyMounds, 0.25),
        ],
    },
    tufts: TuftsLayer [
        (None, 5.0),
        (BushScrub, 0.75),
        (TropicalTufts, 0.35),
    ],
    understory: UnderstoryLayer [
        (None, 4.0),
        (LevantineScrub, 1.0),
        (TropicalUndergrowth, 0.35),
        (TropicalThicket, 0.35),
    ],
    lower_canopy: LowerCanopyLayer [
        (None, 5.0),
        (StrangeOasis, 0.35),
        (UnendingJungle, 0.35),
    ],
    upper_canopy: UpperCanopyLayer [
        (None, 4.0),
        (Orchard, 0.5)
        (DateGrove, 1.0),
        (PalmShade, 0.35),
    ],
}
```

## Intent

Use Damas Edge where desert, oasis, and tropical vegetation meet at low frequency, with dry scrub still setting the baseline.
