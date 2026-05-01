# 3.4.6.4: Goettingen Follow

Goettingen Follow is a low-density lower-canopy grove using common [Braid Oak](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-13-braid-oak/README.md) and common [Storybook Tree](../../../03-01-stalk-and-ball-stick-trees/03-01-07-well-known-tree-constructions/03-01-07-01-storybook-tree/README.md) variants.

It represents a calm follow-layer beneath taller trees: small oaks and rounded storybook forms appear often enough to guide the eye and fill the subcanopy, but with enough spacing that the upper canopy still owns the forest.

Good for temperate woodland interiors, park-like forests, old paths, village edges, lower canopy beneath large oaks, and gentle fantasy groves.

```rust
pub enum GoettingenFollowCell {
    FollowBraidOak(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.05..0.68,
            steepness: 0.0..0.42,
        },
        item: BraidOak {
            height: 4.0..7.0,
            canopy_density: Moderate,
            palette_mix: [[deep_green..fresh_green], [dark_green..light_green]],
        },
    }),
    FollowStorybook(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.05..0.70,
            steepness: 0.0..0.54,
        },
        item: StorybookTree {
            height: 4.0..7.0,
            canopy_density: Moderate,
            palette_mix: [[broadleaf_green..light_green], [deep_green..yellow_green]],
        },
    }),
}

impl CellGrove for GoettingenFollow {
    type Cell = GoettingenFollowCell;

    const CELL_SIZE_RANGE: Range<f32> = 8.0..16.0;
    const DENSITY_RANGE: Range<f32> = 0.10..0.28;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.08..0.28;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.012..0.050;
}
```

## Construction

* Use low-density placement, roughly `10%–28%`.
* Use Braid Oak and Storybook Tree evenly at `4m–7m`.
* Keep canopy density moderate; this grove should fill the lower canopy without becoming a full tree layer.
* Use temperate broadleaf palettes with subtle variation rather than strong tropical or dry scrub colors.
* Let variants tolerate moderate slope, but avoid extreme steepness where lower canopy would feel unstable.
* Use [Bucket Throw](../../03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) varietal selection.

## Usage

* Use where a larger forest needs a sparse lower follow-layer.
* Pair with High Bush, Low Bush, Huelgoat Pitch, Allbed, fallen logs, and taller upper-canopy trees.
* Works well along paths and forest interiors where repeated small trees should lead the eye.
* Avoid high density; the grove should feel like a supporting lower layer, not the main canopy.
