# Spear tuft

Thin flat grass blades — **2D ribbons** projecting upward from a shared anchor.

## Role

Grass-like spear tufts inspired by terrain [`GrassTuft`](https://github.com/ramate-io/maybraid/blob/main/procedures/terrain/src/detail/meshes/tuft.rs): each blade is a single quad strip, very thin in one lateral direction, with belly bulge and pointed tip.

Distinct from [Buddha's-hand tuft](../buddha_hand/README.md), which uses a 4-corner diamond cross-section per ring (palm-hand cluster).

## Construction

[`construction.rs`](construction.rs) — `SpearElement`, `SpearCluster` (two vertices per ring). Width from shared [`BellyTipProfile`](../profile.rs).

## Playground

```bash
/render spear-tuft
/render spear-tuft --bend-segments 1 --noise-frequency 0.5
/render spear-tuft --bend-segments 4 --noise-amplitude 0.12
```

## See also

- [Buddha's-hand tuft](../buddha_hand/README.md) — widening diamond fingers
- [Blade tuft](../blade/README.md) — uniform-width ribbon prisms
