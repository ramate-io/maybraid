# 3.4.3.6: Allbed

This page is subsection **3.4.3.6** of [RFC-183: Chico Vegetation](../../../README.md)


Allbed is a broad, mixed ground-cover grove combining flecking and non-flecking bump outs, colliding and non-colliding surface layers, and [Grassy Mounds](../05-grassy-mounds/README.md#3435-grassy-mounds). It is the most general ground-cover bed and is useful when a region should feel lush, varied, and continuous without committing to a single ground-cover pattern.

Good for rich forest floors, riparian understory, old gardens, meadow edges, fantasy groves, and high-detail mixed biomes.

```rust
pub enum AllbedCell {
    HuelgoatPitch(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.85,
            steepness: 0.0..0.40,
        },
        item: HuelgoatPitchCell,
    }),
    FleckingBed(Bucket {
        weight: 2.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.85,
            steepness: 0.0..0.40,
        },
        item: FleckingBedCell,
    }),
    GrassyMound(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.85,
            steepness: 0.0..0.40,
        },
        item: GrassyMoundsCell,
    }),
    LowNonCollidingBumpOut(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.85,
            steepness: 0.0..0.40,
        },
        item: BumpOut {
            noise: NoiseProfile::LowSmooth,
            height: 0.05..0.12,
            collide: false,
            palette_mix: [
                dark_green..light_green,
                yellow_green..dry_green,
            ],
            flecking_mix: [],
        },
    }),
    CollidingFleckingBumpOut(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.85,
            steepness: 0.0..0.40,
        },
        item: BumpOut {
            noise: NoiseProfile::Moderate,
            height: 0.08..0.18,
            collide: true,
            palette_mix: [
                dark_green..light_green,
                yellow_green..dry_green,
            ],
            flecking_mix: [
                Flecking {
                    kind: FleckingKind::Bloom,
                    strength: LowToModerate,
                    ..Default::default()
                },
                Flecking {
                    kind: FleckingKind::Snowfall,
                    strength: Minimal,
                    ..Snowfall::common_flecking(world_size)
                },
            ],
        },
    }),
}

impl CellGrove for Allbed {
    type Cell = AllbedCell;

    const CELL_SIZE_RANGE: Range<f32> = 15.0..100.0;
    const DENSITY_RANGE: Range<f32> = 0.10..0.90;

    const OFFSET_RANGE: Range<f32> = 0.0..1.0;

    const NOISE_AMPLITUDE_RANGE: Range<f32> = 0.15..0.55;
    const NOISE_FREQUENCY_RANGE: Range<f32> = 0.008..0.060;
}
```

**Construction**

* Mix multiple bump-out forms rather than enforcing a single bed type.
* Include both colliding and non-colliding bump outs.
* Include both flecking and non-flecking variants.
* Add occasional grassy mounds for rounded volumetric breakup.
* Use moderate to mixed density, roughly `10%–90%`.
* Use larger cells for broad beds and smaller cells where more local variation is desired.
* At low LOD, collapse internal cell size to the full grove cell.

**Behavior**

* Colliding bump outs provide physical surface variation.
* Non-colliding bump outs provide visual softness without affecting traversal.
* Flecking variants provide seasonal blooms or snow.
* Grassy mounds add discrete rounded relief and break up continuous mats.

**Use**

Allbed is best treated as a high-variety default ground-cover grove. It should be used where the designer wants a rich floor texture but does not need a strong specific identity like pitch, scrub, or flowering bed.

