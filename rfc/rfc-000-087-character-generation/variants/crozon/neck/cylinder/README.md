# Cylinder — Crozon neck

**Canonical primitive:** [Cylinder (common)](../../common/cylinder/README.md)

## Neck embedding

The neck is a short column between **head** and **torso** anchors. A **straight** cylinder (`lateral_wall_bow = 0`) gives a neutral tube—good for stylized or armored necks. **Negative** `lateral_wall_bow` (**pinch**) emphasizes a slender waist of the neck (swan-like or reptilian narrow mid-neck). **Positive** bow (**flare**) reads as muscle, hood base, or a **flared** transition from torso to head (wider mid-neck or shoulder-adjacent flare).

Place **top** / **bottom** consistently with the rest of Crozon (e.g. **top** toward head attachment, **bottom** toward torso/clavicle region—mirror the species’ head-shape README if it fixes an axis). Keep the **bow plane** aligned with the character’s **frontal** or **sagittal** read so silhouette changes read as left–right or front–back intent, not accidental twist.

For rigging, parent the neck primitive between **head** and **torso** sockets; a pinched mesh may need **collision** or **skin** envelopes adjusted, so the narrowest ring does not intersect the jaw or chest at extreme poses.
