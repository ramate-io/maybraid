# Crozon hand and foot variant multi-meshes

Hands and feet are **multi-mesh by default**: **palm / sole** pad + **digit** extrusions + optional **wrist/ankle** cuff. Primitives from [`../common/`](../common/README.md):

| Variant | Common definition | Hand–foot (Crozon) |
|---------|-------------------|---------------------|
| Cylinder | [`common/cylinder`](../common/cylinder/README.md) | [`hand-foot/cylinder`](./cylinder/README.md) |
| Spheroid | [`common/spheroid`](../common/spheroid/README.md) | [`hand-foot/spheroid`](./spheroid/README.md) |
| Half-spheroid | [`common/half-spheroid`](../common/half-spheroid/README.md) | [`hand-foot/half-spheroid`](./half-spheroid/README.md) |
| Square pyramid | [`common/square-pyramid`](../common/square-pyramid/README.md) | [`hand-foot/square-pyramid`](./square-pyramid/README.md) |
| Egg | [`common/egg`](../common/egg/README.md) | [`hand-foot/egg`](./egg/README.md) |

## Multi-mesh recipe (recommended)

1. **Palm / sole:** [Half-spheroid](./half-spheroid/README.md) (dome outward), full [Spheroid](./spheroid/README.md), or [Egg](./egg/README.md) (elongated pad toward digits) if the mesh is fully exposed (paw blob).
2. **Foot wedge / hoof profile (optional):** [Square pyramid](./square-pyramid/README.md), **flattened** and **side-on** for instep or blade feet (see common semantics).
3. **Digits:** one [Cylinder](./cylinder/README.md) per phalanx chain (proximal → distal), optionally scaled by seed; **3–5** instances per limb side.
4. **Attach** digit roots to **palm** socket points; keep **shared symmetry** policy (mirror vs jitter) from the species spec.

Instancing the same digit spec **N** times still counts as one **variant recipe**; document **count** and **layout** in species rules, not only per-finger readmes.
