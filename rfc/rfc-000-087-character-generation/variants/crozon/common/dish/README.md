# Dish

**Figure (TBD):** add a reference image at `./assets/dish.jpeg` when you have a sketch.

A **concave cap** with a clear **opening plane** and **rim**: the mesh reads as a **dish** or **cup** (interior hollow or implied), not a solid dome on a surface. Differs from [Half-spheroid](../half-spheroid/README.md), which is primarily an **outward** bulge clipped to a plane; here the **cavity** faces the opening normal (e.g. **concha** of an ear, stylized radar dish).

**Semantics.** The **opening** lies in a plane with normal **`opening_normal`** (often **outward** from the head for an ear). **Depth** is measured **inward** from that plane along −`opening_normal`. The **rim** is the boundary loop; the interior is a **spherical cap**, **elliptical cap**, or **low-poly faceted** patch—document which.

**Head shape (inverted).** Parent the same primitive **upside down** relative to the ear case: **`opening_normal`** points **up** (world or character up), and the **cavity opens downward** toward the face / neck. The **rim** then sits at a **brow–hairline** or **helmet** ring while the **concave** interior caps the **cranium**—a stylized **skull vault** or **hard-hat** read without a full [Spheroid](../spheroid/README.md). The opening plane can be tilted slightly **nose-forward** to match sloped foreheads.

## Parameters

**API note:** **Suggestive.** Implementations include **lathe** revolve, spherical patch, or explicit quad strip around the rim.

- `rim_width`, `rim_depth`: ellipse or rectangle spans of the **opening** in the rim plane (or `rim_radius` if circular).
- `depth`: sagitta—how deep the cavity extends from the opening plane inward.
- `rim_thickness`: optional lip thickness for a **solid** rim band (multi-mesh-friendly as a second torus-like strip later).
- `eccentricity` or `rim_squash`: stretch the rim ellipse for **cat**/**elf** concha without changing depth scale.

**Optional:** facet count; asymmetric depth (deeper toward **helix** side) as a future spline field.

## Crozon feature uses

- [Ear](../../ear/dish/README.md)
- [Head shape](../../head-shape/dish/README.md) (inverted: cranial vault / helmet cap)
